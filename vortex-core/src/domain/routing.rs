//! Routing module for defining active traffic targets.

use crate::domain::backend::SharedBackend;
use arc_swap::ArcSwap;
use std::sync::Arc;

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

    /// Selects the best backend using Peak EWMA.
    pub fn get_best_backend(&self) -> Option<SharedBackend> {
        let guard = self.backends.load();

        // Find the backend with the minimum score
        guard
            .iter()
            .filter(|b| b.is_healthy())
            .min_by(|a, b| {
                let score_a = a.ewma.calculate_score();
                let score_b = b.ewma.calculate_score();
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Selects the best backend capable of serving a specific AI model, utilizing
    /// fallback directives if the primary model is unavailable or heavily loaded.
    pub fn get_ai_backend(
        &self,
        metadata: &crate::domain::ai_gateway::AiMetadata,
        fallback_config: Option<&crate::domain::ai_gateway::ModelFallbackConfig>,
    ) -> Option<SharedBackend> {
        let guard = self.backends.load();

        // 1. Try to route to the primary requested model first
        let primary_candidates = guard
            .iter()
            .filter(|b| b.is_healthy() && b.ai_models.contains(&metadata.model));

        let best_primary = primary_candidates
            .min_by(|a, b| {
                a.ewma
                    .calculate_score()
                    .partial_cmp(&b.ewma.calculate_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();

        // 2. If no healthy primary, or if fallback logic applies (e.g., peak latency is too high)
        if best_primary.is_none() {
            if let Some(fallback) = fallback_config {
                for fallback_model in &fallback.fallback_models {
                    let best_fallback = guard
                        .iter()
                        .filter(|b| b.is_healthy() && b.ai_models.contains(fallback_model))
                        .min_by(|a, b| {
                            a.ewma
                                .calculate_score()
                                .partial_cmp(&b.ewma.calculate_score())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .cloned();

                    if best_fallback.is_some() {
                        return best_fallback;
                    }
                }
            }
        }

        best_primary
    }

    /// Retrieve a snapshot of all current backends (e.g., for the health checker).
    pub fn snapshot(&self) -> Arc<Vec<SharedBackend>> {
        self.backends.load_full()
    }
}

/// A shared reference to the lock-free routing table.
pub type SharedRoutingTable = Arc<RoutingTable>;
