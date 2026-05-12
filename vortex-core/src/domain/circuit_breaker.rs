//! Circuit Breaker implementation for backend resilience.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const FAILURE_THRESHOLD: usize = 5;
const RESET_TIMEOUT_SECS: u64 = 30;

/// The state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation. Requests are allowed.
    Closed,
    /// Backend is failing. Requests are blocked.
    Open,
    /// Recovery period. A test request is allowed.
    HalfOpen,
}

/// A lock-free circuit breaker state tracker.
#[derive(Debug)]
pub struct CircuitBreaker {
    failures: AtomicUsize,
    last_failure_time: AtomicU64,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    /// Creates a new circuit breaker in the Closed state.
    pub fn new() -> Self {
        Self {
            failures: AtomicUsize::new(0),
            last_failure_time: AtomicU64::new(0),
        }
    }

    /// Evaluates the current state of the circuit breaker.
    pub fn state(&self) -> CircuitState {
        let failures = self.failures.load(Ordering::Relaxed);
        if failures < FAILURE_THRESHOLD {
            return CircuitState::Closed;
        }

        let last_failure = self.last_failure_time.load(Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > last_failure + RESET_TIMEOUT_SECS {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    /// Records a successful request, resetting the circuit to Closed.
    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.last_failure_time.store(0, Ordering::Relaxed);
    }

    /// Records a failed request, potentially tripping the circuit to Open.
    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed);
        if failures + 1 >= FAILURE_THRESHOLD {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.last_failure_time.store(now, Ordering::Relaxed);
        }
    }
}
