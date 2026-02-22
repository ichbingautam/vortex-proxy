//! Vortex Proxy Engine
//!
//! The main Tokio async engine that manages socket binding, connection pooling, and request pipelining.

#![deny(missing_docs)]

use vortex_core;
use vortex_filters;
use vortex_admin;

mod server;

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

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));

    // Start the server (this will block until failure or shutdown)
    if let Err(e) = server::start_server(addr).await {
        eprintln!("Server failed: {}", e);
    }

    println!("Shutting down gracefully.");
    Ok(())
}
