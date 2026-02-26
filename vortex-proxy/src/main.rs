//! Vortex Proxy Engine
//!
//! The main Tokio async engine that manages socket binding, connection pooling, and request pipelining.

#![deny(missing_docs)]

use vortex_core;
use vortex_filters;
use vortex_admin;

mod server;
mod tls;
mod health_check;

use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use vortex_core::domain::backend::{Backend, BackendId};
use vortex_core::domain::routing::RoutingTable;

/// The primary entrypoint for the Vortex reverse proxy.
///
/// This initializes the multi-threaded Tokio runtime, loads the configuration,
/// and begins listening for incoming TCP connections.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Vortex Proxy Engine...");

    // Initialize core structural components
    vortex_core::core_init();
    vortex_filters::filters_init();
    vortex_admin::admin_init();

    println!("Tokio asynchronous runtime initialized successfully.");

    // Load TLS configuration
    let tls_config = tls::load_tls_config("certs/cert.pem", "certs/key.pem")
        .expect("Failed to load TLS configuration");
    let tls_acceptor = TlsAcceptor::from(tls_config);

    // Prepare mock backends for Phase 2 implementation
    let backends = vec![
        Arc::new(Backend::new(BackendId(1), "127.0.0.1:9090".parse().unwrap())),
        Arc::new(Backend::new(BackendId(2), "127.0.0.1:9091".parse().unwrap())),
    ];
    let routing_table = Arc::new(RoutingTable::new(backends));

    // Start background health-checker probing every 5 seconds
    health_check::prober::spawn_health_checker(routing_table.clone(), 5000);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8443));

    // Start the server with the TLS Acceptor and the routing table
    if let Err(e) = server::start_server(addr, Some(tls_acceptor), routing_table).await {
        eprintln!("Server failed: {}", e);
    }

    println!("Shutting down gracefully.");
    Ok(())
}
