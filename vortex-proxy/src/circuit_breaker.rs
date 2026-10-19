//! State-machine Circuit Breaker pattern for upstream backend health management.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Circuit states in the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation: all traffic is routed to the upstream host.
    Closed,
    /// Upstream failure detected: requests are failed fast without hitting the backend.
    Open,
    /// Cooldown expired: a canary probe quota is permitted to test upstream recovery.
    HalfOpen,
}

/// Error returned when a request is rejected by the circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    /// Circuit is open and rejecting requests until cooldown expires.
    CircuitOpen {
        /// Time remaining before cooldown elapses and half-open probes are admitted.
        remaining_cooldown: Duration,
    },
    /// Circuit is in half-open state and has exhausted its canary probe quota.
    HalfOpenQuotaExceeded,
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitOpen { remaining_cooldown } => {
                write!(
                    f,
                    "circuit breaker is OPEN: rejecting request ({:?} remaining)",
                    remaining_cooldown
                )
            }
            Self::HalfOpenQuotaExceeded => {
                write!(
                    f,
                    "circuit breaker is HALF_OPEN: canary probe quota exhausted"
                )
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

/// Configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Consecutive failure count that trips a Closed circuit to Open.
    pub consecutive_failure_threshold: usize,
    /// Ratio of failures (0.0 to 1.0) over the sliding window that trips the circuit to Open.
    pub failure_rate_threshold: f64,
    /// Minimum requests recorded in sliding window before failure rate triggers are evaluated.
    pub minimum_window_requests: usize,
    /// Duration of the sliding time window for tracking failure rates.
    pub window_duration: Duration,
    /// Base duration to remain in Open state before transitioning to HalfOpen.
    pub cooldown_duration: Duration,
    /// Number of successful canary probe requests required in HalfOpen to restore Closed state.
    pub half_open_probe_quota: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            consecutive_failure_threshold: 5,
            failure_rate_threshold: 0.5,
            minimum_window_requests: 10,
            window_duration: Duration::from_secs(10),
            cooldown_duration: Duration::from_secs(15),
            half_open_probe_quota: 3,
        }
    }
}

#[derive(Debug)]
struct WindowSample {
    timestamp: Instant,
    is_success: bool,
}

#[derive(Debug)]
struct CircuitInner {
    state: CircuitState,
    opened_at: Option<Instant>,
    consecutive_failures: usize,
    half_open_probes_in_flight: usize,
    half_open_successes: usize,
    samples: VecDeque<WindowSample>,
}

/// Thread-safe circuit breaker implementation.
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    inner: RwLock<CircuitInner>,
    total_rejected: AtomicUsize,
    total_trips: AtomicUsize,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with the given name and default configuration.
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_config(name, CircuitBreakerConfig::default())
    }

    /// Creates a new circuit breaker with custom configuration.
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            config,
            inner: RwLock::new(CircuitInner {
                state: CircuitState::Closed,
                opened_at: None,
                consecutive_failures: 0,
                half_open_probes_in_flight: 0,
                half_open_successes: 0,
                samples: VecDeque::new(),
            }),
            total_rejected: AtomicUsize::new(0),
            total_trips: AtomicUsize::new(0),
        }
    }

    /// Returns the name/identifier of this circuit breaker.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Returns the total number of rejected requests.
    pub fn total_rejected(&self) -> usize {
        self.total_rejected.load(Ordering::Relaxed)
    }

    /// Returns the total number of times the circuit tripped from Closed to Open.
    pub fn total_trips(&self) -> usize {
        self.total_trips.load(Ordering::Relaxed)
    }

    /// Evaluates current circuit state, handling automatic transitions from Open to HalfOpen.
    pub fn current_state(&self) -> CircuitState {
        let mut inner = self.inner.write().unwrap();
        self.refresh_state(&mut inner);
        inner.state
    }

    /// Checks if a request is permitted to proceed.
    /// Returns Ok(()) if allowed, or Err(CircuitBreakerError) if blocked.
    pub fn allow_request(&self) -> Result<(), CircuitBreakerError> {
        let mut inner = self.inner.write().unwrap();
        self.refresh_state(&mut inner);

        match inner.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                let remaining = match inner.opened_at {
                    Some(opened) => self
                        .config
                        .cooldown_duration
                        .saturating_sub(opened.elapsed()),
                    None => self.config.cooldown_duration,
                };
                Err(CircuitBreakerError::CircuitOpen {
                    remaining_cooldown: remaining,
                })
            }
            CircuitState::HalfOpen => {
                if inner.half_open_probes_in_flight < self.config.half_open_probe_quota {
                    inner.half_open_probes_in_flight += 1;
                    Ok(())
                } else {
                    self.total_rejected.fetch_add(1, Ordering::Relaxed);
                    Err(CircuitBreakerError::HalfOpenQuotaExceeded)
                }
            }
        }
    }

    /// Records a successful request execution.
    pub fn record_success(&self) {
        let mut inner = self.inner.write().unwrap();
        self.refresh_state(&mut inner);

        match inner.state {
            CircuitState::Closed => {
                inner.consecutive_failures = 0;
                self.add_sample(&mut inner, true);
            }
            CircuitState::HalfOpen => {
                inner.half_open_successes += 1;
                if inner.half_open_probes_in_flight > 0 {
                    inner.half_open_probes_in_flight -= 1;
                }
                // If we met the probe quota, restore to Closed
                if inner.half_open_successes >= self.config.half_open_probe_quota {
                    inner.state = CircuitState::Closed;
                    inner.opened_at = None;
                    inner.consecutive_failures = 0;
                    inner.half_open_probes_in_flight = 0;
                    inner.half_open_successes = 0;
                    inner.samples.clear();
                }
            }
            CircuitState::Open => {
                // Ignore unexpected success while Open
            }
        }
    }

    /// Records a failed request execution.
    pub fn record_failure(&self) {
        let mut inner = self.inner.write().unwrap();
        self.refresh_state(&mut inner);

        match inner.state {
            CircuitState::Closed => {
                inner.consecutive_failures += 1;
                self.add_sample(&mut inner, false);

                // Check consecutive failure threshold
                let trip_consecutive =
                    inner.consecutive_failures >= self.config.consecutive_failure_threshold;

                // Check sliding window failure rate threshold
                let trip_rate = self.evaluate_failure_rate(&inner);

                if trip_consecutive || trip_rate {
                    self.trip_to_open(&mut inner);
                }
            }
            CircuitState::HalfOpen => {
                // Any failure during HalfOpen immediately trips back to Open
                if inner.half_open_probes_in_flight > 0 {
                    inner.half_open_probes_in_flight -= 1;
                }
                self.trip_to_open(&mut inner);
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }

    /// Manually reset circuit breaker back to Closed state.
    pub fn reset(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.state = CircuitState::Closed;
        inner.opened_at = None;
        inner.consecutive_failures = 0;
        inner.half_open_probes_in_flight = 0;
        inner.half_open_successes = 0;
        inner.samples.clear();
    }

    // Helper functions

    fn refresh_state(&self, inner: &mut CircuitInner) {
        if inner.state == CircuitState::Open {
            if let Some(opened) = inner.opened_at {
                if opened.elapsed() >= self.config.cooldown_duration {
                    inner.state = CircuitState::HalfOpen;
                    inner.half_open_probes_in_flight = 0;
                    inner.half_open_successes = 0;
                }
            }
        }
    }

    fn trip_to_open(&self, inner: &mut CircuitInner) {
        inner.state = CircuitState::Open;
        inner.opened_at = Some(Instant::now());
        inner.half_open_probes_in_flight = 0;
        inner.half_open_successes = 0;
        inner.samples.clear();
        self.total_trips.fetch_add(1, Ordering::Relaxed);
    }

    fn add_sample(&self, inner: &mut CircuitInner, is_success: bool) {
        let now = Instant::now();
        inner.samples.push_back(WindowSample {
            timestamp: now,
            is_success,
        });
        self.prune_samples(inner, now);
    }

    fn prune_samples(&self, inner: &mut CircuitInner, now: Instant) {
        while let Some(front) = inner.samples.front() {
            if now.duration_since(front.timestamp) > self.config.window_duration {
                inner.samples.pop_front();
            } else {
                break;
            }
        }
    }

    fn evaluate_failure_rate(&self, inner: &CircuitInner) -> bool {
        if inner.samples.len() < self.config.minimum_window_requests {
            return false;
        }

        let failures = inner.samples.iter().filter(|s| !s.is_success).count();
        let rate = failures as f64 / inner.samples.len() as f64;
        rate >= self.config.failure_rate_threshold
    }
}

/// Thread-safe registry mapping upstream host targets to their individual circuit breakers.
#[derive(Debug, Default)]
pub struct CircuitBreakerRegistry {
    breakers: dashmap::DashMap<String, Arc<CircuitBreaker>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// Creates a new empty registry with default circuit breaker configuration.
    pub fn new() -> Self {
        Self {
            breakers: dashmap::DashMap::new(),
            default_config: CircuitBreakerConfig::default(),
        }
    }

    /// Creates a registry with customized default configuration.
    pub fn with_default_config(config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: dashmap::DashMap::new(),
            default_config: config,
        }
    }

    /// Retrieves an existing circuit breaker or lazily creates a new one for the upstream key.
    pub fn get_or_create(&self, upstream_id: &str) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(upstream_id.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::with_config(
                    upstream_id,
                    self.default_config.clone(),
                ))
            })
            .clone()
    }

    /// Checks if a circuit breaker is currently registered for the given upstream ID.
    pub fn contains(&self, upstream_id: &str) -> bool {
        self.breakers.contains_key(upstream_id)
    }

    /// Clears all registered circuit breakers.
    pub fn clear(&self) {
        self.breakers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_closed() {
        let cb = CircuitBreaker::new("upstream-api");
        assert_eq!(cb.current_state(), CircuitState::Closed);
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn test_consecutive_failure_threshold_trips_to_open() {
        let config = CircuitBreakerConfig {
            consecutive_failure_threshold: 3,
            cooldown_duration: Duration::from_millis(50),
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("auth-service", config);

        assert_eq!(cb.current_state(), CircuitState::Closed);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.current_state(), CircuitState::Closed);

        // Third failure trips to Open
        cb.record_failure();
        assert_eq!(cb.current_state(), CircuitState::Open);
        assert!(cb.allow_request().is_err());
        assert_eq!(cb.total_trips(), 1);
    }

    #[test]
    fn test_cooldown_transition_to_half_open_and_recovery() {
        let config = CircuitBreakerConfig {
            consecutive_failure_threshold: 2,
            cooldown_duration: Duration::from_millis(30),
            half_open_probe_quota: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("payment-svc", config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.current_state(), CircuitState::Open);

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(40));

        // State transitions to HalfOpen
        assert_eq!(cb.current_state(), CircuitState::HalfOpen);

        // Allow canary probe 1
        assert!(cb.allow_request().is_ok());
        cb.record_success();
        assert_eq!(cb.current_state(), CircuitState::HalfOpen);

        // Allow canary probe 2
        assert!(cb.allow_request().is_ok());
        cb.record_success();

        // After meeting quota, transitions back to Closed
        assert_eq!(cb.current_state(), CircuitState::Closed);
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn test_half_open_failure_immediately_reverts_to_open() {
        let config = CircuitBreakerConfig {
            consecutive_failure_threshold: 1,
            cooldown_duration: Duration::from_millis(20),
            half_open_probe_quota: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("search-svc", config);

        cb.record_failure();
        assert_eq!(cb.current_state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(cb.current_state(), CircuitState::HalfOpen);

        // Failed probe immediately trips to Open
        cb.record_failure();
        assert_eq!(cb.current_state(), CircuitState::Open);
        assert!(cb.allow_request().is_err());
    }

    #[test]
    fn test_failure_rate_threshold() {
        let config = CircuitBreakerConfig {
            consecutive_failure_threshold: 10, // high so rate trips first
            failure_rate_threshold: 0.5,
            minimum_window_requests: 4,
            window_duration: Duration::from_secs(5),
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("rate-svc", config);

        cb.record_success();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.current_state(), CircuitState::Closed);

        // 2 failures out of 4 requests = 50%
        cb.record_failure();
        assert_eq!(cb.current_state(), CircuitState::Open);
    }

    #[test]
    fn test_registry_get_or_create() {
        let registry = CircuitBreakerRegistry::new();
        let cb1 = registry.get_or_create("host1.cluster.local");
        let cb2 = registry.get_or_create("host1.cluster.local");

        assert_eq!(cb1.name(), "host1.cluster.local");
        assert!(Arc::ptr_eq(&cb1, &cb2));
        assert!(registry.contains("host1.cluster.local"));
    }
}
