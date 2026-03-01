//! Peak Estimating Exponentially Weighted Moving Average (Peak EWMA)
//!
//! Peak EWMA is an algorithm that tracks the latency of a backend.
//! It is designed to be highly sensitive to latency spikes (peaks) while
//! gracefully decaying back to the historical average over time.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// The mathematical representation of a node's latency characteristics over time.
#[derive(Debug)]
pub struct PeakEwma {
    /// The current calculated exponentially weighted moving average.
    /// Stored as bits of an f64 to allow lock-free atomic updates.
    ewma: AtomicU64,

    /// The decay rate. A higher alpha (e.g. 0.9) means older samples decay slower.
    /// A lower alpha (e.g. 0.1) means the average favors recent data heavily.
    decay_alpha: f64,

    /// The number of active, in-flight requests to this node.
    /// The `selector` multiplies the `ewma` by this value to penalize
    /// nodes with high queue depths.
    active_requests: AtomicU64,
}

impl PeakEwma {
    /// Create a new Peak EWMA tracker with a specified decay alpha.
    ///
    /// Typically, an alpha of `0.5` represents a balanced decay.
    pub fn new(initial_latency_ms: f64, decay_alpha: f64) -> Self {
        Self {
            ewma: AtomicU64::new(initial_latency_ms.to_bits()),
            decay_alpha,
            active_requests: AtomicU64::new(0),
        }
    }

    /// Read the current moving average.
    pub fn get_ewma(&self) -> f64 {
        f64::from_bits(self.ewma.load(Ordering::Relaxed))
    }

    /// Update the moving average with a newly observed latency sample.
    pub fn observe_latency(&self, rtt_ms: f64) {
        let mut current_bits = self.ewma.load(Ordering::Acquire);

        loop {
            let current_ewma = f64::from_bits(current_bits);

            // Peak EWMA Logic:
            // If the new sample is HIGHER than the historical average (a peak),
            // instantly jump the EWMA to track the peak.
            // If the new sample is LOWER (recovering), slowly decay toward it using alpha.
            let next_ewma = if rtt_ms > current_ewma {
                rtt_ms
            } else {
                (rtt_ms * (1.0 - self.decay_alpha)) + (current_ewma * self.decay_alpha)
            };

            let next_bits = next_ewma.to_bits();

            // CAS loop to ensure thread-safe lock-free updates
            match self.ewma.compare_exchange_weak(
                current_bits,
                next_bits,
                Ordering::Release,
                Ordering::Relaxed
            ) {
                Ok(_) => break, // Successfully committed the new average
                Err(updated_bits) => {
                    // Another thread updated the average under us. Retry.
                    current_bits = updated_bits;
                }
            }
        }
    }

    /// Increment the active request counter and return a guard
    /// that will decrement it when dropped.
    pub fn increment_active(&self) -> ActiveRequestGuard<'_> {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        ActiveRequestGuard { ewma: self }
    }

    /// Calculate the current "cost" (weight) of routing to this node.
    /// A lower score is better.
    ///
    /// Score = (EWMA Latency + 1) * (Active Requests + 1)
    pub fn calculate_score(&self) -> f64 {
        let ewma = self.get_ewma();
        let active = self.active_requests.load(Ordering::Relaxed) as f64;

        // Add 1 to prevent multiplying by zero
        (ewma + 1.0) * (active + 1.0)
    }
}

/// A RAII guard that automatically decrements the active request pool for a node
/// when the request finishes and drops the guard.
pub struct ActiveRequestGuard<'a> {
    ewma: &'a PeakEwma,
}

impl<'a> Drop for ActiveRequestGuard<'a> {
    fn drop(&mut self) {
        self.ewma.active_requests.fetch_sub(1, Ordering::Relaxed);
    }
}
