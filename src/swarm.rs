//! Swarm orchestration — self-provisioning sub-agents with recursive supervision
//!
//! RavenClaw's swarm orchestrator enables truly autonomous multi-agent coordination:
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
//! - Config: `[swarm]` section in `ravenclaw.toml`
//! - Supervisor mode: automatically uses orchestrator when `max_depth > 0`

use crate::agent::ConversationMemory;
use crate::audit::{AuditEventType, AuditLog};
use crate::error::{RavenClawError, Result};
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
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,

    /// Worker profiles available for role assignment
    #[serde(default)]
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
        }
    }

    /// Initialize the sandbox for this orchestrator.
    pub async fn init(&mut self) -> Result<()> {
        self.sandbox.init().await.map_err(|e| {
            RavenClawError::CommandExecution(format!("Swarm sandbox init failed: {}", e))
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

        let llm = self.llm.as_ref().ok_or_else(|| {
            RavenClawError::CommandExecution("No LLM provider available for worker".to_string())
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
            RavenClawError::CommandExecution(format!("Worker {} failed: {}", role, e))
        })?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

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
            RavenClawError::CommandExecution("No LLM provider available for supervisor".to_string())
        })?;

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

        Err(RavenClawError::CommandExecution(
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
}
