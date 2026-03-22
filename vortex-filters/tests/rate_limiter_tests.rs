//! Integration tests for the Distributed GCRA Rate Limiter.

use std::time::Duration;
use vortex_core::domain::rate_limit::RateStore;
use vortex_filters::rate_limiter::redis_store::RedisStore;

#[tokio::test]
#[ignore = "Requires a local Redis instance on port 6379"]
async fn test_gcra_rate_limiting() {
    let store = RedisStore::new("redis://127.0.0.1/");
    let key = "test_ip:192.168.1.100";

    // Limit to 5 requests per second
    let limit = 5;
    let period = Duration::from_secs(1);

    // First request should pass
    let res = store.check_rate_limit(key, limit, period).await.unwrap();
    assert!(res.allowed);
    assert_eq!(res.remaining, 4);

    // Exhaust the limit
    for i in 0..4 {
        let res = store.check_rate_limit(key, limit, period).await.unwrap();
        assert!(res.allowed);
        assert_eq!(res.remaining, 3 - i as u64);
    }

    // 6th request should fail
    let res = store.check_rate_limit(key, limit, period).await.unwrap();
    assert!(!res.allowed);
    assert_eq!(res.remaining, 0);
    assert!(res.reset_after.as_millis() > 0);
}
