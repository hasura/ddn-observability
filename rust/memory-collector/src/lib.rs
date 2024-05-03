use std::sync::Arc;
use std::sync::RwLock;

use tokio::net::TcpListener;
use tokio::sync::Notify;
use tonic::transport::server::{Router, TcpIncoming};
use tonic::transport::Server;

use opentelemetry_proto::tonic::collector::trace::v1::*;

pub mod proto {
    pub use opentelemetry_proto::tonic::common::v1::*;
    pub use opentelemetry_proto::tonic::resource::v1::*;
    pub use opentelemetry_proto::tonic::trace::v1::*;
}

#[derive(Clone)]
pub struct State {
    spans: Arc<RwLock<Vec<proto::ResourceSpans>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn append(&self, mut resource_spans: Vec<proto::ResourceSpans>) {
        let mut spans = self.spans.write().unwrap();
        spans.append(&mut resource_spans);
    }

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

pub struct TracingServer {
    shutdown: Arc<Notify>,
}

impl Drop for TracingServer {
    fn drop(&mut self) {
        self.shutdown.notify_one();
    }
}

pub async fn serve(state: &State, listener: TcpListener) -> anyhow::Result<()> {
    let router = create_router(state);
    let incoming = TcpIncoming::from_listener(listener, false, None)
        .map_err(|error| anyhow::anyhow!(error))?;
    router.serve_with_incoming(incoming).await?;
    Ok(())
}

pub fn serve_in_background(state: &State, listener: TcpListener) -> anyhow::Result<TracingServer> {
    let router = create_router(state);
    let incoming = TcpIncoming::from_listener(listener, false, None)
        .map_err(|error| anyhow::anyhow!(error))?;
    let shutdown = Arc::new(Notify::new());
    let shutdown_inner = Arc::clone(&shutdown);
    tokio::task::spawn(async move {
        router
            .serve_with_incoming_shutdown(incoming, shutdown_inner.notified())
            .await
            .unwrap();
    });
    Ok(TracingServer { shutdown })
}

fn create_router(state: &State) -> Router {
    let trace_service_handler = TraceServiceHandler {
        state: state.clone(),
    };
    Server::builder().add_service(trace_service_server::TraceServiceServer::new(
        trace_service_handler,
    ))
}
