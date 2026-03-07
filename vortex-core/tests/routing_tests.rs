//! Integration tests for routing table configuration swaps.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use vortex_core::domain::backend::{Backend, BackendId};
use vortex_core::domain::routing::RoutingTable;

#[tokio::test]
async fn test_zero_downtime_config_swap_draining() {
    let initial_backends = vec![Arc::new(Backend::new(BackendId(1), "127.0.0.1:8080".parse().unwrap()))];
    let routing_table = Arc::new(RoutingTable::new(initial_backends));

    // 1. A request comes in and grabs a reference to the active backend via arc-swap.
    let active_backend_for_req1 = routing_table.get_healthy_backend().expect("Expected healthy backend");
    assert_eq!(active_backend_for_req1.id.0, 1);

    // 2. An admin hot-reloads the configuration to a completely new topology.
    // Notice this does NOT require a lock.
    let new_backends = vec![Arc::new(Backend::new(BackendId(2), "127.0.0.1:9090".parse().unwrap()))];
    routing_table.update_backends(new_backends);

    // 3. The `active_backend_for_req1` instance is still perfectly valid in memory
    // because it holds an Arc increment from the ArcSwap guard before the swap.
    // This allows "req1" to finish processing and drain gracefully with zero downtime.
    sleep(Duration::from_millis(50)).await;
    assert_eq!(active_backend_for_req1.id.0, 1);

    // 4. Any completely new requests moving forward must instantly grab the updated topology.
    let active_backend_for_req2 = routing_table.get_healthy_backend().expect("Expected healthy backend");
    assert_eq!(active_backend_for_req2.id.0, 2);
}
