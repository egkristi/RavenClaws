//! Self-healing engine — automatic detection, retry, and recovery of failed agents
//!
//! # Dead code note
//! Some fields and methods are `#[allow(dead_code)]` because they are used by
//! library tests but not yet by the binary target. As healing is wired into
//! more modules (swarm, heartbeat, background, patterns), these will become
//! used by the binary as well.

// Allow dead code for fields/methods used only by library tests
#![allow(dead_code)]
//!
//! RavenClaws's self-healing system extends the v1.2.0 retry logic from individual
//! LLM calls to the full agent, swarm, and task level. It provides:
//!
//! - **Failure tracking** — per-agent/worker failure records with timestamps
//! - **Circuit breakers** — per-agent circuit state (Closed/Open/HalfOpen)
//! - **Exponential backoff** — configurable retry delays for agent tasks
//! - **Auto-replacement** — detect dead workers and trigger replacement
//! - **Health checks** — verify agent health before delegation
//!
//! # Architecture
//!
//! ```text
//! SelfHealingEngine
//!   ├── HealingConfig — retry limits, backoff, circuit breaker settings
//!   ├── failure_records: HashMap<String, FailureRecord> — per-agent failures
//!   ├── circuit_breakers: HashMap<String, HealingCircuitBreaker> — per-agent circuits
//!   └── Methods:
//!       ├── record_failure() — track a failure for an agent
//!       ├── record_success() — reset failure count for an agent
//!       ├── is_healthy() — check if an agent is healthy (circuit breaker)
//!       ├── retry_with_backoff() — execute with exponential backoff
//!       ├── dead_workers() — get list of dead workers for replacement
//!       └── reset() — clear all failure records
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the self-healing engine.
#[derive(Debug, Clone)]
pub struct HealingConfig {
    /// Maximum retries for a single agent task (default: 3)
    pub max_retries: u32,
    /// Base delay for exponential backoff in ms (default: 1000)
    pub base_delay_ms: u64,
    /// Maximum delay in ms (default: 30000)
    pub max_delay_ms: u64,
    /// Jitter factor (0.0-1.0, default: 0.3)
    pub jitter: f64,
    /// Circuit breaker: failures before opening (default: 5)
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker: seconds before half-open (default: 30)
    pub circuit_breaker_recovery_secs: u64,
    /// Seconds before a worker is considered dead (default: 60)
    pub worker_dead_after_secs: u64,
    /// Maximum failed tasks before worker is marked dead (default: 10)
    pub max_failed_tasks: u32,
}

impl Default for HealingConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter: 0.3,
            circuit_breaker_threshold: 5,
            circuit_breaker_recovery_secs: 30,
            worker_dead_after_secs: 60,
            max_failed_tasks: 10,
        }
    }
}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

/// Circuit breaker state for a single agent/worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingCircuitState {
    /// Normal operation — requests allowed
    Closed,
    /// Failure threshold exceeded — requests blocked
    Open,
    /// Recovery period — one request allowed to test
    HalfOpen,
}

/// Per-agent circuit breaker tracking failure counts and state transitions.
#[derive(Debug, Clone)]
pub struct HealingCircuitBreaker {
    /// Current circuit state
    pub state: HealingCircuitState,
    /// Consecutive failure count
    pub failure_count: u32,
    /// Timestamp of last failure
    pub last_failure_time: Option<Instant>,
    /// Duration the circuit stays open before half-open
    pub open_duration: Duration,
    /// Threshold for opening the circuit
    pub threshold: u32,
}

impl HealingCircuitBreaker {
    /// Create a new circuit breaker with the given recovery duration and threshold.
    pub fn new(open_duration_secs: u64, threshold: u32) -> Self {
        Self {
            state: HealingCircuitState::Closed,
            failure_count: 0,
            last_failure_time: None,
            open_duration: Duration::from_secs(open_duration_secs),
            threshold,
        }
    }

    /// Record a success — resets failure count and closes the circuit.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = HealingCircuitState::Closed;
    }

    /// Record a failure — increments count and opens circuit if threshold exceeded.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());
        if self.failure_count >= self.threshold {
            self.state = HealingCircuitState::Open;
            warn!(
                failure_count = self.failure_count,
                threshold = self.threshold,
                "Circuit breaker opened"
            );
        }
    }

    /// Check if requests are allowed.
    /// Transitions from Open → HalfOpen after recovery duration.
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            HealingCircuitState::Closed => true,
            HealingCircuitState::Open => {
                if let Some(last) = self.last_failure_time {
                    if last.elapsed() >= self.open_duration {
                        self.state = HealingCircuitState::HalfOpen;
                        info!("Circuit breaker half-open — allowing test request");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            HealingCircuitState::HalfOpen => true,
        }
    }
}

// ---------------------------------------------------------------------------
// Failure tracking
// ---------------------------------------------------------------------------

/// A record of a failure for a specific agent or worker.
#[derive(Debug, Clone)]
pub struct FailureRecord {
    /// Agent/worker identifier
    pub agent_id: String,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Timestamp of first failure in this sequence
    pub first_failure: Instant,
    /// Timestamp of most recent failure
    pub last_failure: Instant,
    /// Error message from the most recent failure
    pub last_error: String,
    /// Whether this agent is considered dead
    pub is_dead: bool,
}

impl FailureRecord {
    /// Create a new failure record for an agent.
    pub fn new(agent_id: &str, error: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            failure_count: 1,
            first_failure: Instant::now(),
            last_failure: Instant::now(),
            last_error: error.to_string(),
            is_dead: false,
        }
    }

    /// Record another failure.
    pub fn add_failure(&mut self, error: &str) {
        self.failure_count += 1;
        self.last_failure = Instant::now();
        self.last_error = error.to_string();
    }

    /// Record a success — resets the failure count.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.is_dead = false;
    }

    /// Time since the last failure.
    pub fn time_since_last_failure(&self) -> Duration {
        self.last_failure.elapsed()
    }
}

// ---------------------------------------------------------------------------
// Self-healing engine
// ---------------------------------------------------------------------------

/// The self-healing engine tracks failures, manages circuit breakers, and
/// provides retry-with-backoff for agent tasks.
///
/// This is the central coordination point for all self-healing in RavenClaws.
/// It is shared across the agent loop, swarm orchestrator, heartbeat agent,
/// and background task manager via `Arc<RwLock<>>`.
#[derive(Debug)]
pub struct SelfHealingEngine {
    /// Configuration
    pub config: HealingConfig,
    /// Per-agent failure records
    failure_records: HashMap<String, FailureRecord>,
    /// Per-agent circuit breakers
    circuit_breakers: HashMap<String, HealingCircuitBreaker>,
}

impl SelfHealingEngine {
    /// Create a new self-healing engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: HealingConfig::default(),
            failure_records: HashMap::new(),
            circuit_breakers: HashMap::new(),
        }
    }

    /// Create a new self-healing engine with custom configuration.
    pub fn with_config(config: HealingConfig) -> Self {
        Self {
            config,
            failure_records: HashMap::new(),
            circuit_breakers: HashMap::new(),
        }
    }

    // ── Failure recording ──────────────────────────────────────────────

    /// Record a failure for an agent.
    ///
    /// Returns the updated failure count for that agent.
    pub fn record_failure(&mut self, agent_id: &str, error: &str) -> u32 {
        // Update failure record
        let record = self
            .failure_records
            .entry(agent_id.to_string())
            .and_modify(|r| r.add_failure(error))
            .or_insert_with(|| FailureRecord::new(agent_id, error));

        // Check if agent should be marked dead
        if record.failure_count >= self.config.max_failed_tasks {
            record.is_dead = true;
            warn!(
                agent_id = %agent_id,
                failure_count = record.failure_count,
                max_failed = self.config.max_failed_tasks,
                "Agent marked as dead"
            );
        }

        // Update circuit breaker
        let cb = self
            .circuit_breakers
            .entry(agent_id.to_string())
            .or_insert_with(|| {
                HealingCircuitBreaker::new(
                    self.config.circuit_breaker_recovery_secs,
                    self.config.circuit_breaker_threshold,
                )
            });
        cb.record_failure();

        record.failure_count
    }

    /// Record a success for an agent — resets failure tracking.
    pub fn record_success(&mut self, agent_id: &str) {
        if let Some(record) = self.failure_records.get_mut(agent_id) {
            record.record_success();
        }
        if let Some(cb) = self.circuit_breakers.get_mut(agent_id) {
            cb.record_success();
        }
    }

    // ── Health checks ──────────────────────────────────────────────────

    /// Check if an agent is healthy.
    ///
    /// An agent is healthy if:
    /// 1. Its circuit breaker allows execution (Closed or HalfOpen)
    /// 2. It's not marked as dead
    pub fn is_healthy(&mut self, agent_id: &str) -> bool {
        // Check circuit breaker
        if let Some(cb) = self.circuit_breakers.get_mut(agent_id) {
            if !cb.can_execute() {
                debug!(
                    agent_id = %agent_id,
                    state = ?cb.state,
                    "Agent blocked by circuit breaker"
                );
                return false;
            }
        }

        // Check if marked dead
        if let Some(record) = self.failure_records.get(agent_id) {
            if record.is_dead {
                // Check if enough time has passed to revive
                if record.time_since_last_failure()
                    > Duration::from_secs(self.config.worker_dead_after_secs)
                {
                    debug!(
                        agent_id = %agent_id,
                        "Agent revived after dead timeout"
                    );
                    return true;
                }
                return false;
            }
        }

        true
    }

    /// Get the failure count for an agent.
    pub fn failure_count(&self, agent_id: &str) -> u32 {
        self.failure_records
            .get(agent_id)
            .map(|r| r.failure_count)
            .unwrap_or(0)
    }

    /// Get the circuit state for an agent.
    pub fn circuit_state(&self, agent_id: &str) -> Option<HealingCircuitState> {
        self.circuit_breakers.get(agent_id).map(|cb| cb.state)
    }

    // ── Dead worker detection ──────────────────────────────────────────

    /// Get a list of agent IDs that are marked as dead and past the replacement timeout.
    pub fn dead_workers(&self) -> Vec<String> {
        self.failure_records
            .iter()
            .filter(|(_, r)| {
                r.is_dead
                    && r.time_since_last_failure()
                        > Duration::from_secs(self.config.worker_dead_after_secs)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get a list of all agent IDs that are currently dead (regardless of timeout).
    pub fn all_dead_workers(&self) -> Vec<String> {
        self.failure_records
            .iter()
            .filter(|(_, r)| r.is_dead)
            .map(|(id, _)| id.clone())
            .collect()
    }

    // ── Retry with backoff ─────────────────────────────────────────────

    /// Calculate delay for a retry attempt using exponential backoff with jitter.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        use rand::Rng;
        let exp = 2u64.pow(attempt);
        let base = self.config.base_delay_ms * exp;
        let capped = base.min(self.config.max_delay_ms);
        let jitter_range = (capped as f64) * self.config.jitter;
        let jitter = rand::thread_rng().gen_range(-jitter_range..=jitter_range) as u64;
        let delay = capped.saturating_add(jitter).max(self.config.base_delay_ms);
        Duration::from_millis(delay)
    }

    /// Execute an async function with retry and exponential backoff.
    ///
    /// The function receives the attempt number (0-based) and should return
    /// `Ok(T)` on success or `Err(String)` on failure.
    /// Only transient errors should be retried — pass `is_transient` to classify.
    ///
    /// Returns `Ok(T)` on success, or `Err(String)` with the last error after
    /// all retries are exhausted.
    pub async fn retry_with_backoff<F, T>(
        &self,
        agent_id: &str,
        is_transient: impl Fn(&str) -> bool,
        mut operation: F,
    ) -> Result<T, String>
    where
        F: FnMut(u32) -> Result<T, String>,
    {
        let max_attempts = self.config.max_retries + 1; // +1 for the initial attempt
        let mut last_error = String::from("Unknown error");

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = self.delay_for_attempt(attempt - 1);
                debug!(
                    agent_id = %agent_id,
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying after backoff"
                );
                tokio::time::sleep(delay).await;
            }

            match operation(attempt) {
                Ok(result) => {
                    if attempt > 0 {
                        info!(
                            agent_id = %agent_id,
                            attempt = attempt,
                            "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = e.clone();
                    if !is_transient(&e) {
                        debug!(
                            agent_id = %agent_id,
                            attempt = attempt,
                            error = %e,
                            "Non-transient error — not retrying"
                        );
                        return Err(e);
                    }
                    warn!(
                        agent_id = %agent_id,
                        attempt = attempt,
                        max_attempts = max_attempts,
                        error = %e,
                        "Transient error — will retry"
                    );
                }
            }
        }

        Err(format!(
            "All {} attempts failed for agent '{}': {}",
            max_attempts, agent_id, last_error
        ))
    }

    // ── Reset ──────────────────────────────────────────────────────────

    /// Reset all failure records and circuit breakers.
    pub fn reset(&mut self) {
        self.failure_records.clear();
        self.circuit_breakers.clear();
        info!("Self-healing engine reset — all failure records cleared");
    }

    /// Reset failure records for a specific agent.
    pub fn reset_agent(&mut self, agent_id: &str) {
        self.failure_records.remove(agent_id);
        self.circuit_breakers.remove(agent_id);
        debug!(agent_id = %agent_id, "Self-healing record reset for agent");
    }

    // ── Metrics ────────────────────────────────────────────────────────

    /// Get the total number of tracked agents.
    pub fn tracked_agents(&self) -> usize {
        self.failure_records.len()
    }

    /// Get the number of dead agents.
    pub fn dead_agent_count(&self) -> usize {
        self.all_dead_workers().len()
    }

    /// Get the number of agents with open circuit breakers.
    pub fn open_circuit_count(&self) -> usize {
        self.circuit_breakers
            .values()
            .filter(|cb| cb.state == HealingCircuitState::Open)
            .count()
    }
}

impl Default for SelfHealingEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healing_config_default() {
        let config = HealingConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.circuit_breaker_threshold, 5);
        assert_eq!(config.circuit_breaker_recovery_secs, 30);
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let mut cb = HealingCircuitBreaker::new(30, 5);
        assert_eq!(cb.state, HealingCircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let mut cb = HealingCircuitBreaker::new(30, 3);
        assert!(cb.can_execute());

        cb.record_failure();
        assert!(cb.can_execute());
        assert_eq!(cb.failure_count, 1);

        cb.record_failure();
        assert!(cb.can_execute());
        assert_eq!(cb.failure_count, 2);

        cb.record_failure();
        assert_eq!(cb.state, HealingCircuitState::Open);
        assert!(!cb.can_execute());
        assert_eq!(cb.failure_count, 3);
    }

    #[test]
    fn test_circuit_breaker_success_resets() {
        let mut cb = HealingCircuitBreaker::new(30, 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count, 2);

        cb.record_success();
        assert_eq!(cb.state, HealingCircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_failure_record_new() {
        let record = FailureRecord::new("agent-1", "timeout");
        assert_eq!(record.agent_id, "agent-1");
        assert_eq!(record.failure_count, 1);
        assert_eq!(record.last_error, "timeout");
        assert!(!record.is_dead);
    }

    #[test]
    fn test_failure_record_add_failure() {
        let mut record = FailureRecord::new("agent-1", "timeout");
        record.add_failure("connection refused");
        assert_eq!(record.failure_count, 2);
        assert_eq!(record.last_error, "connection refused");
    }

    #[test]
    fn test_failure_record_success_resets() {
        let mut record = FailureRecord::new("agent-1", "timeout");
        record.add_failure("connection refused");
        assert_eq!(record.failure_count, 2);

        record.record_success();
        assert_eq!(record.failure_count, 0);
        assert!(!record.is_dead);
    }

    #[test]
    fn test_self_healing_engine_new() {
        let engine = SelfHealingEngine::new();
        assert_eq!(engine.tracked_agents(), 0);
        assert_eq!(engine.dead_agent_count(), 0);
    }

    #[test]
    fn test_self_healing_engine_record_failure() {
        let mut engine = SelfHealingEngine::new();
        let count = engine.record_failure("agent-1", "timeout");
        assert_eq!(count, 1);
        assert_eq!(engine.failure_count("agent-1"), 1);
        assert!(engine.is_healthy("agent-1"));
    }

    #[test]
    fn test_self_healing_engine_marks_dead() {
        let mut engine = SelfHealingEngine::new();
        engine.config.max_failed_tasks = 3;

        engine.record_failure("agent-1", "error 1");
        engine.record_failure("agent-1", "error 2");
        engine.record_failure("agent-1", "error 3");

        assert!(!engine.is_healthy("agent-1"));
        assert!(engine.all_dead_workers().contains(&"agent-1".to_string()));
    }

    #[test]
    fn test_self_healing_engine_circuit_breaker() {
        let mut engine = SelfHealingEngine::new();
        engine.config.circuit_breaker_threshold = 3;

        // 3 failures should open the circuit
        engine.record_failure("agent-1", "error 1");
        engine.record_failure("agent-1", "error 2");
        engine.record_failure("agent-1", "error 3");

        assert_eq!(
            engine.circuit_state("agent-1"),
            Some(HealingCircuitState::Open)
        );
        assert!(!engine.is_healthy("agent-1"));
    }

    #[test]
    fn test_self_healing_engine_success_resets() {
        let mut engine = SelfHealingEngine::new();
        engine.config.circuit_breaker_threshold = 3;

        engine.record_failure("agent-1", "error 1");
        engine.record_failure("agent-1", "error 2");
        assert_eq!(engine.failure_count("agent-1"), 2);

        engine.record_success("agent-1");
        assert_eq!(engine.failure_count("agent-1"), 0);
        assert_eq!(
            engine.circuit_state("agent-1"),
            Some(HealingCircuitState::Closed)
        );
    }

    #[test]
    fn test_delay_for_attempt() {
        let engine = SelfHealingEngine::new();
        // Attempt 0: base * 2^0 = 1000ms + jitter
        let d0 = engine.delay_for_attempt(0);
        assert!(d0.as_millis() >= 700); // 1000 - 30% jitter
        assert!(d0.as_millis() <= 1300); // 1000 + 30% jitter

        // Attempt 1: base * 2^1 = 2000ms + jitter
        let d1 = engine.delay_for_attempt(1);
        assert!(d1.as_millis() >= 1400);
        assert!(d1.as_millis() <= 2600);

        // Attempt 4: base * 2^4 = 16000ms, capped at 30000ms
        let d4 = engine.delay_for_attempt(4);
        assert!(d4.as_millis() <= 30000 + 9000); // max + jitter
    }

    #[test]
    fn test_reset() {
        let mut engine = SelfHealingEngine::new();
        engine.record_failure("agent-1", "error");
        engine.record_failure("agent-2", "error");
        assert_eq!(engine.tracked_agents(), 2);

        engine.reset();
        assert_eq!(engine.tracked_agents(), 0);
    }

    #[test]
    fn test_reset_agent() {
        let mut engine = SelfHealingEngine::new();
        engine.record_failure("agent-1", "error");
        engine.record_failure("agent-2", "error");
        assert_eq!(engine.tracked_agents(), 2);

        engine.reset_agent("agent-1");
        assert_eq!(engine.tracked_agents(), 1);
        assert_eq!(engine.failure_count("agent-1"), 0);
    }

    #[test]
    fn test_dead_workers_empty_initially() {
        let engine = SelfHealingEngine::new();
        assert!(engine.dead_workers().is_empty());
        assert!(engine.all_dead_workers().is_empty());
    }

    #[test]
    fn test_open_circuit_count() {
        let mut engine = SelfHealingEngine::new();
        engine.config.circuit_breaker_threshold = 2;

        engine.record_failure("agent-1", "error");
        engine.record_failure("agent-1", "error");
        engine.record_failure("agent-2", "error");
        engine.record_failure("agent-2", "error");

        assert_eq!(engine.open_circuit_count(), 2);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_succeeds_on_first_attempt() {
        let engine = SelfHealingEngine::new();
        let result = engine
            .retry_with_backoff("agent-1", |_| true, |_| Ok::<_, String>(42))
            .await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_succeeds_after_retries() {
        let engine = SelfHealingEngine::new();
        let attempt = std::sync::atomic::AtomicU32::new(0);

        let result = engine
            .retry_with_backoff(
                "agent-1",
                |_| true,
                |_| {
                    let prev = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if prev < 2 {
                        Err("transient error".to_string())
                    } else {
                        Ok(42)
                    }
                },
            )
            .await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_fails_on_non_transient() {
        let engine = SelfHealingEngine::new();
        let result: Result<i32, String> = engine
            .retry_with_backoff(
                "agent-1",
                |e| !e.contains("non-transient"),
                |_| Err("non-transient error".to_string()),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-transient"));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_exhausts_retries() {
        let mut engine = SelfHealingEngine::new();
        engine.config.max_retries = 2; // 3 total attempts (0, 1, 2)

        let result: Result<i32, String> = engine
            .retry_with_backoff(
                "agent-1",
                |_| true,
                |_| Err("persistent transient error".to_string()),
            )
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("All 3 attempts failed"));
        assert!(err.contains("persistent transient error"));
    }
}
