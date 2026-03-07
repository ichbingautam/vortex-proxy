//! Server implementation for the Vortex Admin API.

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

use crate::proto::admin_service_server::{AdminService, AdminServiceServer};
use crate::proto::{GetStatsRequest, GetStatsResponse, ReloadConfigRequest, ReloadConfigResponse};

/// Implementation of the AdminService gRPC server.
#[derive(Default)]
pub struct AdminServerImpl {}

#[tonic::async_trait]
impl AdminService for AdminServerImpl {
    async fn reload_config(
        &self,
        request: Request<ReloadConfigRequest>,
    ) -> Result<Response<ReloadConfigResponse>, Status> {
        let req = request.into_inner();
        println!("Received reload config request for path: {}", req.config_path);

        // TODO: Actually trigger the arc-swap zero-downtime swap here

        Ok(Response::new(ReloadConfigResponse {
            success: true,
            message: format!("Ack: Will reload config from {}", req.config_path),
        }))
    }

    async fn get_stats(
        &self,
        _request: Request<GetStatsRequest>,
    ) -> Result<Response<GetStatsResponse>, Status> {
        // TODO: Wire up actual telemetry here
        Ok(Response::new(GetStatsResponse {
            active_connections: 0,
        }))
    }
}

/// Start the Admin gRPC server listening on a Unix Domain Socket.
pub async fn start_admin_server(socket_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Ensure any dangling socket from a previous process is cleaned up
    let _ = std::fs::remove_file(socket_path);

    let uds = UnixListener::bind(socket_path)?;
    let stream = UnixListenerStream::new(uds);

    let admin_service = AdminServerImpl::default();

    println!("Starting Admin Unix Socket API at {}", socket_path);

    tonic::transport::Server::builder()
        .add_service(AdminServiceServer::new(admin_service))
        .serve_with_incoming(stream)
        .await?;

    Ok(())
}
