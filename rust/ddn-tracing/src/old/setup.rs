use std::error::Error;
use std::mem::ManuallyDrop;

use super::global_tracer;
use super::tracer::Tracer;

pub fn start_tracer(
    endpoint: Option<&str>,
    service_name: &'static str,
    service_version: &'static str,
) -> Result<Tracer, Box<dyn Error + Send + Sync>> {
    // Do not drop the global tracing provider immediately.
    // This is handled by `shutdown_tracer` instead.
    let _ = ManuallyDrop::new(crate::setup::init_tracing(
        endpoint,
        service_name,
        service_version,
    )?);
    Ok(global_tracer())
}

pub fn shutdown_tracer() {
    opentelemetry::global::shutdown_tracer_provider();
}
