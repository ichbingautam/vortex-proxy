//! Distributed GCRA (Generic Cell Rate Algorithm) implementation using Redis.
//!
//! Evaluates rate limits atomically using a Lua script to eliminate race conditions
//! across a distributed proxy fleet.

use std::time::{SystemTime, UNIX_EPOCH, Duration};
use async_trait::async_trait;
use deadpool_redis::{Pool, Config, Runtime};
use redis::Script;
use vortex_core::domain::rate_limit::{RateStore, RateLimitResult};

/// A Redis-backed rate store implementing the GCRA algorithm.
pub struct RedisStore {
    pool: Pool,
    script: Script,
}

impl RedisStore {
    /// Initialize a new RedisStore connection pool and pre-compile the GCRA Lua script.
    pub fn new(redis_url: &str) -> Self {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

        // This GCRA implementation calculates the Theoretical Arrival Time (TAT).
        let script = Script::new(r#"
            local key = KEYS[1]
            local limit = tonumber(ARGV[1])
            local period_ms = tonumber(ARGV[2])
            local now_ms = tonumber(ARGV[3])

            local emission_interval = period_ms / limit
            local burst_offset = period_ms

            local tat = redis.call("GET", key)
            if not tat then
                tat = now_ms
            else
                tat = tonumber(tat)
            end

            tat = math.max(tat, now_ms)
            local new_tat = tat + emission_interval
            local allow_at = new_tat - burst_offset

            if allow_at > now_ms then
                -- Rate Limit Exceeded: return {allowed=0, remaining, reset_after_ms}
                local remaining = math.floor((period_ms - (tat - now_ms)) / emission_interval)
                if remaining < 0 then remaining = 0 end
                return {0, remaining, tat - now_ms}
            else
                -- Allowed: update TAT and return new state
                redis.call("SET", key, new_tat, "PX", math.ceil(new_tat - now_ms))
                local remaining = math.floor((period_ms - (new_tat - now_ms)) / emission_interval)
                if remaining < 0 then remaining = 0 end
                return {1, remaining, new_tat - now_ms}
            end
        "#);

        Self { pool, script }
    }
}

#[async_trait]
impl RateStore for RedisStore {
    async fn check_rate_limit(
        &self,
        key: &str,
        limit: u64,
        period: Duration,
    ) -> Result<RateLimitResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let period_ms = period.as_millis() as u64;

        let result: Vec<i64> = self.script
            .key(key)
            .arg(limit)
            .arg(period_ms)
            .arg(now)
            .invoke_async(&mut conn).await?;

        let allowed = result[0] == 1;
        let remaining = result[1] as u64;
        let reset_after = Duration::from_millis(result[2].max(0) as u64);

        Ok(RateLimitResult {
            allowed,
            remaining,
            reset_after,
        })
    }
}
