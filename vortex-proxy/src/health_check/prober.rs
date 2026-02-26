//! Background prober for active TCP health checks.

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time;

use vortex_core::domain::routing::SharedRoutingTable;

/// Spawns a background Tokio task that periodically probes a list of backends
/// and updates their internal atomic health state.
pub fn spawn_health_checker(routing_table: SharedRoutingTable, interval_ms: u64) {
    let check_interval = Duration::from_millis(interval_ms);

    tokio::spawn(async move {
        let mut interval = time::interval(check_interval);

        // Prevent immediately ticking when spawned
        interval.tick().await;

        loop {
            interval.tick().await;

            let backends = routing_table.snapshot();
            for backend in backends.iter() {
                // Perform a simple and fast TCP connect to check health
                // In Phase 3, we can extend this to L7 HTTP probes or gRPC Ping checks
                let is_healthy = match time::timeout(
                    Duration::from_millis(1500),
                    TcpStream::connect(backend.addr)
                ).await {
                    Ok(Ok(_stream)) => true, // Successfully connected
                    _ => false,              // Timeout or Connection Refused
                };

                let was_healthy = backend.is_healthy();

                if is_healthy != was_healthy {
                    println!(
                        "[HEALTH-CHECK] Backend {} ({}) state changed: {} -> {}",
                        backend.id.0, backend.addr, was_healthy, is_healthy
                    );
                    backend.set_healthy(is_healthy);
                }
            }
        }
    });
}
