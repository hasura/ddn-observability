//! Sets up tracing globally.

use std::error::Error;

use opentelemetry::propagation::composite::TextMapCompositePropagator;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_semantic_conventions as semcov;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const DEFAULT_LEVEL: tracing::level_filters::LevelFilter =
    tracing::level_filters::LevelFilter::INFO;

pub struct GlobalTracing;

/// Initialize a generic tracing setup that exports traces, and install it as
/// the global tracing provider.
///
/// The tracing provider will be unregistered on drop.
///
/// All configuration is done by standard environment variables:
///
///   * https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/
///   * https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/
pub fn init_tracing(
    endpoint: Option<&str>,
    service_name: &'static str,
    service_version: &'static str,
) -> Result<GlobalTracing, Box<dyn Error + Send + Sync>> {
    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(opentelemetry_zipkin::Propagator::new()),
    ]));

    let mut exporter = opentelemetry_otlp::new_exporter().tonic();
    exporter = if let Some(endpoint) = endpoint {
        exporter.with_endpoint(endpoint)
    } else {
        exporter
    };

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
            opentelemetry_sdk::Resource::new(vec![
                KeyValue::new(semcov::resource::SERVICE_NAME, service_name),
                KeyValue::new(semcov::resource::SERVICE_VERSION, service_version),
            ]),
        ))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    tracing_subscriber::registry()
        .with(
            tracing_opentelemetry::layer()
                .with_error_records_to_exceptions(true)
                .with_tracer(tracer),
        )
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(DEFAULT_LEVEL.into())
                .from_env_lossy(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_timer(tracing_subscriber::fmt::time::time()),
        )
        .init();

    Ok(GlobalTracing)
}

impl Drop for GlobalTracing {
    fn drop(&mut self) {
        global::shutdown_tracer_provider();
    }
}
