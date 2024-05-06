//! A simple web server that echoes a POST body back.
//!
//! It publishes traces to a tracing server.

use std::env;
use std::net;

use ddn_tracing::tracing;
use test_servers::termination::wait_for_termination;

const DEFAULT_PORT: u16 = 9001;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let host = net::IpAddr::V6(net::Ipv6Addr::LOCALHOST);
    let port = env::var("PORT")
        .map(|s| s.parse())
        .unwrap_or(Ok(DEFAULT_PORT))?;
    let address = net::SocketAddr::new(host, port);

    let service_name = env!("CARGO_BIN_NAME");
    let service_version = env!("CARGO_PKG_VERSION");
    let _global_tracing = ddn_tracing::setup::init_tracing(None, service_name, service_version)
        .map_err(|e| anyhow::anyhow!(e))?;

    let app = axum::Router::new()
        .route(
            "/echo",
            axum::routing::post(|body: String| async {
                tracing::info!(path = "/echo", body);
                body
            }),
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
