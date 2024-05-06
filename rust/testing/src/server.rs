//! Testing utilities for servers.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

use crate::latch::Latch;

/// A reference to a server running in the background, which will shut
/// it down on drop.
pub struct BackgroundServer {
    address: SocketAddr,
    shutdown: Latch,
}

impl BackgroundServer {
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    pub fn url(&self) -> String {
        format!("http://[{}]:{}", self.address.ip(), self.address.port())
    }
}

impl Drop for BackgroundServer {
    fn drop(&mut self) {
        self.shutdown.unlock();
    }
}

/// The various pieces required to build a [`BackgroundServer`].
#[async_trait::async_trait]
pub trait BackgroundServerBuilder {
    type Server: Send + 'static;

    /// Create a new server object listening on the returned address, but does
    /// not actually start the server.
    async fn create_server(&self) -> anyhow::Result<(Self::Server, SocketAddr)>;

    /// Starts the server, giving it a shutdown signal.
    ///
    /// The server must await the signal and shut down once it has been
    /// received.
    async fn start_server(
        &self,
        server: Self::Server,
        shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send + Sync>>,
    ) -> ();
}

/// Starts a new server in the background. Returns an object which can be
/// queried for the server address.
///
/// Shuts down the server on drop.
pub async fn serve_in_background(
    builder: impl BackgroundServerBuilder + Send + 'static,
) -> anyhow::Result<BackgroundServer> {
    let (server, address) = builder.create_server().await?;
    let shutdown = Latch::new();
    let shutdown_inner = shutdown.clone();
    tokio::task::spawn(async move { builder.start_server(server, Box::pin(shutdown_inner)).await });
    Ok(BackgroundServer { address, shutdown })
}
