//! Domain models and traits for Rate Limiting (GCRA).

use std::time::Duration;
use async_trait::async_trait;

/// The result of a rate limit check.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitResult {
    /// Whether the request is allowed to pass.
    pub allowed: bool,
    /// The number of tokens/requests remaining in the current window.
    pub remaining: u64,
    /// The time until the rate limit fully resets.
    pub reset_after: Duration,
}

/// A generic store for distributed rate limiting state.
/// This allows the core proxy to decouple from Redis or other backends.
#[async_trait]
pub trait RateStore: Send + Sync {
    /// Checks and updates the rate limit for a given key.
    ///
    /// # Arguments
    /// * `key` - The unique identifier for the limit (e.g., "ip:192.168.1.1").
    /// * `limit` - The maximum burst size (capacity).
    /// * `period` - The time period over which the limit applies (refill rate).
    async fn check_rate_limit(
        &self,
        key: &str,
        limit: u64,
        period: Duration,
    ) -> Result<RateLimitResult, Box<dyn std::error::Error + Send + Sync>>;
}
