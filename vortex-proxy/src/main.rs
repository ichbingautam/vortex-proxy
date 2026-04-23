//! Vortex Proxy Engine
//!
//! The main Tokio async engine that manages socket binding, connection pooling, and request pipelining.

#![deny(missing_docs)]

mod connection_pool;
mod health_check;
pub mod metrics_ext;
mod quic_server;
mod server;
pub mod telemetry;
mod tls;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use crate::connection_pool::pool::ConnectionPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use vortex_core::domain::backend::{Backend, BackendId};
use vortex_core::domain::routing::RoutingTable;
use vortex_filters::wasm_engine::WasmEngine;

/// The primary entrypoint for the Vortex reverse proxy.
///
/// This initializes the multi-threaded Tokio runtime, loads the configuration,
/// and begins listening for incoming TCP connections.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core_ids = core_affinity::get_core_ids().unwrap_or_default();
    let core_idx = Arc::new(AtomicUsize::new(0));
    let num_cores = core_ids.len();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(if num_cores > 0 { num_cores } else { 4 })
        .on_thread_start(move || {
            if num_cores > 0 {
                let idx = core_idx.fetch_add(1, Ordering::SeqCst);
                let core = core_ids[idx % num_cores];
                core_affinity::set_for_current(core);
            }
        })
        .build()?;

    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize non-blocking OpenTelemetry tracing
    telemetry::init_telemetry().unwrap();

    // Initialize Prometheus metrics exporter on port 9091
    metrics_ext::init_metrics_exporter(9091).unwrap();

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

    #[cfg(target_os = "linux")]
    let _xdp_limiter = {
        let bpf_path = "vortex-ebpf/bpf/xdp_drop.o";
        if let Ok(bpf_code) = std::fs::read(bpf_path) {
            match vortex_ebpf::linux::LinuxXdpLimiter::new("eth0", &bpf_code) {
                Ok(limiter) => {
                    tracing::info!("Successfully loaded eBPF XDP rate limiter on eth0");
                    Some(Arc::new(limiter))
                }
                Err(e) => {
                    tracing::warn!("Failed to load eBPF XDP rate limiter: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!(
                "Could not find eBPF object file at {}. Skipping eBPF initialization.",
                bpf_path
            );
            None
        }
    };

    // Prepare mock backends for Phase 2 implementation
    let backends = vec![
        Arc::new(Backend::new(
            BackendId(1),
            "127.0.0.1:9090".parse().unwrap(),
        )),
        Arc::new(Backend::new(
            BackendId(2),
            "127.0.0.1:9091".parse().unwrap(),
        )),
    ];
    let routing_table = Arc::new(RoutingTable::new(backends));

    // Start background health-checker probing every 5 seconds
    health_check::prober::spawn_health_checker(routing_table.clone(), 5000);

    let active_connections = Arc::new(AtomicUsize::new(0));

    // Spawn the Control Plane API on a Unix Domain Socket
    let admin_routing_table = routing_table.clone();
    let admin_active_connections = active_connections.clone();
    tokio::spawn(async move {
        if let Err(e) = vortex_admin::server::start_admin_server(
            "/tmp/vortex_admin.sock",
            admin_routing_table,
            admin_active_connections,
        )
        .await
        {
            eprintln!("Admin gRPC server failed: {}", e);
        }
    });

    // Spawn Kubernetes CRD Watcher
    let k8s_routing_table = routing_table.clone();
    tokio::spawn(async move {
        if let Err(e) = vortex_admin::k8s_watcher::start_k8s_watcher(k8s_routing_table).await {
            tracing::warn!(
                "K8s watcher not running (e.g., outside cluster) or failed: {}",
                e
            );
        }
    });

    let connection_pool = ConnectionPool::new();
    let wasm_engine = Arc::new(WasmEngine::new());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8443));

    // Spawn the QUIC / HTTP/3 server concurrently on the same port (UDP)
    let quic_routing_table = routing_table.clone();
    let quic_addr = addr;
    tokio::spawn(async move {
        if let Err(e) = quic_server::start_quic_server(
            quic_addr,
            "certs/cert.pem",
            "certs/key.pem",
            quic_routing_table,
        )
        .await
        {
            eprintln!("QUIC Server failed: {}", e);
        }
    });

    // Start the server with the TLS Acceptor, routing table, hot pool, and Wasm runtime
    if let Err(e) = server::start_server(
        addr,
        Some(tls_acceptor),
        routing_table,
        connection_pool,
        wasm_engine,
        active_connections,
    )
    .await
    {
        eprintln!("Server failed: {}", e);
    }

    println!("Shutting down gracefully.");
    Ok(())
}
