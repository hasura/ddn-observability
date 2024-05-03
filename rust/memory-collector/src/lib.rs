use std::net;

use opentelemetry_proto::tonic::collector::trace::v1::*;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Default)]
pub struct TraceServiceHandler {}

#[tonic::async_trait]
impl trace_service_server::TraceService for TraceServiceHandler {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        let resource_spans = request.into_inner().resource_spans;
        eprintln!("resource_spans: {resource_spans:?}");
        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: Some(ExportTracePartialSuccess {
                rejected_spans: 0,
                error_message: String::new(),
            }),
        }))
    }
}

pub async fn serve(address: impl Into<net::SocketAddr>) -> Result<(), tonic::transport::Error> {
    let trace_service_handler = TraceServiceHandler::default();

    Server::builder()
        .add_service(trace_service_server::TraceServiceServer::new(
            trace_service_handler,
        ))
        .serve(address.into())
        .await
}
