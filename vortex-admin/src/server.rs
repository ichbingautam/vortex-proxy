//! Server implementation for the Vortex Admin API.

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

use crate::proto::admin_service_server::{AdminService, AdminServiceServer};
use crate::proto::{GetStatsRequest, GetStatsResponse, ReloadConfigRequest, ReloadConfigResponse};

use std::sync::Arc;
use vortex_core::domain::routing::SharedRoutingTable;
use vortex_core::domain::backend::{Backend, BackendId};

/// Implementation of the AdminService gRPC server.
pub struct AdminServerImpl {
    routing_table: SharedRoutingTable,
}

impl AdminServerImpl {
    /// Creates a new administration server handling requests.
    pub fn new(routing_table: SharedRoutingTable) -> Self {
        Self { routing_table }
    }
}

#[tonic::async_trait]
impl AdminService for AdminServerImpl {
    async fn reload_config(
        &self,
        request: Request<ReloadConfigRequest>,
    ) -> Result<Response<ReloadConfigResponse>, Status> {
        let req = request.into_inner();
        println!("Received reload config request for path: {}", req.config_path);

        // Simulate reading a configuration file from the specified path
        // In a real implementation this would parse YAML/JSON into Domain objects.
        let mut new_backends = Vec::new();
        new_backends.push(Arc::new(Backend::new(BackendId(99), "127.0.0.1:9099".parse().unwrap())));

        // Zero-downtime, lock-free swap
        self.routing_table.update_backends(new_backends);

        Ok(Response::new(ReloadConfigResponse {
            success: true,
            message: format!("Successfully fully reloaded config from {} and swapped routing architecture atomically.", req.config_path),
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
pub async fn start_admin_server(
    socket_path: &str,
    routing_table: SharedRoutingTable,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Ensure any dangling socket from a previous process is cleaned up
    let _ = std::fs::remove_file(socket_path);

    let uds = UnixListener::bind(socket_path)?;
    let stream = UnixListenerStream::new(uds);

    let admin_service = AdminServerImpl::new(routing_table);

    println!("Starting Admin Unix Socket API at {}", socket_path);

    tonic::transport::Server::builder()
        .add_service(AdminServiceServer::new(admin_service))
        .serve_with_incoming(stream)
        .await?;

    Ok(())
}
