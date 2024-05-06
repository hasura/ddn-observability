//! An in-memory tracing collector, used for testing.
//!
//! This will gather any spans sent via gRPC and store the deserialized Protocol
//! Buffers structures. They can later be read.

use std::net;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

use ddn_observability_testing::server::BackgroundServer;
use opentelemetry_proto::tonic::collector::trace::v1::*;
use tokio::net::TcpListener;
use tonic::transport::server::{Router, TcpIncoming};
use tonic::transport::Server;

use ddn_observability_testing::async_trait;
use ddn_observability_testing::server::BackgroundServerBuilder;

pub mod proto {
    pub use opentelemetry_proto::tonic::common::v1::*;
    pub use opentelemetry_proto::tonic::resource::v1::*;
    pub use opentelemetry_proto::tonic::trace::v1::*;
}

/// The collector state. Create a new one to use it.
///
/// A clone of this will share the underlying state.
#[derive(Clone)]
pub struct State {
    spans: Arc<RwLock<Vec<proto::ResourceSpans>>>,
}

impl State {
    /// Creates a new state.
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Appends a new set of spans to the list.
    fn append(&self, mut resource_spans: Vec<proto::ResourceSpans>) {
        let mut spans = self.spans.write().unwrap();
        spans.append(&mut resource_spans);
    }

    /// Gets all the spans recorded up until now.
    pub fn read(&self) -> Vec<proto::ResourceSpans> {
        let spans = self.spans.read().unwrap();
        spans.clone()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Handles traces by storing them in the in-memory state.
struct TraceServiceHandler {
    state: State,
}

#[tonic::async_trait]
impl trace_service_server::TraceService for TraceServiceHandler {
    async fn export(
        &self,
        request: tonic::Request<ExportTraceServiceRequest>,
    ) -> Result<tonic::Response<ExportTraceServiceResponse>, tonic::Status> {
        let resource_spans = request.into_inner().resource_spans;
        self.state.append(resource_spans);
        Ok(tonic::Response::new(ExportTraceServiceResponse {
            partial_success: Some(ExportTracePartialSuccess {
                rejected_spans: 0,
                error_message: String::new(),
            }),
        }))
    }
}

/// Creates a new in-memory collection server on the specified address.
///
/// Runs in the foreground.
pub async fn serve(state: &State, address: impl Into<net::SocketAddr>) -> anyhow::Result<()> {
    let router = create_router(state);
    router.serve(address.into()).await?;
    Ok(())
}

pub struct BackgroundTracingServer {
    state: State,
}

impl BackgroundTracingServer {
    pub fn new(state: &State) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

#[async_trait]
impl BackgroundServerBuilder for BackgroundTracingServer {
    type Server = (Router, TcpIncoming);

    async fn create_server(&self) -> anyhow::Result<(Self::Server, net::SocketAddr)> {
        let router = create_router(&self.state);

        let listener = TcpListener::bind((net::IpAddr::V6(net::Ipv6Addr::LOCALHOST), 0)).await?;
        let address = listener.local_addr()?;
        let incoming = TcpIncoming::from_listener(listener, false, None)
            .map_err(|error| anyhow::anyhow!(error))?;

        Ok(((router, incoming), address))
    }

    async fn start_server(
        &self,
        (router, incoming): Self::Server,
        shutdown_signal: Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync>>,
    ) -> () {
        router
            .serve_with_incoming_shutdown(incoming, shutdown_signal)
            .await
            .unwrap();
    }
}

/// Creates a new in-memory collection server on the specified TCP listener.
///
/// Runs in the background.
pub async fn serve_in_background(state: &State) -> anyhow::Result<BackgroundServer> {
    ddn_observability_testing::server::serve_in_background(BackgroundTracingServer::new(state))
        .await
}

fn create_router(state: &State) -> Router {
    let trace_service_handler = TraceServiceHandler {
        state: state.clone(),
    };
    Server::builder().add_service(trace_service_server::TraceServiceServer::new(
        trace_service_handler,
    ))
}
