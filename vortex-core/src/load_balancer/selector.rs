//! Load Balancing Selector logic

use std::sync::Arc;
use crate::domain::backend::{Backend, SharedBackend};
use crate::domain::routing::SharedRoutingTable;
use crate::load_balancer::ewma::PeakEwma;

/// Selects the optimal backend using the Peak EWMA algorithm.
pub fn select_best_backend(routing_table: &SharedRoutingTable) -> Option<SharedBackend> {
    let backends = routing_table.snapshot();

    backends
        .iter()
        .filter(|b| b.is_healthy())
        .min_by(|a, b| {
            let score_a = a.ewma.calculate_score();
            let score_b = b.ewma.calculate_score();
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}
