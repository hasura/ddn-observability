//! Functions to assist in tracing when making a HTTP request.

use tracing_opentelemetry::OpenTelemetrySpanExt;

// Extract the headers required to propagate the trace context across services
// from the current context.
// This function accepts a `Span` from the `tracing` crate and extracts the context from it
pub fn trace_headers_for_span(span: tracing::Span) -> http::HeaderMap {
    let ctx = span.context();

    let mut headers_map = http::HeaderMap::new();
    let mut header_injector = opentelemetry_http::HeaderInjector(&mut headers_map);
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&ctx, &mut header_injector);
    });
    headers_map
}
