//! Functions to assist in enabling tracing for an HTTP server.

use http::Request;
use hyper::Body;
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// A Tower layer that enables tracing and produces a root span for each
/// request.
///
/// If trace parent headers are specified in the incoming request, they will be
/// adopted and used as the span parent.
pub fn layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    MakeRequestSpan,
> {
    TraceLayer::new_for_http().make_span_with(MakeRequestSpan {})
}

/// A custom object for making spans.
#[derive(Clone)]
pub struct MakeRequestSpan;

impl MakeSpan<Body> for MakeRequestSpan {
    fn make_span(&mut self, request: &Request<Body>) -> Span {
        use opentelemetry::trace::TraceContextExt;

        // Create the root span
        let span = tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            version = ?request.version(),
        );

        // Get the parent trace ID from headers, if available.
        // This uses the OpenTelemetry `set_parent` extension rather than
        // setting a field directly on the span to ensure it works no matter
        // which propagator is configured.
        let parent_context = opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.extract(&opentelemetry_http::HeaderExtractor(request.headers()))
        });

        // If there is no parent span ID, we get something nonsensical, so we
        // need to validate it (yes, this is hilarious).
        let parent_context_span = parent_context.span();
        let parent_context_span_context = parent_context_span.span_context();
        if parent_context_span_context.is_valid() {
            span.set_parent(parent_context);
        }

        span
    }
}
