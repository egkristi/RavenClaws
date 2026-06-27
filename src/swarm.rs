//! Swarm orchestration — self-provisioning sub-agents with recursive supervision
//!
//! RavenClaws's swarm orchestrator enables truly autonomous multi-agent coordination:
//! supervisors can spawn sub-supervisors, creating recursive task decomposition
//! trees of arbitrary depth. Each worker has a declarative profile specifying its
//! persona, tools, provider, model, and resource limits.
//!
//! # Architecture
//!
//! ```text
//! SwarmOrchestrator
//!   ├── topology: Star | Mesh | Hierarchical | Hybrid
//!   ├── max_depth: recursion limit (default: 3)
//!   ├── max_workers: total worker cap (default: 100)
//!   ├── profiles: Vec<WorkerProfile> — available worker types
//!   ├── message_bus: AgentMessageBus — inter-agent communication
//!   └── orchestrate(task):
//!       1. Analyze task → determine required roles
//!       2. Assign roles → select profiles
//!       3. Spawn workers → local or remote via RavenFabric
//!       4. Coordinate → recursive supervisor if task is complex
//!       5. Aggregate → collect and merge results
//! ```
//!
//! # Inter-Agent Communication
//!
//! When `enable_agent_communication` is set, swarm workers can exchange messages
//! through a shared `AgentMessageBus`. Messages are routed by role and include
//! typed payloads (information, question, result, error, coordination).
//! All messages are audited and timestamped.
//!
//! # Integration
//!
//! - CLI: `--swarm-topology <star|mesh|hierarchical|hybrid> --max-workers <N>`
//! - Config: `[swarm]` section in `ravenclaws.toml`
//! - Supervisor mode: automatically uses orchestrator when `max_depth > 0`

use crate::agent::ConversationMemory;
use crate::audit::{AuditEventType, AuditLog};
use crate::error::{RavenClawsError, Result};
use crate::llm::{ChatMessage, LLMProviderTrait, MultiModelManager};
use crate::policy::PolicyEngine;
use crate::ravenfabric::RavenFabricClient;
use crate::sandbox::Sandbox;
use crate::tools::ToolRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

// ---------------------------------------------------------------------------
// Inter-agent communication
// ---------------------------------------------------------------------------

/// A message exchanged between swarm workers.
///
/// Messages are typed so workers can filter and respond appropriately.
/// All messages are timestamped and include the sender's role for auditability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID
    pub id: String,

    /// Sender's worker role (e.g., "researcher", "executor")
    pub sender: String,

    /// Recipient's worker role ("*" = broadcast to all)
    pub recipient: String,

    /// Message type
    pub msg_type: MessageType,

    /// Message content
    pub content: String,

    /// ISO 8601 timestamp
    pub timestamp: String,

    /// Optional metadata (e.g., task ID, iteration number)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// The type of an inter-agent message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Sharing information or findings
    Information,
    /// Asking a question or requesting input
    Question,
    /// Reporting a result or completion
    Result,
    /// Reporting an error or issue
    Error,
    /// Coordination message (e.g., task re-assignment)
    Coordination,
    /// General purpose message
    Generic,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Information => write!(f, "information"),
            MessageType::Question => write!(f, "question"),
            MessageType::Result => write!(f, "result"),
            MessageType::Error => write!(f, "error"),
            MessageType::Coordination => write!(f, "coordination"),
            MessageType::Generic => write!(f, "generic"),
        }
    }
}

/// A shared message bus for inter-agent communication within a swarm.
///
/// The message bus is shared across all workers in a swarm via `Arc<RwLock<>>`.
/// Workers can send messages to specific roles or broadcast to all.
/// Messages are retained for the lifetime of the swarm and can be queried.
#[derive(Debug, Clone)]
pub struct AgentMessageBus {
    /// All messages sent in this swarm session
    messages: Vec<AgentMessage>,
    /// Maximum messages to retain (0 = unlimited)
    max_messages: usize,
}

#[allow(dead_code)]
impl AgentMessageBus {
    /// Create a new message bus with the given capacity.
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    /// Send a message to a specific recipient (or "*" for broadcast).
    pub fn send(
        &mut self,
        sender: &str,
        recipient: &str,
        msg_type: MessageType,
        content: &str,
        metadata: HashMap<String, String>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();

        let msg = AgentMessage {
            id: id.clone(),
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            msg_type,
            content: content.to_string(),
            timestamp,
            metadata,
        };

        self.messages.push(msg);

        // Trim oldest messages if over capacity
        if self.max_messages > 0 && self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }

        id
    }

    /// Get all messages addressed to a specific role (or "*" for broadcast).
    pub fn messages_for(&self, role: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.recipient == role || m.recipient == "*")
            .collect()
    }

    /// Get all messages from a specific sender.
    pub fn messages_from(&self, sender: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.sender == sender)
            .collect()
    }

    /// Get all messages of a specific type.
    pub fn messages_of_type(&self, msg_type: &MessageType) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.msg_type == *msg_type)
            .collect()
    }

    /// Get all messages in the bus.
    pub fn all_messages(&self) -> &[AgentMessage] {
        &self.messages
    }

    /// Get the number of messages in the bus.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the bus is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Format recent messages for inclusion in an LLM prompt.
    ///
    /// Returns a string with the last N messages formatted for the agent's context.
    pub fn format_for_prompt(&self, role: &str, max_messages: usize) -> String {
        let relevant: Vec<&AgentMessage> = self
            .messages
            .iter()
            .filter(|m| m.recipient == role || m.recipient == "*" || m.sender == role)
            .rev()
            .take(max_messages)
            .collect();

        if relevant.is_empty() {
            return String::new();
        }

        let mut output = String::from("\n\n--- Inter-Agent Messages ---\n");
        for msg in relevant.iter().rev() {
            output.push_str(&format!(
                "[{}] {} → {} ({}): {}\n",
                msg.msg_type, msg.sender, msg.recipient, msg.timestamp, msg.content
            ));
        }
        output.push_str("--- End Inter-Agent Messages ---\n");
        output
    }
}

// ---------------------------------------------------------------------------
// Swarm health monitoring
// ---------------------------------------------------------------------------

/// Health status of a single swarm worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerHealthStatus {
    /// Worker is active and responding to heartbeats
    Healthy,
    /// Worker is running but has not responded to recent heartbeats
    Degraded,
    /// Worker has not responded within the timeout window
    Unhealthy,
    /// Worker has been terminated and needs replacement
    Dead,
}

impl std::fmt::Display for WorkerHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerHealthStatus::Healthy => write!(f, "healthy"),
            WorkerHealthStatus::Degraded => write!(f, "degraded"),
            WorkerHealthStatus::Unhealthy => write!(f, "unhealthy"),
            WorkerHealthStatus::Dead => write!(f, "dead"),
        }
    }
}

/// Telemetry snapshot for a single worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTelemetry {
    /// Worker role name
    pub role: String,
    /// Current health status
    pub status: WorkerHealthStatus,
    /// Tasks completed by this worker
    pub tasks_completed: u64,
    /// Tasks that failed
    pub tasks_failed: u64,
    /// Total errors encountered
    pub error_count: u64,
    /// Average task duration in milliseconds
    pub avg_duration_ms: f64,
    /// Last heartbeat timestamp (ISO 8601)
    pub last_heartbeat: String,
    /// When the worker was spawned (ISO 8601)
    pub spawned_at: String,
    /// Number of messages sent via the bus
    pub messages_sent: u64,
    /// Number of messages received via the bus
    pub messages_received: u64,
    /// Current iteration count
    pub iteration: u64,
}

/// Aggregate swarm health metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMetrics {
    /// Total workers in the swarm
    pub total_workers: usize,
    /// Healthy workers
    pub healthy_workers: usize,
    /// Degraded workers
    pub degraded_workers: usize,
    /// Unhealthy workers
    pub unhealthy_workers: usize,
    /// Dead workers
    pub dead_workers: usize,
    /// Total tasks completed across all workers
    pub total_tasks_completed: u64,
    /// Total tasks failed across all workers
    pub total_tasks_failed: u64,
    /// Total errors across all workers
    pub total_errors: u64,
    /// Overall average task duration in milliseconds
    pub overall_avg_duration_ms: f64,
    /// Task throughput (tasks per second over the last window)
    pub task_throughput: f64,
    /// Communication latency (average ms between send and receive)
    pub communication_latency_ms: f64,
    /// Worker utilization (0.0–1.0, ratio of busy time to total time)
    pub worker_utilization: f64,
    /// Error rate (errors per task)
    pub error_rate: f64,
    /// Timestamp of this metrics snapshot
    pub timestamp: String,
}

/// Per-worker heartbeat tracker.
#[derive(Debug, Clone)]
struct WorkerHeartbeat {
    /// Worker role
    role: String,
    /// When the worker was spawned
    spawned_at: chrono::DateTime<chrono::Utc>,
    /// Last heartbeat received
    last_heartbeat: chrono::DateTime<chrono::Utc>,
    /// Number of missed heartbeats
    missed_beats: u32,
    /// Current health status
    status: WorkerHealthStatus,
    /// Tasks completed
    tasks_completed: u64,
    /// Tasks failed
    tasks_failed: u64,
    /// Error count
    error_count: u64,
    /// Total duration of all completed tasks in milliseconds
    total_duration_ms: f64,
    /// Number of tasks with recorded duration
    duration_samples: u64,
    /// Messages sent
    messages_sent: u64,
    /// Messages received
    messages_received: u64,
    /// Current iteration
    iteration: u64,
    /// Whether this worker is currently busy
    is_busy: bool,
    /// When the current task started
    task_started_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Swarm health monitor — tracks worker health, detects failures, and
/// provides aggregate telemetry.
///
/// # Heartbeat Protocol
///
/// Workers send heartbeats at regular intervals. If a worker misses
/// `max_missed_beats` consecutive heartbeats, it is marked as `Unhealthy`.
/// After `max_missed_beats * 2` missed beats, it is marked as `Dead`.
///
/// # Dead-Agent Detection
///
/// The health monitor periodically scans all tracked workers. Workers that
/// have been `Dead` for longer than `replacement_timeout_secs` are candidates
/// for automatic replacement.
///
/// # Metrics
///
/// The monitor tracks task throughput, worker utilization, error rates, and
/// communication latency. These are exposed via `metrics()`.
#[derive(Debug, Clone)]
pub struct SwarmHealthMonitor {
    /// Per-worker heartbeat trackers
    heartbeats: HashMap<String, WorkerHeartbeat>,
    /// Heartbeat interval in seconds (default: 5)
    heartbeat_interval_secs: u64,
    /// Max missed heartbeats before marking unhealthy (default: 3)
    max_missed_beats: u32,
    /// Time in seconds before a dead worker is replaced (default: 30)
    replacement_timeout_secs: u64,
    /// When monitoring started
    started_at: chrono::DateTime<chrono::Utc>,
    /// Total messages sent across all workers (for latency calculation)
    total_messages_sent: u64,
    /// Total messages received across all workers
    total_messages_received: u64,
    /// Sum of all task durations for throughput calculation
    total_duration_ms: f64,
    /// Total tasks completed across all workers
    total_tasks_completed: u64,
}

impl Default for SwarmHealthMonitor {
    fn default() -> Self {
        Self {
            heartbeats: HashMap::new(),
            heartbeat_interval_secs: 5,
            max_missed_beats: 3,
            replacement_timeout_secs: 30,
            started_at: chrono::Utc::now(),
            total_messages_sent: 0,
            total_messages_received: 0,
            total_duration_ms: 0.0,
            total_tasks_completed: 0,
        }
    }
}

#[allow(dead_code)]
impl SwarmHealthMonitor {
    /// Create a new health monitor with custom parameters.
    pub fn new(
        heartbeat_interval_secs: u64,
        max_missed_beats: u32,
        replacement_timeout_secs: u64,
    ) -> Self {
        Self {
            heartbeats: HashMap::new(),
            heartbeat_interval_secs,
            max_missed_beats,
            replacement_timeout_secs,
            started_at: chrono::Utc::now(),
            total_messages_sent: 0,
            total_messages_received: 0,
            total_duration_ms: 0.0,
            total_tasks_completed: 0,
        }
    }

    /// Register a new worker for health tracking.
    pub fn register_worker(&mut self, role: &str) {
        let now = chrono::Utc::now();
        self.heartbeats
            .entry(role.to_string())
            .or_insert(WorkerHeartbeat {
                role: role.to_string(),
                spawned_at: now,
                last_heartbeat: now,
                missed_beats: 0,
                status: WorkerHealthStatus::Healthy,
                tasks_completed: 0,
                tasks_failed: 0,
                error_count: 0,
                total_duration_ms: 0.0,
                duration_samples: 0,
                messages_sent: 0,
                messages_received: 0,
                iteration: 0,
                is_busy: false,
                task_started_at: None,
            });
    }

    /// Record a heartbeat from a worker.
    pub fn heartbeat(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.last_heartbeat = chrono::Utc::now();
            hb.missed_beats = 0;
            hb.status = WorkerHealthStatus::Healthy;
        }
    }

    /// Record that a worker started a task.
    pub fn task_started(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.is_busy = true;
            hb.task_started_at = Some(chrono::Utc::now());
            hb.iteration += 1;
        }
    }

    /// Record that a worker completed a task successfully.
    pub fn task_completed(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.tasks_completed += 1;
            hb.is_busy = false;

            if let Some(started) = hb.task_started_at {
                let duration = (chrono::Utc::now() - started).num_milliseconds() as f64;
                hb.total_duration_ms += duration;
                hb.duration_samples += 1;
                self.total_duration_ms += duration;
            }
            self.total_tasks_completed += 1;
            hb.task_started_at = None;
        }
    }

    /// Record that a worker's task failed.
    pub fn task_failed(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.tasks_failed += 1;
            hb.error_count += 1;
            hb.is_busy = false;
            hb.task_started_at = None;
        }
    }

    /// Record an error from a worker.
    pub fn record_error(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.error_count += 1;
        }
    }

    /// Record a message sent by a worker.
    pub fn message_sent(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.messages_sent += 1;
        }
        self.total_messages_sent += 1;
    }

    /// Record a message received by a worker.
    pub fn message_received(&mut self, role: &str) {
        if let Some(hb) = self.heartbeats.get_mut(role) {
            hb.messages_received += 1;
        }
        self.total_messages_received += 1;
    }

    /// Scan all workers and update their health status based on heartbeat timing.
    /// Returns a list of workers that have been detected as dead.
    pub fn check_health(&mut self) -> Vec<String> {
        let now = chrono::Utc::now();
        let mut dead_workers = Vec::new();

        for hb in self.heartbeats.values_mut() {
            let elapsed = (now - hb.last_heartbeat).num_seconds();

            if elapsed > (self.heartbeat_interval_secs * self.max_missed_beats as u64 * 2) as i64 {
                if hb.status != WorkerHealthStatus::Dead {
                    hb.status = WorkerHealthStatus::Dead;
                    dead_workers.push(hb.role.clone());
                }
            } else if elapsed > (self.heartbeat_interval_secs * self.max_missed_beats as u64) as i64
            {
                hb.status = WorkerHealthStatus::Unhealthy;
            } else if elapsed > (self.heartbeat_interval_secs * 2) as i64 {
                hb.status = WorkerHealthStatus::Degraded;
            }
        }

        dead_workers
    }

    /// Get telemetry for a specific worker.
    pub fn worker_telemetry(&self, role: &str) -> Option<WorkerTelemetry> {
        self.heartbeats.get(role).map(|hb| {
            let avg_dur = if hb.duration_samples > 0 {
                hb.total_duration_ms / hb.duration_samples as f64
            } else {
                0.0
            };

            WorkerTelemetry {
                role: hb.role.clone(),
                status: hb.status.clone(),
                tasks_completed: hb.tasks_completed,
                tasks_failed: hb.tasks_failed,
                error_count: hb.error_count,
                avg_duration_ms: avg_dur,
                last_heartbeat: hb.last_heartbeat.to_rfc3339(),
                spawned_at: hb.spawned_at.to_rfc3339(),
                messages_sent: hb.messages_sent,
                messages_received: hb.messages_received,
                iteration: hb.iteration,
            }
        })
    }

    /// Get telemetry for all workers.
    pub fn all_worker_telemetry(&self) -> Vec<WorkerTelemetry> {
        let mut roles: Vec<String> = self.heartbeats.keys().cloned().collect();
        roles.sort();
        roles
            .iter()
            .filter_map(|r| self.worker_telemetry(r))
            .collect()
    }

    /// Get aggregate swarm metrics.
    pub fn metrics(&self) -> SwarmMetrics {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut dead = 0;
        let mut total_completed = 0u64;
        let mut total_failed = 0u64;
        let mut total_errors = 0u64;
        let mut busy_workers = 0u64;
        let total_workers = self.heartbeats.len();

        for hb in self.heartbeats.values() {
            match hb.status {
                WorkerHealthStatus::Healthy => healthy += 1,
                WorkerHealthStatus::Degraded => degraded += 1,
                WorkerHealthStatus::Unhealthy => unhealthy += 1,
                WorkerHealthStatus::Dead => dead += 1,
            }
            total_completed += hb.tasks_completed;
            total_failed += hb.tasks_failed;
            total_errors += hb.error_count;
            if hb.is_busy {
                busy_workers += 1;
            }
        }

        let elapsed_secs = (chrono::Utc::now() - self.started_at).num_seconds().max(1) as f64;
        let task_throughput = self.total_tasks_completed as f64 / elapsed_secs;

        let worker_utilization = if total_workers > 0 {
            busy_workers as f64 / total_workers as f64
        } else {
            0.0
        };

        let error_rate = if total_completed + total_failed > 0 {
            total_errors as f64 / (total_completed + total_failed) as f64
        } else {
            0.0
        };

        let overall_avg_duration = if self.total_tasks_completed > 0 {
            self.total_duration_ms / self.total_tasks_completed as f64
        } else {
            0.0
        };

        let comm_latency = if self.total_messages_sent > 0 {
            // Approximate: average time between send and receive is estimated
            // from the ratio of received to sent messages × average interval
            let ratio = self.total_messages_received as f64 / self.total_messages_sent as f64;
            ratio * (self.heartbeat_interval_secs as f64 * 1000.0) / 2.0
        } else {
            0.0
        };

        SwarmMetrics {
            total_workers,
            healthy_workers: healthy,
            degraded_workers: degraded,
            unhealthy_workers: unhealthy,
            dead_workers: dead,
            total_tasks_completed: total_completed,
            total_tasks_failed: total_failed,
            total_errors,
            overall_avg_duration_ms: overall_avg_duration,
            task_throughput,
            communication_latency_ms: comm_latency,
            worker_utilization,
            error_rate,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get workers that are candidates for replacement (dead for > replacement_timeout).
    pub fn dead_workers_for_replacement(&self) -> Vec<String> {
        let now = chrono::Utc::now();
        self.heartbeats
            .iter()
            .filter(|(_, hb)| hb.status == WorkerHealthStatus::Dead)
            .filter(|(_, hb)| {
                let elapsed = (now - hb.last_heartbeat).num_seconds();
                elapsed >= self.replacement_timeout_secs as i64
            })
            .map(|(role, _)| role.clone())
            .collect()
    }

    /// Remove a worker from tracking (after replacement).
    pub fn remove_worker(&mut self, role: &str) {
        self.heartbeats.remove(role);
    }

    /// Get the number of tracked workers.
    pub fn worker_count(&self) -> usize {
        self.heartbeats.len()
    }

    /// Format health status for logging.
    pub fn format_status(&self) -> String {
        let m = self.metrics();
        format!(
            "Swarm Health: {}/{} healthy, {} degraded, {} unhealthy, {} dead | \
             {} tasks ({:.1}/s) | {:.1}% utilization | {:.2}% error rate",
            m.healthy_workers,
            m.total_workers,
            m.degraded_workers,
            m.unhealthy_workers,
            m.dead_workers,
            m.total_tasks_completed,
            m.task_throughput,
            m.worker_utilization * 100.0,
            m.error_rate * 100.0,
        )
    }
}

// ---------------------------------------------------------------------------
// Worker profile
// ---------------------------------------------------------------------------

/// Declarative profile for a swarm worker agent.
///
/// Each worker has a unique combination of persona, capabilities, and resource
/// limits. Profiles are composable (can inherit from a base profile) and
/// inheritable (sub-profiles can override specific fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerProfile {
    /// Unique name for this profile (e.g., "researcher", "coder", "reviewer")
    pub name: String,

    /// Human-readable description of this worker's role
    #[serde(default)]
    pub description: String,

    /// System prompt / persona for this worker
    pub persona: String,

    /// Which tools this worker can use (empty = all available)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Provider override (empty = use default from config)
    #[serde(default)]
    pub provider: Option<String>,

    /// Model override (empty = use default from config)
    #[serde(default)]
    pub model: Option<String>,

    /// Maximum iterations for this worker's agent loop
    #[serde(default = "default_worker_max_iterations")]
    pub max_iterations: usize,

    /// Maximum memory messages for this worker
    #[serde(default = "default_worker_memory")]
    pub max_memory_messages: usize,

    /// Whether this worker can spawn sub-workers (recursive supervision)
    #[serde(default = "default_true")]
    pub can_delegate: bool,

    /// Resource limits
    #[serde(default)]
    pub resource_limits: ResourceLimits,
}

fn default_worker_max_iterations() -> usize {
    10
}

fn default_worker_memory() -> usize {
    20
}

fn default_true() -> bool {
    true
}

/// Resource limits for a worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum tool calls per tick
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: usize,

    /// Maximum total execution time in seconds
    #[serde(default = "default_max_exec_secs")]
    pub max_exec_secs: u64,
}

fn default_max_tool_calls() -> usize {
    50
}

fn default_max_exec_secs() -> u64 {
    300
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_tool_calls: default_max_tool_calls(),
            max_exec_secs: default_max_exec_secs(),
        }
    }
}

impl Default for WorkerProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: String::new(),
            persona: "You are a helpful assistant.".to_string(),
            allowed_tools: Vec::new(),
            provider: None,
            model: None,
            max_iterations: default_worker_max_iterations(),
            max_memory_messages: default_worker_memory(),
            can_delegate: default_true(),
            resource_limits: ResourceLimits::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in profiles
// ---------------------------------------------------------------------------

impl WorkerProfile {
    /// Analytical researcher profile — focused on logic, structure, and precision
    pub fn researcher() -> Self {
        Self {
            name: "researcher".to_string(),
            description: "Analytical researcher focused on data gathering and analysis".to_string(),
            persona: "You are an analytical researcher. Focus on gathering data, \
                      verifying facts, and providing well-structured analysis. \
                      Be thorough and cite your sources."
                .to_string(),
            allowed_tools: vec![
                "web_fetch".to_string(),
                "web_search".to_string(),
                "read_file".to_string(),
            ],
            max_iterations: 15,
            can_delegate: false,
            ..Default::default()
        }
    }

    /// Creative problem-solver profile — focused on innovation and alternatives
    pub fn creative() -> Self {
        Self {
            name: "creative".to_string(),
            description: "Creative problem-solver focused on innovation".to_string(),
            persona: "You are a creative problem-solver. Focus on generating \
                      innovative solutions, exploring alternatives, and thinking \
                      outside the box. Consider multiple perspectives."
                .to_string(),
            allowed_tools: vec!["write_file".to_string(), "web_search".to_string()],
            max_iterations: 10,
            can_delegate: false,
            ..Default::default()
        }
    }

    /// Pragmatic executor profile — focused on getting things done efficiently
    pub fn executor() -> Self {
        Self {
            name: "executor".to_string(),
            description: "Pragmatic executor focused on efficient task completion".to_string(),
            persona: "You are a pragmatic executor. Focus on completing tasks \
                      efficiently and correctly. Prioritize simplicity and \
                      practicality over perfection."
                .to_string(),
            allowed_tools: vec![
                "shell_exec".to_string(),
                "read_file".to_string(),
                "write_file".to_string(),
                "web_fetch".to_string(),
            ],
            max_iterations: 8,
            can_delegate: false,
            ..Default::default()
        }
    }

    /// Reviewer / quality-assurance profile — focused on verification and validation
    pub fn reviewer() -> Self {
        Self {
            name: "reviewer".to_string(),
            description: "Quality assurance reviewer focused on verification".to_string(),
            persona: "You are a meticulous reviewer. Focus on verifying correctness, \
                      identifying issues, and ensuring quality. Be critical and \
                      constructive. Check for errors, edge cases, and improvements."
                .to_string(),
            allowed_tools: vec!["read_file".to_string(), "web_fetch".to_string()],
            max_iterations: 10,
            can_delegate: false,
            ..Default::default()
        }
    }

    /// Supervisor profile — can delegate to sub-workers
    pub fn supervisor() -> Self {
        Self {
            name: "supervisor".to_string(),
            description: "Supervisor that decomposes tasks and coordinates sub-agents".to_string(),
            persona: "You are a supervisor agent. Your role is to decompose complex \
                      tasks into subtasks and coordinate sub-agents to complete them. \
                      Analyze the task, break it down, assign work, and aggregate results. \
                      \n\nFor each subtask, respond with:\n\
                      SUBTASK: <description>\n\
                      ROLE: <researcher|creative|executor|reviewer|supervisor>\n\
                      \nWhen all subtasks are complete, respond with:\n\
                      FINAL: <aggregated result>"
                .to_string(),
            allowed_tools: Vec::new(),
            max_iterations: 20,
            can_delegate: true,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Swarm configuration
// ---------------------------------------------------------------------------

/// Swarm topology — how workers are organized
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmTopology {
    /// Single coordinator delegates to workers (default)
    #[serde(alias = "flat")]
    Star,
    /// Peer-to-peer — workers communicate directly
    Mesh,
    /// Tree of supervisors — recursive decomposition
    Hierarchical,
    /// Combination of topologies based on task
    Hybrid,
}

#[allow(clippy::derivable_impls)]
impl Default for SwarmTopology {
    fn default() -> Self {
        Self::Star
    }
}

/// Swarm orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Swarm topology
    #[serde(default)]
    pub topology: SwarmTopology,

    /// Maximum recursion depth for hierarchical decomposition (default: 3)
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Maximum number of workers in the swarm (default: 100)
    #[serde(default = "default_max_workers", alias = "agent_count")]
    pub max_workers: usize,

    /// Worker profiles available for role assignment
    ///
    /// Accepts both `[[swarm.profiles]]` array-of-tables and `[swarm.profiles.name]`
    /// map syntax in TOML. The map syntax uses the key as the profile name and the
    /// value as the persona string (shorthand for quick configuration).
    ///
    /// # Examples (TOML)
    ///
    /// ```toml
    /// # Array-of-tables (full syntax)
    /// [[swarm.profiles]]
    /// name = "researcher"
    /// persona = "You are a research specialist..."
    ///
    /// # Map shorthand (name → persona)
    /// [swarm.profiles]
    /// coder = "You are a Rust expert..."
    /// reviewer = "You are a code reviewer..."
    /// ```
    #[serde(default, deserialize_with = "deserialize_profiles")]
    pub profiles: Vec<WorkerProfile>,

    /// Enable dynamic role assignment based on task analysis
    #[serde(default = "default_true")]
    pub dynamic_role_assignment: bool,

    /// Enable inter-agent communication
    #[serde(default)]
    pub enable_agent_communication: bool,

    /// Enable swarm health monitoring
    #[serde(default)]
    pub enable_health_monitoring: bool,
}

fn default_max_depth() -> usize {
    3
}

fn default_max_workers() -> usize {
    100
}

/// Custom deserializer for `profiles` that accepts both array-of-tables and map syntax.
///
/// In TOML:
/// - `[[swarm.profiles]]` → array of tables (standard)
/// - `[swarm.profiles.name]` → map with key as profile name, value as persona string
///
/// The map shorthand creates a `WorkerProfile` with the key as `name` and the
/// string value as `persona`. All other fields use defaults.
fn deserialize_profiles<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<WorkerProfile>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Try to deserialize as a map first (shorthand syntax)
    // If that fails, fall back to array-of-tables
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ProfilesOrMap {
        Array(Vec<WorkerProfile>),
        Map(std::collections::HashMap<String, String>),
    }

    match ProfilesOrMap::deserialize(deserializer) {
        Ok(ProfilesOrMap::Array(profiles)) => Ok(profiles),
        Ok(ProfilesOrMap::Map(map)) => {
            let profiles: Vec<WorkerProfile> = map
                .into_iter()
                .map(|(name, persona)| WorkerProfile {
                    name,
                    persona,
                    ..WorkerProfile::default()
                })
                .collect();
            Ok(profiles)
        }
        Err(e) => Err(e),
    }
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            topology: SwarmTopology::default(),
            max_depth: default_max_depth(),
            max_workers: default_max_workers(),
            profiles: vec![
                WorkerProfile::researcher(),
                WorkerProfile::creative(),
                WorkerProfile::executor(),
                WorkerProfile::reviewer(),
                WorkerProfile::supervisor(),
            ],
            dynamic_role_assignment: true,
            enable_agent_communication: false,
            enable_health_monitoring: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Swarm orchestrator
// ---------------------------------------------------------------------------

/// The swarm orchestrator manages recursive task decomposition and worker
/// coordination. It is the core of the self-provisioning sub-agents feature.
pub struct SwarmOrchestrator {
    /// Swarm configuration
    config: SwarmConfig,
    /// Current recursion depth (for enforcing max_depth)
    current_depth: usize,
    /// Total workers spawned (for enforcing max_workers)
    worker_count: usize,
    /// The LLM provider for this orchestrator instance
    llm: Option<Arc<dyn LLMProviderTrait>>,
    /// Multi-model manager (if using multiple providers)
    multi_llm: Option<MultiModelManager>,
    /// RavenFabric client for remote execution
    ravenfabric: Option<RavenFabricClient>,
    /// Policy engine for security (reserved for future tool execution)
    #[allow(dead_code)]
    policy_engine: PolicyEngine,
    /// Sandbox for execution
    sandbox: Sandbox,
    /// Audit log
    audit_log: AuditLog,
    /// Tool registry (reserved for future worker tool execution)
    #[allow(dead_code)]
    registry: ToolRegistry,
    /// Inter-agent message bus (shared across sub-orchestrators)
    message_bus: Option<Arc<RwLock<AgentMessageBus>>>,
    /// Swarm health monitor (active when enable_health_monitoring is true)
    /// Uses Arc<RwLock> for interior mutability since execute_with_profile takes &self
    health_monitor: Option<Arc<RwLock<SwarmHealthMonitor>>>,
}

impl SwarmOrchestrator {
    /// Create a new swarm orchestrator with the given configuration.
    pub fn new(
        config: SwarmConfig,
        llm: Option<Arc<dyn LLMProviderTrait>>,
        multi_llm: Option<MultiModelManager>,
        ravenfabric: Option<RavenFabricClient>,
    ) -> Self {
        let policy_engine = PolicyEngine::default_secure();
        let sandbox = Sandbox::default();
        let audit_log = AuditLog::new(format!("swarm-{}", std::process::id()));
        let registry = ToolRegistry::with_default_tools();

        // Create message bus if inter-agent communication is enabled
        let message_bus = if config.enable_agent_communication {
            Some(Arc::new(RwLock::new(AgentMessageBus::new(1000))))
        } else {
            None
        };

        // Create health monitor if health monitoring is enabled
        let health_monitor = if config.enable_health_monitoring {
            Some(Arc::new(RwLock::new(SwarmHealthMonitor::default())))
        } else {
            None
        };

        Self {
            config,
            current_depth: 0,
            worker_count: 0,
            llm,
            multi_llm,
            ravenfabric,
            policy_engine,
            sandbox,
            audit_log,
            registry,
            message_bus,
            health_monitor,
        }
    }

    /// Create a new swarm orchestrator with a shared message bus.
    ///
    /// This is used when spawning sub-orchestrators that should share
    /// the same communication channel as their parent.
    #[allow(dead_code)]
    pub fn new_with_bus(
        config: SwarmConfig,
        llm: Option<Arc<dyn LLMProviderTrait>>,
        multi_llm: Option<MultiModelManager>,
        ravenfabric: Option<RavenFabricClient>,
        message_bus: Option<Arc<RwLock<AgentMessageBus>>>,
    ) -> Self {
        let policy_engine = PolicyEngine::default_secure();
        let sandbox = Sandbox::default();
        let audit_log = AuditLog::new(format!("swarm-{}", std::process::id()));
        let registry = ToolRegistry::with_default_tools();

        // Create health monitor if health monitoring is enabled
        let health_monitor = if config.enable_health_monitoring {
            Some(Arc::new(RwLock::new(SwarmHealthMonitor::default())))
        } else {
            None
        };

        Self {
            config,
            current_depth: 0,
            worker_count: 0,
            llm,
            multi_llm,
            ravenfabric,
            policy_engine,
            sandbox,
            audit_log,
            registry,
            message_bus,
            health_monitor,
        }
    }

    /// Initialize the sandbox for this orchestrator.
    pub async fn init(&mut self) -> Result<()> {
        self.sandbox.init().await.map_err(|e| {
            RavenClawsError::CommandExecution(format!("Swarm sandbox init failed: {}", e))
        })?;
        Ok(())
    }

    /// Get the current worker count.
    #[allow(dead_code)]
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Get the current recursion depth.
    #[allow(dead_code)]
    pub fn current_depth(&self) -> usize {
        self.current_depth
    }

    /// Get swarm health metrics, if health monitoring is enabled.
    #[allow(dead_code)]
    pub fn health_metrics(&self) -> Option<SwarmMetrics> {
        self.health_monitor
            .as_ref()
            .and_then(|hm| hm.try_read().ok())
            .map(|hm| hm.metrics())
    }

    /// Get telemetry for all workers, if health monitoring is enabled.
    #[allow(dead_code)]
    pub fn worker_telemetry(&self) -> Option<Vec<WorkerTelemetry>> {
        self.health_monitor
            .as_ref()
            .and_then(|hm| hm.try_read().ok())
            .map(|hm| hm.all_worker_telemetry())
    }

    /// Orchestrate a task — decompose, assign, execute, and aggregate.
    ///
    /// This is the main entry point for swarm execution. It analyzes the task,
    /// determines required roles, spawns workers, and aggregates results.
    #[instrument(skip(self, task), fields(depth = self.current_depth, workers = self.worker_count))]
    pub async fn orchestrate(&mut self, task: &str) -> Result<String> {
        // Delegate to the boxed implementation to handle recursion safely.
        self.orchestrate_impl(task).await
    }

    /// Non-recursive entry point — calls the boxed recursive implementation.
    async fn orchestrate_impl(&mut self, task: &str) -> Result<String> {
        info!(
            depth = self.current_depth,
            max_depth = self.config.max_depth,
            "Orchestrating task"
        );

        // Log health status at start if monitoring is enabled
        if let Some(ref hm) = self.health_monitor {
            if let Ok(hm_guard) = hm.try_read() {
                info!(health = %hm_guard.format_status(), "Swarm health at start");
            }
        }

        // Check recursion depth
        if self.current_depth >= self.config.max_depth {
            warn!(
                depth = self.current_depth,
                max_depth = self.config.max_depth,
                "Max recursion depth reached, executing directly"
            );
            return self.execute_direct(task).await;
        }

        // Check worker limit
        if self.worker_count >= self.config.max_workers {
            warn!(
                workers = self.worker_count,
                max_workers = self.config.max_workers,
                "Max workers reached, executing directly"
            );
            return self.execute_direct(task).await;
        }

        // Analyze task and determine roles
        let roles = if self.config.dynamic_role_assignment {
            self.analyze_task_roles(task).await?
        } else {
            // Default: use supervisor + executor
            vec!["supervisor".to_string()]
        };

        info!(roles = ?roles, "Assigned roles for task");

        // If only one role and it's not supervisor, execute directly
        if roles.len() == 1 && roles[0] != "supervisor" {
            return self.execute_with_profile(task, &roles[0]).await;
        }

        // If supervisor role is present, do recursive decomposition
        if roles.contains(&"supervisor".to_string()) || roles.len() > 1 {
            return self.recursive_supervise_impl(task, &roles).await;
        }

        // Default: execute directly
        self.execute_direct(task).await
    }

    /// Analyze a task and determine which worker roles are needed.
    async fn analyze_task_roles(&self, task: &str) -> Result<Vec<String>> {
        // Use the LLM to analyze the task if available
        if let Some(ref llm) = self.llm {
            let analysis_prompt = format!(
                "Analyze this task and determine which roles are needed to complete it. \
                 Available roles: researcher, creative, executor, reviewer, supervisor. \
                 \n\nTask: {}\n\n\
                 Respond with a comma-separated list of roles needed, nothing else. \
                 Example: researcher, executor, reviewer",
                task
            );

            let messages = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a task analysis expert. Respond only with a comma-separated list of roles."
                        .to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: analysis_prompt,
                },
            ];

            match llm.chat(messages).await {
                Ok(response) => {
                    let content = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();

                    let roles: Vec<String> = content
                        .split(',')
                        .map(|r| r.trim().to_lowercase())
                        .filter(|r| {
                            matches!(
                                r.as_str(),
                                "researcher" | "creative" | "executor" | "reviewer" | "supervisor"
                            )
                        })
                        .collect();

                    if roles.is_empty() {
                        Ok(vec!["executor".to_string()])
                    } else {
                        Ok(roles)
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Task analysis failed, using default roles");
                    Ok(vec!["executor".to_string()])
                }
            }
        } else {
            // No LLM available, use default roles
            Ok(vec!["executor".to_string()])
        }
    }

    /// Execute a task directly with a specific worker profile.
    async fn execute_with_profile(&self, task: &str, role: &str) -> Result<String> {
        let profile = self
            .config
            .profiles
            .iter()
            .find(|p| p.name == role)
            .cloned()
            .unwrap_or_else(|| {
                if role == "supervisor" {
                    WorkerProfile::supervisor()
                } else {
                    WorkerProfile::executor()
                }
            });

        info!(role = %role, profile = %profile.name, "Executing task with profile");

        // Register worker with health monitor and record task start
        if let Some(ref hm) = self.health_monitor {
            if let Ok(mut hm_guard) = hm.try_write() {
                hm_guard.register_worker(role);
                hm_guard.task_started(role);
            }
        }

        let llm = self.llm.as_ref().ok_or_else(|| {
            RavenClawsError::CommandExecution("No LLM provider available for worker".to_string())
        })?;

        let mut memory = ConversationMemory::new(&profile.persona, profile.max_memory_messages);

        // Include inter-agent messages in the prompt if communication is enabled
        let enriched_task = if let Some(ref bus) = self.message_bus {
            if let Ok(bus_guard) = bus.try_read() {
                let msg_context = bus_guard.format_for_prompt(role, 20);
                format!("{}{}", task, msg_context)
            } else {
                task.to_string()
            }
        } else {
            task.to_string()
        };

        memory.add_user_message(&enriched_task);

        let messages = memory.history().to_vec();
        let response = llm.chat(messages).await.map_err(|e| {
            // Record failure in health monitor
            if let Some(ref hm) = self.health_monitor {
                if let Ok(mut hm_guard) = hm.try_write() {
                    hm_guard.task_failed(role);
                }
            }
            RavenClawsError::CommandExecution(format!("Worker {} failed: {}", role, e))
        })?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Record task completion in health monitor
        if let Some(ref hm) = self.health_monitor {
            if let Ok(mut hm_guard) = hm.try_write() {
                hm_guard.task_completed(role);
                hm_guard.heartbeat(role);
            }
        }

        // Broadcast result to other workers via message bus
        if let Some(ref bus) = self.message_bus {
            if let Ok(mut bus_guard) = bus.try_write() {
                bus_guard.send(
                    role,
                    "*",
                    MessageType::Result,
                    &format!(
                        "Completed task. Result ({} chars): {}",
                        content.len(),
                        &content[..content.len().min(500)]
                    ),
                    HashMap::new(),
                );
            }
            // Track message in health monitor
            if let Some(ref hm) = self.health_monitor {
                if let Ok(mut hm_guard) = hm.try_write() {
                    hm_guard.message_sent(role);
                }
            }
        }

        let _ = self.audit_log.append(
            AuditEventType::AgentFinish,
            &format!("worker-{}", role),
            &format!("Worker {} completed task", role),
            Some(serde_json::json!({
                "role": role,
                "task_length": task.len(),
                "response_length": content.len(),
            })),
        );

        Ok(content)
    }

    /// Execute a task directly without decomposition (leaf node).
    async fn execute_direct(&self, task: &str) -> Result<String> {
        self.execute_with_profile(task, "executor").await
    }

    /// Recursive supervision — decompose task, spawn sub-supervisors or workers.
    ///
    /// This is a thin wrapper that delegates to the boxed implementation
    /// to avoid Rust's recursive async fn limitation.
    #[allow(dead_code)]
    async fn recursive_supervise(&self, task: &str, roles: &[String]) -> Result<String> {
        let task = task.to_string();
        let roles = roles.to_vec();
        let this: &SwarmOrchestrator = self;
        Box::pin(async move { this.recursive_supervise_impl(&task, &roles).await }).await
    }

    /// Recursive supervision implementation (boxed to avoid infinite future size).
    async fn recursive_supervise_impl(&self, task: &str, roles: &[String]) -> Result<String> {
        let llm = self.llm.as_ref().ok_or_else(|| {
            RavenClawsError::CommandExecution(
                "No LLM provider available for supervisor".to_string(),
            )
        })?;

        // Register supervisor with health monitor
        if let Some(ref hm) = self.health_monitor {
            if let Ok(mut hm_guard) = hm.try_write() {
                hm_guard.register_worker("supervisor");
                hm_guard.task_started("supervisor");
            }
        }

        let supervisor_profile = WorkerProfile::supervisor();
        let mut memory = ConversationMemory::new(
            &supervisor_profile.persona,
            supervisor_profile.max_memory_messages,
        );

        let role_list = roles.join(", ");

        // Include inter-agent messages in the supervisor prompt if communication is enabled
        let msg_context = if let Some(ref bus) = self.message_bus {
            if let Ok(bus_guard) = bus.try_read() {
                bus_guard.format_for_prompt("supervisor", 20)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let supervise_prompt = format!(
            "Decompose this task into subtasks and assign each to the most appropriate role.\n\
             Available roles: {}\n\n\
             Task: {}\n\n\
             For each subtask, respond with:\n\
             SUBTASK: <description>\n\
             ROLE: <role>\n\n\
             When all subtasks are complete, respond with:\n\
             FINAL: <aggregated result>\n\
             {}",
            role_list, task, msg_context
        );

        memory.add_user_message(&supervise_prompt);

        let mut subtask_results: Vec<String> = Vec::new();
        let mut iteration = 0;
        let max_iterations = supervisor_profile.max_iterations;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                warn!("Supervisor reached max iterations");
                break;
            }

            let messages = memory.history().to_vec();
            let response = match llm.chat(messages).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(error = %e, "Supervisor LLM request failed");
                    continue;
                }
            };

            let content = response
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default();

            // Periodically check health status
            if iteration % 3 == 0 {
                if let Some(ref hm) = self.health_monitor {
                    if let Ok(hm_guard) = hm.try_read() {
                        let status = hm_guard.format_status();
                        info!(health = %status, "Swarm health check");
                        // Check for dead workers
                        let dead = hm_guard.dead_workers_for_replacement();
                        if !dead.is_empty() {
                            warn!(dead_workers = ?dead, "Dead workers detected");
                        }
                    }
                }
            }

            // Check for FINAL: completion
            if content.contains("FINAL:") {
                let final_response = content
                    .split("FINAL:")
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                info!(
                    iteration = iteration,
                    subtasks = subtask_results.len(),
                    "Supervisor completed"
                );

                // Record supervisor completion in health monitor
                if let Some(ref hm) = self.health_monitor {
                    if let Ok(mut hm_guard) = hm.try_write() {
                        hm_guard.task_completed("supervisor");
                        hm_guard.heartbeat("supervisor");
                    }
                }

                let _ = self.audit_log.append(
                    AuditEventType::AgentFinish,
                    "supervisor",
                    "Supervisor completed recursive decomposition",
                    Some(serde_json::json!({
                        "iterations": iteration,
                        "subtasks_completed": subtask_results.len(),
                        "depth": self.current_depth,
                    })),
                );

                if !subtask_results.is_empty() {
                    let aggregated = subtask_results.join("\n\n");
                    return Ok(format!(
                        "{}\n\n## Aggregated Results\n\n{}",
                        final_response, aggregated
                    ));
                }
                return Ok(final_response);
            }

            // Check for SUBTASK: decomposition
            if content.contains("SUBTASK:") {
                let subtask_block = content.split("SUBTASK:").nth(1).unwrap_or("");
                let subtask_lines: Vec<&str> = subtask_block.lines().take(4).collect();

                let subtask_desc = subtask_lines.first().unwrap_or(&"").trim();
                let role = subtask_lines
                    .iter()
                    .find(|l| l.starts_with("ROLE:"))
                    .and_then(|l| l.split(':').nth(1))
                    .unwrap_or("executor")
                    .trim()
                    .to_lowercase();

                if !subtask_desc.is_empty() {
                    info!(role = %role, subtask = %subtask_desc, "Delegating subtask");

                    // Broadcast coordination message before delegating
                    if let Some(ref bus) = self.message_bus {
                        if let Ok(mut bus_guard) = bus.try_write() {
                            bus_guard.send(
                                "supervisor",
                                &role,
                                MessageType::Coordination,
                                &format!("Delegating subtask: {}", subtask_desc),
                                HashMap::new(),
                            );
                        }
                    }

                    let result =
                        if role == "supervisor" && self.current_depth < self.config.max_depth {
                            // Recursive: spawn a sub-supervisor (boxed to avoid recursive async fn)
                            let config = self.config.clone();
                            let current_depth = self.current_depth + 1;
                            let worker_count = self.worker_count + 1;
                            let llm = self.llm.clone();
                            let multi_llm = self.multi_llm.clone();
                            let ravenfabric = self.ravenfabric.clone();
                            let subtask = subtask_desc.to_string();
                            let message_bus = self.message_bus.clone();
                            let health_monitor = self.health_monitor.clone();

                            Box::pin(async move {
                                let mut sub_orchestrator = SwarmOrchestrator {
                                    config,
                                    current_depth,
                                    worker_count,
                                    llm,
                                    multi_llm,
                                    ravenfabric,
                                    policy_engine: PolicyEngine::default_secure(),
                                    sandbox: Sandbox::default(),
                                    audit_log: AuditLog::new(format!(
                                        "sub-swarm-{}-{}",
                                        current_depth,
                                        std::process::id()
                                    )),
                                    registry: ToolRegistry::with_default_tools(),
                                    message_bus,
                                    health_monitor,
                                };

                                // Initialize sub-orchestrator sandbox
                                let _ = sub_orchestrator.init().await;
                                sub_orchestrator.orchestrate(&subtask).await
                            })
                            .await
                        } else {
                            // Execute with the assigned profile
                            self.execute_with_profile(subtask_desc, &role).await
                        };

                    match result {
                        Ok(result) => {
                            info!(
                                role = %role,
                                chars = result.len(),
                                "Subtask completed"
                            );
                            subtask_results.push(format!("[{}] {}", role, result));

                            memory.add_assistant_message(&format!(
                                "Delegated subtask to {}: {}",
                                role, subtask_desc
                            ));
                            memory.add_user_message(&format!("Result from {}: {}", role, result));
                        }
                        Err(e) => {
                            warn!(role = %role, error = %e, "Subtask failed");
                            // Record failure in health monitor
                            if let Some(ref hm) = self.health_monitor {
                                if let Ok(mut hm_guard) = hm.try_write() {
                                    hm_guard.task_failed(&role);
                                    hm_guard.record_error(&role);
                                }
                            }
                            memory.add_assistant_message(&format!(
                                "Subtask for {} failed: {}",
                                role, e
                            ));
                        }
                    }
                }
            } else {
                memory.add_assistant_message(&content);
            }
        }

        // Fallback: return aggregated results
        if !subtask_results.is_empty() {
            let aggregated = subtask_results.join("\n\n");
            info!(
                "Supervisor aggregated {} subtask results",
                subtask_results.len()
            );
            return Ok(aggregated);
        }

        Err(RavenClawsError::CommandExecution(
            "Supervisor completed without results".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_profile_default() {
        let profile = WorkerProfile::default();
        assert_eq!(profile.name, "default");
        assert!(profile.can_delegate);
        assert_eq!(profile.max_iterations, 10);
        assert_eq!(profile.max_memory_messages, 20);
    }

    #[test]
    fn test_worker_profile_researcher() {
        let profile = WorkerProfile::researcher();
        assert_eq!(profile.name, "researcher");
        assert!(!profile.can_delegate);
        assert!(profile.allowed_tools.contains(&"web_fetch".to_string()));
        assert!(profile.allowed_tools.contains(&"web_search".to_string()));
    }

    #[test]
    fn test_worker_profile_creative() {
        let profile = WorkerProfile::creative();
        assert_eq!(profile.name, "creative");
        assert!(!profile.can_delegate);
    }

    #[test]
    fn test_worker_profile_executor() {
        let profile = WorkerProfile::executor();
        assert_eq!(profile.name, "executor");
        assert!(!profile.can_delegate);
        assert!(profile.allowed_tools.contains(&"shell_exec".to_string()));
    }

    #[test]
    fn test_worker_profile_reviewer() {
        let profile = WorkerProfile::reviewer();
        assert_eq!(profile.name, "reviewer");
        assert!(!profile.can_delegate);
    }

    #[test]
    fn test_worker_profile_supervisor() {
        let profile = WorkerProfile::supervisor();
        assert_eq!(profile.name, "supervisor");
        assert!(profile.can_delegate);
        assert!(profile.persona.contains("SUBTASK:"));
        assert!(profile.persona.contains("FINAL:"));
    }

    #[test]
    fn test_swarm_config_default() {
        let config = SwarmConfig::default();
        assert_eq!(config.topology, SwarmTopology::Star);
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.max_workers, 100);
        assert!(config.dynamic_role_assignment);
        assert_eq!(config.profiles.len(), 5);
    }

    #[test]
    fn test_swarm_topology_serde() {
        let topologies = vec![
            SwarmTopology::Star,
            SwarmTopology::Mesh,
            SwarmTopology::Hierarchical,
            SwarmTopology::Hybrid,
        ];

        for t in &topologies {
            let json = serde_json::to_string(t).unwrap();
            let deserialized: SwarmTopology = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, deserialized);
        }
    }

    #[test]
    fn test_swarm_config_serde() {
        let config = SwarmConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: SwarmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.topology, deserialized.topology);
        assert_eq!(config.max_depth, deserialized.max_depth);
        assert_eq!(config.max_workers, deserialized.max_workers);
        assert_eq!(config.profiles.len(), deserialized.profiles.len());
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_tool_calls, 50);
        assert_eq!(limits.max_exec_secs, 300);
    }

    #[test]
    fn test_swarm_orchestrator_new() {
        let config = SwarmConfig::default();
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        assert_eq!(orchestrator.current_depth(), 0);
        assert_eq!(orchestrator.worker_count(), 0);
    }

    #[test]
    fn test_swarm_orchestrator_depth_limit() {
        let config = SwarmConfig {
            max_depth: 0, // No recursion allowed
            ..SwarmConfig::default()
        };
        let mut orchestrator = SwarmOrchestrator::new(config, None, None, None);
        orchestrator.current_depth = 0;

        // At depth 0 with max_depth 0, should hit the limit
        assert!(orchestrator.current_depth >= orchestrator.config.max_depth);
    }

    #[tokio::test]
    async fn test_analyze_task_roles_fallback() {
        let config = SwarmConfig::default();
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);

        // Without an LLM, should return default roles
        let result = orchestrator.analyze_task_roles("test task").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_worker_profile_custom() {
        let profile = WorkerProfile {
            name: "custom".to_string(),
            description: "Custom worker".to_string(),
            persona: "You are a custom worker.".to_string(),
            allowed_tools: vec!["read_file".to_string()],
            provider: Some("openai".to_string()),
            model: Some("gpt-4".to_string()),
            max_iterations: 5,
            max_memory_messages: 10,
            can_delegate: false,
            resource_limits: ResourceLimits {
                max_tool_calls: 10,
                max_exec_secs: 60,
            },
        };

        assert_eq!(profile.name, "custom");
        assert_eq!(profile.provider, Some("openai".to_string()));
        assert_eq!(profile.model, Some("gpt-4".to_string()));
        assert_eq!(profile.max_iterations, 5);
        assert_eq!(profile.resource_limits.max_tool_calls, 10);
    }

    #[test]
    fn test_swarm_config_custom_profiles() {
        let config = SwarmConfig {
            profiles: vec![WorkerProfile::researcher(), WorkerProfile::executor()],
            topology: SwarmTopology::Hierarchical,
            max_depth: 5,
            max_workers: 50,
            ..SwarmConfig::default()
        };

        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.topology, SwarmTopology::Hierarchical);
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.max_workers, 50);
    }

    // ── Inter-agent communication tests ─────────────────────────────────

    #[test]
    fn test_message_bus_new() {
        let bus = AgentMessageBus::new(100);
        assert!(bus.is_empty());
        assert_eq!(bus.len(), 0);
    }

    #[test]
    fn test_message_bus_send_and_receive() {
        let mut bus = AgentMessageBus::new(100);

        let id = bus.send(
            "researcher",
            "executor",
            MessageType::Information,
            "Found relevant data",
            HashMap::new(),
        );

        assert!(!id.is_empty());
        assert_eq!(bus.len(), 1);

        let msgs = bus.messages_for("executor");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Found relevant data");
        assert_eq!(msgs[0].sender, "researcher");
    }

    #[test]
    fn test_message_bus_broadcast() {
        let mut bus = AgentMessageBus::new(100);

        bus.send(
            "supervisor",
            "*",
            MessageType::Coordination,
            "All workers proceed",
            HashMap::new(),
        );

        // All roles should receive broadcast messages
        assert_eq!(bus.messages_for("researcher").len(), 1);
        assert_eq!(bus.messages_for("executor").len(), 1);
        assert_eq!(bus.messages_for("reviewer").len(), 1);
    }

    #[test]
    fn test_message_bus_filter_by_type() {
        let mut bus = AgentMessageBus::new(100);

        bus.send(
            "researcher",
            "*",
            MessageType::Information,
            "Data found",
            HashMap::new(),
        );
        bus.send(
            "executor",
            "supervisor",
            MessageType::Result,
            "Task done",
            HashMap::new(),
        );
        bus.send(
            "executor",
            "supervisor",
            MessageType::Error,
            "Failed",
            HashMap::new(),
        );

        let errors = bus.messages_of_type(&MessageType::Error);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].content, "Failed");

        let results = bus.messages_of_type(&MessageType::Result);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_message_bus_max_messages() {
        let mut bus = AgentMessageBus::new(5); // Only keep 5 messages

        for i in 0..10 {
            bus.send(
                "worker",
                "*",
                MessageType::Generic,
                &format!("Message {}", i),
                HashMap::new(),
            );
        }

        // Should only have the last 5 messages
        assert_eq!(bus.len(), 5);
        let all = bus.all_messages();
        assert_eq!(all[0].content, "Message 5");
        assert_eq!(all[4].content, "Message 9");
    }

    #[test]
    fn test_message_bus_format_for_prompt() {
        let mut bus = AgentMessageBus::new(100);

        bus.send(
            "researcher",
            "*",
            MessageType::Information,
            "Found key insight",
            HashMap::new(),
        );
        bus.send(
            "executor",
            "supervisor",
            MessageType::Result,
            "Implementation complete",
            HashMap::new(),
        );

        let prompt = bus.format_for_prompt("supervisor", 10);
        assert!(prompt.contains("Inter-Agent Messages"));
        assert!(prompt.contains("researcher"));
        assert!(prompt.contains("executor"));
        assert!(prompt.contains("Found key insight"));
    }

    #[test]
    fn test_message_bus_empty_format() {
        let bus = AgentMessageBus::new(100);
        let prompt = bus.format_for_prompt("supervisor", 10);
        assert!(prompt.is_empty());
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(format!("{}", MessageType::Information), "information");
        assert_eq!(format!("{}", MessageType::Question), "question");
        assert_eq!(format!("{}", MessageType::Result), "result");
        assert_eq!(format!("{}", MessageType::Error), "error");
        assert_eq!(format!("{}", MessageType::Coordination), "coordination");
        assert_eq!(format!("{}", MessageType::Generic), "generic");
    }

    #[test]
    fn test_message_bus_messages_from() {
        let mut bus = AgentMessageBus::new(100);

        bus.send(
            "researcher",
            "*",
            MessageType::Information,
            "Data A",
            HashMap::new(),
        );
        bus.send(
            "researcher",
            "executor",
            MessageType::Information,
            "Data B",
            HashMap::new(),
        );
        bus.send(
            "executor",
            "supervisor",
            MessageType::Result,
            "Done",
            HashMap::new(),
        );

        let from_researcher = bus.messages_from("researcher");
        assert_eq!(from_researcher.len(), 2);

        let from_executor = bus.messages_from("executor");
        assert_eq!(from_executor.len(), 1);
    }

    #[test]
    fn test_orchestrator_new_with_communication() {
        let config = SwarmConfig {
            enable_agent_communication: true,
            ..SwarmConfig::default()
        };
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        assert!(orchestrator.message_bus.is_some());
    }

    #[test]
    fn test_orchestrator_new_without_communication() {
        let config = SwarmConfig {
            enable_agent_communication: false,
            ..SwarmConfig::default()
        };
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        assert!(orchestrator.message_bus.is_none());
    }

    #[test]
    fn test_swarm_config_communication_default() {
        let config = SwarmConfig::default();
        assert!(!config.enable_agent_communication); // Default: disabled
        assert!(!config.enable_health_monitoring); // Default: disabled
    }

    // ── Health monitoring tests ─────────────────────────────────────────

    #[test]
    fn test_health_status_display() {
        assert_eq!(format!("{}", WorkerHealthStatus::Healthy), "healthy");
        assert_eq!(format!("{}", WorkerHealthStatus::Degraded), "degraded");
        assert_eq!(format!("{}", WorkerHealthStatus::Unhealthy), "unhealthy");
        assert_eq!(format!("{}", WorkerHealthStatus::Dead), "dead");
    }

    #[test]
    fn test_health_monitor_default() {
        let hm = SwarmHealthMonitor::default();
        assert_eq!(hm.heartbeat_interval_secs, 5);
        assert_eq!(hm.max_missed_beats, 3);
        assert_eq!(hm.replacement_timeout_secs, 30);
        assert_eq!(hm.worker_count(), 0);
    }

    #[test]
    fn test_health_monitor_register_worker() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("researcher");
        assert_eq!(hm.worker_count(), 1);

        let telemetry = hm.worker_telemetry("researcher");
        assert!(telemetry.is_some());
        assert_eq!(telemetry.unwrap().role, "researcher");
    }

    #[test]
    fn test_health_monitor_heartbeat() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        hm.heartbeat("executor");

        let telemetry = hm.worker_telemetry("executor").unwrap();
        assert_eq!(telemetry.status, WorkerHealthStatus::Healthy);
    }

    #[test]
    fn test_health_monitor_task_lifecycle() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        hm.task_started("executor");
        hm.task_completed("executor");

        let telemetry = hm.worker_telemetry("executor").unwrap();
        assert_eq!(telemetry.tasks_completed, 1);
        assert_eq!(telemetry.tasks_failed, 0);
    }

    #[test]
    fn test_health_monitor_task_failure() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        hm.task_started("executor");
        hm.task_failed("executor");

        let telemetry = hm.worker_telemetry("executor").unwrap();
        assert_eq!(telemetry.tasks_completed, 0);
        assert_eq!(telemetry.tasks_failed, 1);
        assert_eq!(telemetry.error_count, 1);
    }

    #[test]
    fn test_health_monitor_metrics_empty() {
        let hm = SwarmHealthMonitor::default();
        let metrics = hm.metrics();
        assert_eq!(metrics.total_workers, 0);
        assert_eq!(metrics.healthy_workers, 0);
        assert_eq!(metrics.total_tasks_completed, 0);
        assert_eq!(metrics.task_throughput, 0.0);
    }

    #[test]
    fn test_health_monitor_metrics_with_workers() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("researcher");
        hm.register_worker("executor");
        hm.task_started("executor");
        hm.task_completed("executor");
        hm.task_started("researcher");
        hm.task_completed("researcher");

        let metrics = hm.metrics();
        assert_eq!(metrics.total_workers, 2);
        assert_eq!(metrics.healthy_workers, 2);
        assert_eq!(metrics.total_tasks_completed, 2);
    }

    #[test]
    fn test_health_monitor_dead_worker_detection() {
        let mut hm = SwarmHealthMonitor {
            heartbeat_interval_secs: 1,
            max_missed_beats: 1,
            replacement_timeout_secs: 0, // Immediate replacement
            ..SwarmHealthMonitor::default()
        };

        hm.register_worker("executor");
        // Set last heartbeat far in the past by manipulating directly
        if let Some(hb) = hm.heartbeats.get_mut("executor") {
            hb.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(10);
        }

        let dead = hm.check_health();
        assert!(!dead.is_empty());
        assert_eq!(dead[0], "executor");
    }

    #[test]
    fn test_health_monitor_degraded_detection() {
        let mut hm = SwarmHealthMonitor {
            heartbeat_interval_secs: 1,
            max_missed_beats: 3,
            ..SwarmHealthMonitor::default()
        };

        hm.register_worker("executor");
        // Set last heartbeat to 3 seconds ago (between 2x interval and max_missed*interval)
        if let Some(hb) = hm.heartbeats.get_mut("executor") {
            hb.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(3);
        }

        let _dead = hm.check_health();
        let telemetry = hm.worker_telemetry("executor").unwrap();
        assert_eq!(telemetry.status, WorkerHealthStatus::Degraded);
    }

    #[test]
    fn test_health_monitor_message_tracking() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("researcher");
        hm.message_sent("researcher");
        hm.message_sent("researcher");
        hm.message_received("researcher");

        let telemetry = hm.worker_telemetry("researcher").unwrap();
        assert_eq!(telemetry.messages_sent, 2);
        assert_eq!(telemetry.messages_received, 1);
    }

    #[test]
    fn test_health_monitor_error_tracking() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        hm.record_error("executor");
        hm.record_error("executor");
        hm.record_error("executor");

        let telemetry = hm.worker_telemetry("executor").unwrap();
        assert_eq!(telemetry.error_count, 3);
    }

    #[test]
    fn test_health_monitor_format_status() {
        let hm = SwarmHealthMonitor::default();
        let status = hm.format_status();
        assert!(status.contains("Swarm Health:"));
        assert!(status.contains("healthy"));
    }

    #[test]
    fn test_health_monitor_all_worker_telemetry() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("researcher");
        hm.register_worker("executor");
        hm.register_worker("reviewer");

        let all = hm.all_worker_telemetry();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_health_monitor_remove_worker() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        assert_eq!(hm.worker_count(), 1);
        hm.remove_worker("executor");
        assert_eq!(hm.worker_count(), 0);
    }

    #[test]
    fn test_orchestrator_new_with_health_monitoring() {
        let config = SwarmConfig {
            enable_health_monitoring: true,
            ..SwarmConfig::default()
        };
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        assert!(orchestrator.health_monitor.is_some());
    }

    #[test]
    fn test_orchestrator_new_without_health_monitoring() {
        let config = SwarmConfig {
            enable_health_monitoring: false,
            ..SwarmConfig::default()
        };
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        assert!(orchestrator.health_monitor.is_none());
    }

    #[test]
    fn test_health_metrics_accessor() {
        let config = SwarmConfig {
            enable_health_monitoring: true,
            ..SwarmConfig::default()
        };
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        let metrics = orchestrator.health_metrics();
        assert!(metrics.is_some());
        assert_eq!(metrics.unwrap().total_workers, 0);
    }

    #[test]
    fn test_worker_telemetry_accessor() {
        let config = SwarmConfig {
            enable_health_monitoring: true,
            ..SwarmConfig::default()
        };
        let orchestrator = SwarmOrchestrator::new(config, None, None, None);
        let telemetry = orchestrator.worker_telemetry();
        assert!(telemetry.is_some());
        assert!(telemetry.unwrap().is_empty());
    }

    #[test]
    fn test_health_monitor_new_custom() {
        let hm = SwarmHealthMonitor::new(10, 5, 60);
        assert_eq!(hm.heartbeat_interval_secs, 10);
        assert_eq!(hm.max_missed_beats, 5);
        assert_eq!(hm.replacement_timeout_secs, 60);
    }

    #[test]
    fn test_health_monitor_dead_workers_for_replacement() {
        let mut hm = SwarmHealthMonitor {
            heartbeat_interval_secs: 1,
            max_missed_beats: 1,
            replacement_timeout_secs: 0,
            ..SwarmHealthMonitor::default()
        };

        hm.register_worker("executor");
        // Set last heartbeat far in the past
        if let Some(hb) = hm.heartbeats.get_mut("executor") {
            hb.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(30);
            hb.status = WorkerHealthStatus::Dead;
        }

        let candidates = hm.dead_workers_for_replacement();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "executor");
    }

    #[test]
    fn test_health_monitor_metrics_error_rate() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        hm.task_started("executor");
        hm.task_completed("executor");
        hm.task_started("executor");
        hm.task_failed("executor");

        let metrics = hm.metrics();
        assert_eq!(metrics.total_tasks_completed, 1);
        assert_eq!(metrics.total_tasks_failed, 1);
        assert!(metrics.error_rate > 0.0);
    }

    #[test]
    fn test_health_monitor_metrics_utilization() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("busy_worker");
        hm.register_worker("idle_worker");

        // Mark one worker as busy
        if let Some(hb) = hm.heartbeats.get_mut("busy_worker") {
            hb.is_busy = true;
        }

        let metrics = hm.metrics();
        assert_eq!(metrics.total_workers, 2);
        assert!((metrics.worker_utilization - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_health_monitor_iteration_tracking() {
        let mut hm = SwarmHealthMonitor::default();
        hm.register_worker("executor");
        hm.task_started("executor");
        hm.task_completed("executor");
        hm.task_started("executor");
        hm.task_completed("executor");
        hm.task_started("executor");

        let telemetry = hm.worker_telemetry("executor").unwrap();
        assert_eq!(telemetry.iteration, 3);
        assert_eq!(telemetry.tasks_completed, 2);
    }
}
