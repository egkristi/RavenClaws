//! Graceful degradation under load — rate limiting, concurrency control, and load shedding.
//!
//! This module provides the infrastructure for RavenClaws to degrade gracefully
//! under load rather than failing hard. It includes:
//!
//! - **Rate limiting** — Token bucket algorithm for per-endpoint and global rate limits
//! - **Concurrency control** — Semaphore-based limit on in-flight requests
//! - **Load shedding** — Metrics-based overload detection and 503 responses
//! - **Backpressure** — Queue depth tracking and admission control
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐     ┌──────────────────┐     ┌────────────────┐
//! │  Incoming    │ ──→ │  LoadManager     │ ──→ │  Agent/Server  │
//! │  Requests    │     │  ┌────────────┐  │     │  Handler       │
//! │              │     │  │ RateLimiter│  │     │                │
//! │              │     │  ├────────────┤  │     │                │
//! │              │     │  │ Concurrency│  │     │                │
//! │              │     │  │  Limiter   │  │     │                │
//! │              │     │  ├────────────┤  │     │                │
//! │              │     │  │ LoadShedder│  │     │                │
//! └──────────────┘     └──┴────────────┴──┘     └────────────────┘
//! ```
//!
//! # Stability
//!
//! All public types are `#[non_exhaustive]` — new fields and variants may be added
//! in minor releases.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{debug, warn};

// ── Configuration ──────────────────────────────────────────────────────────

/// Load management configuration (v1.1.0)
///
/// Controls how RavenClaws handles overload conditions — rate limiting,
/// concurrency limits, and load shedding thresholds.
///
/// # Example (TOML)
///
/// ```toml
/// [load]
/// max_concurrent_requests = 50
/// rate_limit_per_second = 100
/// rate_limit_burst = 200
/// overload_error_threshold = 50
/// overload_window_secs = 60
/// shed_load_at_queue_depth = 1000
/// ```
///
/// # Stability
/// This struct is `#[non_exhaustive]` — new fields may be added in minor releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoadConfig {
    /// Maximum number of concurrent in-flight requests (0 = unlimited)
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,

    /// Rate limit: maximum requests per second (0 = unlimited)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_second: u64,

    /// Rate limit: maximum burst size (0 = use rate_limit_per_second)
    #[serde(default = "default_rate_burst")]
    pub rate_limit_burst: u64,

    /// Error rate threshold (%) for overload detection (0-100)
    /// When the error rate in the window exceeds this, the load shedder activates.
    #[serde(default = "default_error_threshold")]
    pub overload_error_threshold: u8,

    /// Time window (seconds) for overload detection
    #[serde(default = "default_window_secs")]
    pub overload_window_secs: u64,

    /// Queue depth at which to start shedding load (0 = disabled)
    #[serde(default = "default_queue_depth")]
    pub shed_load_at_queue_depth: usize,

    /// Whether to enable graceful degradation features
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: default_max_concurrent(),
            rate_limit_per_second: default_rate_limit(),
            rate_limit_burst: default_rate_burst(),
            overload_error_threshold: default_error_threshold(),
            overload_window_secs: default_window_secs(),
            shed_load_at_queue_depth: default_queue_depth(),
            enabled: default_enabled(),
        }
    }
}

fn default_max_concurrent() -> usize {
    50
}

fn default_rate_limit() -> u64 {
    100
}

fn default_rate_burst() -> u64 {
    200
}

fn default_error_threshold() -> u8 {
    50
}

fn default_window_secs() -> u64 {
    60
}

fn default_queue_depth() -> usize {
    1000
}

fn default_enabled() -> bool {
    true
}

// ── Admission decision ─────────────────────────────────────────────────────

/// Result of an admission check — whether a request is allowed through.
///
/// # Stability
/// This enum is `#[non_exhaustive]` — new variants may be added in minor releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Admission {
    /// Request is allowed to proceed
    Allowed,
    /// Request is rate-limited (too many requests per second)
    RateLimited,
    /// Request is concurrency-limited (too many in-flight requests)
    ConcurrencyLimited,
    /// Request is load-shed (system is overloaded)
    LoadShed,
}

impl Admission {
    /// Returns `true` if the request is allowed through.
    pub fn is_allowed(self) -> bool {
        matches!(self, Admission::Allowed)
    }
}

// ── Token bucket rate limiter ──────────────────────────────────────────────

/// Token bucket rate limiter.
///
/// Implements the token bucket algorithm for rate limiting. Tokens are added
/// at a fixed rate (`rate_per_sec`) up to a maximum burst size (`burst_size`).
/// Each request consumes one token. If no tokens are available, the request
/// is rate-limited.
#[derive(Debug)]
pub struct TokenBucket {
    /// Tokens currently available
    tokens: AtomicU64,
    /// Maximum tokens (burst size)
    capacity: u64,
    /// Tokens added per second
    rate_per_sec: u64,
    /// Last refill timestamp (nanoseconds)
    last_refill: AtomicU64,
}

impl TokenBucket {
    /// Create a new token bucket with the given rate and burst capacity.
    pub fn new(rate_per_sec: u64, burst_size: u64) -> Self {
        let burst = if burst_size > 0 {
            burst_size
        } else {
            rate_per_sec
        };
        Self {
            tokens: AtomicU64::new(burst),
            capacity: burst,
            rate_per_sec,
            last_refill: AtomicU64::new(Self::now_nanos()),
        }
    }

    /// Try to consume one token. Returns `true` if allowed.
    pub fn try_consume(&self) -> bool {
        self.refill();
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
            if self
                .tokens
                .compare_exchange(current, current - 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&self) {
        let now = Self::now_nanos();
        let last = self.last_refill.load(Ordering::Relaxed);
        if now <= last {
            return;
        }
        // Only one thread should refill at a time
        if self
            .last_refill
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return; // Another thread already refilled
        }
        let elapsed_ns = now - last;
        let tokens_to_add = (elapsed_ns as u128 * self.rate_per_sec as u128) / 1_000_000_000;
        if tokens_to_add > 0 {
            let new_tokens = (self.tokens.load(Ordering::Relaxed) as u128)
                .saturating_add(tokens_to_add)
                .min(self.capacity as u128) as u64;
            self.tokens.store(new_tokens, Ordering::Relaxed);
        }
    }

    fn now_nanos() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

// ── Sliding window error tracker ───────────────────────────────────────────

/// Tracks error rates within a sliding time window for overload detection.
#[derive(Debug)]
pub struct ErrorTracker {
    /// Window duration in seconds
    window_secs: u64,
    /// Ring buffer of (timestamp_secs, is_error) entries
    entries: std::sync::Mutex<Vec<(u64, bool)>>,
}

impl ErrorTracker {
    /// Create a new error tracker with the given window size.
    pub fn new(window_secs: u64) -> Self {
        Self {
            window_secs,
            entries: std::sync::Mutex::new(Vec::with_capacity(1024)),
        }
    }

    /// Record a request outcome.
    pub fn record(&self, is_error: bool) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if let Ok(mut entries) = self.entries.lock() {
            entries.push((now, is_error));
            // Trim old entries
            let cutoff = now.saturating_sub(self.window_secs);
            entries.retain(|(ts, _)| *ts >= cutoff);
        }
    }

    /// Get the current error rate (0.0 to 1.0) within the window.
    /// Returns 0.0 if there are no entries.
    pub fn error_rate(&self) -> f64 {
        if let Ok(entries) = self.entries.lock() {
            if entries.is_empty() {
                return 0.0;
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let cutoff = now.saturating_sub(self.window_secs);
            let total = entries.iter().filter(|(ts, _)| *ts >= cutoff).count();
            let errors = entries
                .iter()
                .filter(|(ts, is_err)| *ts >= cutoff && *is_err)
                .count();
            if total == 0 {
                0.0
            } else {
                errors as f64 / total as f64
            }
        } else {
            0.0
        }
    }
}

// ── LoadManager ────────────────────────────────────────────────────────────

/// Outcome of a request processed through the load manager.
///
/// Used to feed back request results for error rate tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// Request succeeded
    Success,
    /// Request failed with an error
    Failure,
}

/// Central load management coordinator.
///
/// Combines rate limiting, concurrency control, and load shedding into a single
/// admission control API. Used by the HTTP server and agent loop to degrade
/// gracefully under load.
#[derive(Debug)]
pub struct LoadManager {
    /// Configuration
    config: LoadConfig,
    /// Token bucket for rate limiting
    rate_limiter: Option<TokenBucket>,
    /// Semaphore for concurrency limiting
    concurrency_limiter: Option<Arc<Semaphore>>,
    /// Error rate tracker for overload detection
    error_tracker: ErrorTracker,
    /// Current queue depth estimate
    queue_depth: AtomicU64,
    /// Peak queue depth seen
    peak_queue_depth: AtomicU64,
    /// Total requests admitted
    total_admitted: AtomicU64,
    /// Total requests rejected
    total_rejected: AtomicU64,
    /// Start time
    start_time: Instant,
}

impl LoadManager {
    /// Create a new load manager from configuration.
    pub fn new(config: LoadConfig) -> Self {
        let rate_limiter = if config.enabled && config.rate_limit_per_second > 0 {
            Some(TokenBucket::new(
                config.rate_limit_per_second,
                config.rate_limit_burst,
            ))
        } else {
            None
        };

        let concurrency_limiter = if config.enabled && config.max_concurrent_requests > 0 {
            Some(Arc::new(Semaphore::new(config.max_concurrent_requests)))
        } else {
            None
        };

        Self {
            error_tracker: ErrorTracker::new(config.overload_window_secs),
            rate_limiter,
            concurrency_limiter,
            queue_depth: AtomicU64::new(0),
            peak_queue_depth: AtomicU64::new(0),
            total_admitted: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
            start_time: Instant::now(),
            config,
        }
    }

    /// Check whether a request should be admitted.
    ///
    /// Returns `Admission::Allowed` if the request can proceed, or a rejection
    /// reason if the system is under load.
    pub fn check_admission(&self) -> Admission {
        if !self.config.enabled {
            return Admission::Allowed;
        }

        // 1. Check queue depth — shed if too deep
        let depth = self.queue_depth.load(Ordering::Relaxed);
        if self.config.shed_load_at_queue_depth > 0
            && depth > self.config.shed_load_at_queue_depth as u64
        {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            warn!(
                queue_depth = depth,
                threshold = self.config.shed_load_at_queue_depth,
                "Load shedding: queue depth exceeded threshold"
            );
            return Admission::LoadShed;
        }

        // 2. Check error rate — shed if too many errors
        let error_rate = self.error_tracker.error_rate();
        let threshold = self.config.overload_error_threshold as f64 / 100.0;
        if error_rate > threshold && depth > 10 {
            // Only shed if there's also a queue building
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            warn!(
                error_rate = %format!("{:.1}%", error_rate * 100.0),
                threshold = %format!("{}%", self.config.overload_error_threshold),
                "Load shedding: error rate exceeded threshold"
            );
            return Admission::LoadShed;
        }

        // 3. Check rate limit
        if let Some(ref limiter) = self.rate_limiter {
            if !limiter.try_consume() {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                debug!("Rate limit exceeded");
                return Admission::RateLimited;
            }
        }

        // 4. Check concurrency limit
        if let Some(ref semaphore) = self.concurrency_limiter {
            if semaphore.available_permits() == 0 {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                debug!("Concurrency limit reached");
                return Admission::ConcurrencyLimited;
            }
        }

        self.total_admitted.fetch_add(1, Ordering::Relaxed);
        Admission::Allowed
    }

    /// Try to acquire a concurrency permit.
    ///
    /// Returns `Some(permit)` if a permit was acquired, or `None` if the
    /// concurrency limit is reached. The permit is automatically returned
    /// when dropped.
    #[allow(dead_code)]
    pub async fn acquire_permit(&self) -> Option<OwnedSemaphorePermit> {
        if !self.config.enabled {
            return None;
        }
        match self.concurrency_limiter.as_ref() {
            Some(semaphore) => {
                let permit = semaphore.clone().acquire_owned().await.ok()?;
                Some(permit)
            }
            None => None,
        }
    }

    /// Record a request outcome for error rate tracking.
    pub fn record_outcome(&self, outcome: RequestOutcome) {
        match outcome {
            RequestOutcome::Success => {
                self.error_tracker.record(false);
            }
            RequestOutcome::Failure => {
                self.error_tracker.record(true);
            }
        }
    }

    /// Update the queue depth estimate.
    #[allow(dead_code)]
    pub fn set_queue_depth(&self, depth: u64) {
        self.queue_depth.store(depth, Ordering::Relaxed);
        let peak = self.peak_queue_depth.load(Ordering::Relaxed);
        if depth > peak {
            let _ = self.peak_queue_depth.compare_exchange(
                peak,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
        }
    }

    /// Get current load metrics.
    pub fn metrics(&self) -> LoadMetrics {
        LoadMetrics {
            queue_depth: self.queue_depth.load(Ordering::Relaxed),
            peak_queue_depth: self.peak_queue_depth.load(Ordering::Relaxed),
            total_admitted: self.total_admitted.load(Ordering::Relaxed),
            total_rejected: self.total_rejected.load(Ordering::Relaxed),
            error_rate: self.error_tracker.error_rate(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            available_permits: self
                .concurrency_limiter
                .as_ref()
                .map(|s| s.available_permits())
                .unwrap_or(0),
        }
    }
}

/// Snapshot of load manager metrics.
#[derive(Debug, Clone, Serialize)]
pub struct LoadMetrics {
    /// Current estimated queue depth
    pub queue_depth: u64,
    /// Peak queue depth seen
    pub peak_queue_depth: u64,
    /// Total requests admitted
    pub total_admitted: u64,
    /// Total requests rejected (rate limited, load shed, etc.)
    pub total_rejected: u64,
    /// Current error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Available concurrency permits
    pub available_permits: usize,
}

impl LoadMetrics {
    /// Format as Prometheus-style text for `/metrics` endpoint.
    pub fn to_prometheus_text(&self) -> String {
        format!(
            "# HELP ravenclaws_load_queue_depth Current estimated queue depth\n\
             # TYPE ravenclaws_load_queue_depth gauge\n\
             ravenclaws_load_queue_depth {}\n\
             \n\
             # HELP ravenclaws_load_peak_queue_depth Peak queue depth seen\n\
             # TYPE ravenclaws_load_peak_queue_depth gauge\n\
             ravenclaws_load_peak_queue_depth {}\n\
             \n\
             # HELP ravenclaws_load_total_admitted Total requests admitted\n\
             # TYPE ravenclaws_load_total_admitted counter\n\
             ravenclaws_load_total_admitted {}\n\
             \n\
             # HELP ravenclaws_load_total_rejected Total requests rejected\n\
             # TYPE ravenclaws_load_total_rejected counter\n\
             ravenclaws_load_total_rejected {}\n\
             \n\
             # HELP ravenclaws_load_error_rate Current error rate (0.0-1.0)\n\
             # TYPE ravenclaws_load_error_rate gauge\n\
             ravenclaws_load_error_rate {:.4}\n\
             \n\
             # HELP ravenclaws_load_available_permits Available concurrency permits\n\
             # TYPE ravenclaws_load_available_permits gauge\n\
             ravenclaws_load_available_permits {}\n",
            self.queue_depth,
            self.peak_queue_depth,
            self.total_admitted,
            self.total_rejected,
            self.error_rate,
            self.available_permits,
        )
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_allows_initial_burst() {
        let bucket = TokenBucket::new(10, 10);
        for _ in 0..10 {
            assert!(bucket.try_consume(), "Should allow up to burst size");
        }
        // 11th should fail (no refill yet)
        assert!(!bucket.try_consume(), "Should deny after burst exhausted");
    }

    #[test]
    fn test_token_bucket_zero_rate_allows_none() {
        let bucket = TokenBucket::new(0, 0);
        assert!(!bucket.try_consume(), "Zero rate should deny all");
    }

    #[test]
    fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(1000, 1000);
        // Exhaust the bucket
        for _ in 0..1000 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume(), "Should be exhausted");

        // Simulate time passing by setting last_refill back
        let past = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            - 1_500_000_000; // 1.5 seconds ago
        bucket.last_refill.store(past, Ordering::Relaxed);

        // Should have ~1500 tokens now (1000/s * 1.5s, capped at 1000)
        assert!(bucket.try_consume(), "Should refill after time passes");
    }

    #[test]
    fn test_error_tracker_empty() {
        let tracker = ErrorTracker::new(60);
        assert_eq!(
            tracker.error_rate(),
            0.0,
            "Empty tracker should have 0 rate"
        );
    }

    #[test]
    fn test_error_tracker_all_success() {
        let tracker = ErrorTracker::new(60);
        for _ in 0..10 {
            tracker.record(false);
        }
        assert_eq!(tracker.error_rate(), 0.0, "All success should have 0 rate");
    }

    #[test]
    fn test_error_tracker_all_errors() {
        let tracker = ErrorTracker::new(60);
        for _ in 0..10 {
            tracker.record(true);
        }
        assert_eq!(tracker.error_rate(), 1.0, "All errors should have 1.0 rate");
    }

    #[test]
    fn test_error_tracker_mixed() {
        let tracker = ErrorTracker::new(60);
        for _ in 0..3 {
            tracker.record(true); // 3 errors
        }
        for _ in 0..7 {
            tracker.record(false); // 7 successes
        }
        let rate = tracker.error_rate();
        assert!(
            (rate - 0.3).abs() < 0.01,
            "Expected 0.3 error rate, got {}",
            rate
        );
    }

    #[test]
    fn test_load_manager_disabled() {
        let config = LoadConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = LoadManager::new(config);
        assert_eq!(
            manager.check_admission(),
            Admission::Allowed,
            "Disabled load manager should allow all"
        );
    }

    #[test]
    fn test_load_manager_rate_limits() {
        let config = LoadConfig {
            enabled: true,
            rate_limit_per_second: 5,
            rate_limit_burst: 5,
            max_concurrent_requests: 0,
            shed_load_at_queue_depth: 0,
            overload_error_threshold: 100,
            ..Default::default()
        };
        let manager = LoadManager::new(config);

        // First 5 should be allowed (burst)
        for i in 0..5 {
            assert_eq!(
                manager.check_admission(),
                Admission::Allowed,
                "Request {} should be allowed (burst)",
                i
            );
        }

        // 6th should be rate limited
        assert_eq!(
            manager.check_admission(),
            Admission::RateLimited,
            "Should be rate limited after burst exhausted"
        );
    }

    #[test]
    fn test_load_manager_queue_depth_shedding() {
        let config = LoadConfig {
            enabled: true,
            shed_load_at_queue_depth: 5,
            rate_limit_per_second: 0,
            max_concurrent_requests: 0,
            overload_error_threshold: 100,
            ..Default::default()
        };
        let manager = LoadManager::new(config);
        manager.set_queue_depth(3);
        assert_eq!(
            manager.check_admission(),
            Admission::Allowed,
            "Should allow when queue depth is under threshold"
        );

        manager.set_queue_depth(10);
        assert_eq!(
            manager.check_admission(),
            Admission::LoadShed,
            "Should shed when queue depth exceeds threshold"
        );
    }

    #[test]
    fn test_load_manager_metrics() {
        let config = LoadConfig {
            enabled: true,
            rate_limit_per_second: 100,
            rate_limit_burst: 100,
            max_concurrent_requests: 10,
            shed_load_at_queue_depth: 0,
            overload_error_threshold: 100,
            ..Default::default()
        };
        let manager = LoadManager::new(config);

        // Admit some requests
        assert_eq!(manager.check_admission(), Admission::Allowed);
        manager.record_outcome(RequestOutcome::Success);
        manager.record_outcome(RequestOutcome::Failure);
        manager.set_queue_depth(5);

        let metrics = manager.metrics();
        assert_eq!(metrics.total_admitted, 1);
        assert_eq!(metrics.queue_depth, 5);
        assert_eq!(metrics.available_permits, 10);
        assert!((metrics.error_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_load_metrics_prometheus_format() {
        let metrics = LoadMetrics {
            queue_depth: 5,
            peak_queue_depth: 10,
            total_admitted: 100,
            total_rejected: 3,
            error_rate: 0.05,
            uptime_secs: 3600,
            available_permits: 47,
        };

        let text = metrics.to_prometheus_text();
        assert!(text.contains("ravenclaws_load_queue_depth 5"));
        assert!(text.contains("ravenclaws_load_peak_queue_depth 10"));
        assert!(text.contains("ravenclaws_load_total_admitted 100"));
        assert!(text.contains("ravenclaws_load_total_rejected 3"));
        assert!(text.contains("ravenclaws_load_error_rate 0.0500"));
        assert!(text.contains("ravenclaws_load_available_permits 47"));
    }

    #[test]
    fn test_admission_is_allowed() {
        assert!(Admission::Allowed.is_allowed());
        assert!(!Admission::RateLimited.is_allowed());
        assert!(!Admission::ConcurrencyLimited.is_allowed());
        assert!(!Admission::LoadShed.is_allowed());
    }

    #[tokio::test]
    async fn test_load_manager_concurrency_limit() {
        let config = LoadConfig {
            enabled: true,
            max_concurrent_requests: 2,
            rate_limit_per_second: 0,
            shed_load_at_queue_depth: 0,
            overload_error_threshold: 100,
            ..Default::default()
        };
        let manager = LoadManager::new(config);

        // Acquire two permits to exhaust the semaphore
        let _p1 = manager.acquire_permit().await;
        let _p2 = manager.acquire_permit().await;

        // Third should be concurrency limited (no permits available)
        assert_eq!(manager.check_admission(), Admission::ConcurrencyLimited);
    }
}
