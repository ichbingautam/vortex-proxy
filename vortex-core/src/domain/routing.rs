//! Routing module for defining active traffic targets.

use arc_swap::ArcSwap;
use std::sync::Arc;
use crate::domain::backend::SharedBackend;

/// A lock-free routing table mapping traffic to backends.
///
/// Uses `ArcSwap` to allow atomic, zero-downtime hot reloads of the backend
/// topology without acquiring read locks on the hot path (like `RwLock` would).
#[derive(Debug)]
pub struct RoutingTable {
    backends: ArcSwap<Vec<SharedBackend>>,
}

impl RoutingTable {
    /// Create a new routing table with the initial set of backends.
    pub fn new(initial_backends: Vec<SharedBackend>) -> Self {
        Self {
            backends: ArcSwap::from_pointee(initial_backends),
        }
    }

    /// Atomically replace the entire set of backends (e.g., during config hot-reload).
    pub fn update_backends(&self, new_backends: Vec<SharedBackend>) {
        self.backends.store(Arc::new(new_backends));
    }

    /// Selects the first available healthy backend.
    ///
    /// In the future, this will be replaced by Peak EWMA load balancing.
    pub fn get_healthy_backend(&self) -> Option<SharedBackend> {
        let guard = self.backends.load();
        guard.iter().find(|b| b.is_healthy()).cloned()
    }

    /// Retrieve a snapshot of all current backends (e.g., for the health checker).
    pub fn snapshot(&self) -> Arc<Vec<SharedBackend>> {
        self.backends.load_full()
    }
}

/// A shared reference to the lock-free routing table.
pub type SharedRoutingTable = Arc<RoutingTable>;
