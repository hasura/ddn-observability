//! A simple web server that proxies another.
//!
//! The proxied server root URL should be specified in the `TARGET_URL`
//! environment variable.
//!
//! It publishes traces to a tracing server.

use std::env;
use std::net;

use ddn_tracing::tracing;
use test_servers::termination::wait_for_termination;

const DEFAULT_PORT: u16 = 9002;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let target_uri: http::uri::Uri = env::var("TARGET_URL")?.parse()?;
    let target_uri_scheme = target_uri
        .scheme()
        .ok_or_else(|| anyhow::anyhow!("target URL has no scheme"))?
        .clone();
    let target_uri_authority = target_uri
        .authority()
        .ok_or_else(|| anyhow::anyhow!("target URL has no authority"))?
        .clone();

    let host = net::IpAddr::V6(net::Ipv6Addr::LOCALHOST);
    let port = env::var("PORT")
        .map(|s| s.parse())
        .unwrap_or(Ok(DEFAULT_PORT))?;
    let address = net::SocketAddr::new(host, port);

    let service_name = env!("CARGO_BIN_NAME");
    let service_version = env!("CARGO_PKG_VERSION");
    let _global_tracing = ddn_tracing::setup::init_tracing(None, service_name, service_version)
        .map_err(|e| anyhow::anyhow!(e))?;

    let client = reqwest::Client::new();
    let app = axum::Router::new()
        .fallback(
            move |method: http::method::Method, request_uri: http::uri::Uri, body: String| {
                async move {
                    let span = tracing::info_span!("request", uri = %request_uri, body);
                    let mut target_uri_builder = http::uri::Uri::builder()
                        .scheme(target_uri_scheme)
                        .authority(target_uri_authority);

                    target_uri_builder = match request_uri.path_and_query() {
                        None => target_uri_builder,
                        Some(path_and_query) => {
                            target_uri_builder.path_and_query(path_and_query.clone())
                        }
                    };
                    let target_uri = target_uri_builder.build().unwrap().to_string();

                    let trace_headers = ddn_tracing::http_client::trace_headers_for_span(span);

                    let response = client
                        .request(method, target_uri)
                        .headers(trace_headers)
                        .body(body)
                        .send()
                        .await
                        .unwrap();
                    response.text().await.unwrap()
                }
            },
        )
        .layer(ddn_tracing::http_server::layer());

    let server = axum::Server::bind(&address).serve(app.into_make_service());
    let address = server.local_addr();
    tracing::info!(
        server.address = %address.ip(),
        server.port = address.port(),
       "started",
    );
    server
        .with_graceful_shutdown(wait_for_termination())
        .await?;

    Ok(())
}
