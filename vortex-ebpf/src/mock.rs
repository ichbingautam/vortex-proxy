//! Mock XDP implementation for non-Linux targets (e.g. macOS).

use super::XdpRateLimiter;
use std::net::IpAddr;

/// A mock rate limiter that just logs blocked IPs instead of actually updating eBPF maps.
pub struct MockXdpLimiter;

impl MockXdpLimiter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockXdpLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl XdpRateLimiter for MockXdpLimiter {
    fn block_ip(&self, ip: IpAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Mock XDP: Blocked IP {}", ip);
        Ok(())
    }

    fn unblock_ip(&self, ip: IpAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Mock XDP: Unblocked IP {}", ip);
        Ok(())
    }
}
