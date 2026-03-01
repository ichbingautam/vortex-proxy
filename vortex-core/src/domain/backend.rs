//! Backend server models.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::load_balancer::ewma::PeakEwma;

/// A unique identifier for a backend server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackendId(pub u32);

/// Represents a single upstream backend server
#[derive(Debug)]
pub struct Backend {
    /// The unique ID of the backend
    pub id: BackendId,
    /// The socket address of the backend
    pub addr: SocketAddr,
    /// Whether the backend is currently considered healthy
    healthy: AtomicBool,
    /// The Peak EWMA tracker for this specific backend
    pub ewma: PeakEwma,
}

impl Backend {
    /// Create a new generic backend
    pub fn new(id: BackendId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            healthy: AtomicBool::new(true), // assume healthy initially

            // Initialize EWMA with 50.0ms baseline and 0.5 balanced decay
            ewma: PeakEwma::new(50.0, 0.5),
        }
    }

    /// Check if the backend is marked healthy
    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire)
    }

    /// Update the health status of the backend
    pub fn set_healthy(&self, is_healthy: bool) {
        self.healthy.store(is_healthy, Ordering::Release);
    }
}

/// A thread-safe reference to a Backend.
pub type SharedBackend = Arc<Backend>;
