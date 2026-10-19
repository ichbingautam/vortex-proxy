#![allow(missing_docs)]

use std::sync::Arc;
use std::time::Duration;
use vortex_proxy::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitState,
};

#[tokio::test]
async fn test_circuit_breaker_concurrent_requests_and_tripping() {
    let config = CircuitBreakerConfig {
        consecutive_failure_threshold: 10,
        cooldown_duration: Duration::from_millis(50),
        half_open_probe_quota: 3,
        ..Default::default()
    };
    let cb = Arc::new(CircuitBreaker::with_config("payment-backend", config));

    // Initially closed and permits requests
    assert_eq!(cb.current_state(), CircuitState::Closed);
    assert!(cb.allow_request().is_ok());

    // Concurrently record failures to trip the circuit
    let mut handles = Vec::new();
    for _ in 0..10 {
        let cb_clone = Arc::clone(&cb);
        handles.push(tokio::spawn(async move {
            cb_clone.record_failure();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Circuit should now be open
    assert_eq!(cb.current_state(), CircuitState::Open);
    assert!(cb.allow_request().is_err());
    assert_eq!(cb.total_trips(), 1);

    // Sleep to allow cooldown to elapse
    tokio::time::sleep(Duration::from_millis(60)).await;

    // State transitions to HalfOpen
    assert_eq!(cb.current_state(), CircuitState::HalfOpen);

    // Canary probe requests
    for _ in 0..3 {
        assert!(cb.allow_request().is_ok());
        cb.record_success();
    }

    // Circuit successfully recovers back to Closed
    assert_eq!(cb.current_state(), CircuitState::Closed);
    assert!(cb.allow_request().is_ok());
}

#[tokio::test]
async fn test_circuit_breaker_registry_multi_tenant() {
    let registry = Arc::new(CircuitBreakerRegistry::new());

    let cb1 = registry.get_or_create("cluster-a.internal");
    let cb2 = registry.get_or_create("cluster-b.internal");

    // Trip cluster-a
    for _ in 0..5 {
        cb1.record_failure();
    }
    assert_eq!(cb1.current_state(), CircuitState::Open);

    // cluster-b should remain unaffected and closed
    assert_eq!(cb2.current_state(), CircuitState::Closed);
    assert!(cb2.allow_request().is_ok());
}
