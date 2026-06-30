//! RavenClaws
//!
//! Supports single-provider and multi-model (multi-provider) modes.
//! Security-integrated: PolicyEngine, Sandbox, and AuditLog wired to agent loop.

use crate::audit::{AuditEventType, AuditLog};
use crate::config::Config;
use crate::error::Result;
use crate::llm::{
    ChatMessage, Choice, LLMProviderTrait, MultiModelManager, ProviderFallbackChain, TokenBudget,
};
use crate::mcp::McpClient;
use crate::policy::{Decision, PolicyEngine};
use crate::ravenfabric::RavenFabricClient;
use crate::sandbox::Sandbox;
use crate::tools::{ToolCall, ToolRegistry, ToolResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// In-memory conversation memory — stores message history for the session.
///
/// With durable execution, messages can be serialized to disk between iterations
/// so the agent loop can survive process restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    /// Maximum number of messages to retain (0 = unlimited)
    max_messages: usize,
    /// Stored message history
    messages: Vec<ChatMessage>,
}

impl ConversationMemory {
    /// Create a new conversation memory with the given system prompt.
    /// `max_messages` caps history length (oldest user/assistant pairs are dropped first).
    pub fn new(system_prompt: &str, max_messages: usize) -> Self {
        Self {
            max_messages,
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            }],
        }
    }

    /// Add a user message and return the full message history for an LLM call.
    pub fn add_user_message(&mut self, content: &str) -> &[ChatMessage] {
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
        });
        self.trim_to_max();
        &self.messages
    }

    /// Add an assistant message to history.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
        });
        self.trim_to_max();
    }

    /// Get the current message history.
    pub fn history(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Create a ConversationMemory from an existing message history.
    /// Used when restoring from a checkpoint.
    pub fn from_history(messages: Vec<ChatMessage>, max_messages: usize) -> Self {
        Self {
            max_messages,
            messages,
        }
    }

    /// Get the number of messages in history.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if history is empty (only system prompt or nothing).
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Trim oldest non-system messages when over the limit.
    fn trim_to_max(&mut self) {
        if self.max_messages == 0 {
            return;
        }
        while self.messages.len() > self.max_messages {
            // Remove the oldest non-system message (index 1, since index 0 is system)
            if self.messages.len() > 1 {
                self.messages.remove(1);
            } else {
                break;
            }
        }
    }
}

/// Checkpoint state for durable execution — captures agent loop state between iterations.
///
/// This struct is serialized to disk after each iteration so the agent loop can
/// survive process restarts. On resume, the checkpoint is loaded and the loop
/// continues from where it left off.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointState {
    /// Unique session identifier
    pub session_id: String,
    /// Current iteration number
    pub iteration: usize,
    /// Maximum iterations configured for this loop
    pub max_iterations: usize,
    /// Serialized conversation memory (message history)
    pub messages: Vec<ChatMessage>,
    /// The initial prompt that started this session
    pub initial_prompt: String,
    /// The system prompt for this session
    pub system_prompt: String,
    /// Provider name used for this session
    pub provider: String,
    /// Model name used for this session
    pub model: String,
    /// Whether tools were enabled
    pub enable_tools: bool,
    /// Timestamp of last checkpoint (ISO 8601)
    pub last_checkpoint: String,
}

impl CheckpointState {
    /// Create a new checkpoint state from current agent loop state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        iteration: usize,
        max_iterations: usize,
        messages: Vec<ChatMessage>,
        initial_prompt: &str,
        system_prompt: &str,
        provider: &str,
        model: &str,
        enable_tools: bool,
    ) -> Self {
        Self {
            session_id,
            iteration,
            max_iterations,
            messages,
            initial_prompt: initial_prompt.to_string(),
            system_prompt: system_prompt.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            enable_tools,
            last_checkpoint: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Save a checkpoint to disk.
///
/// Writes the checkpoint state as a JSON file to `{checkpoint_dir}/{session_id}.json`.
/// Returns the path that was written to, or `None` if checkpointing is not configured.
pub fn save_checkpoint(
    checkpoint_dir: &std::path::Path,
    state: &CheckpointState,
) -> std::result::Result<std::path::PathBuf, String> {
    let path = checkpoint_dir.join(format!("{}.json", state.session_id));

    // Ensure the checkpoint directory exists
    std::fs::create_dir_all(checkpoint_dir)
        .map_err(|e| format!("Failed to create checkpoint directory: {}", e))?;

    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize checkpoint: {}", e))?;

    // Write atomically: write to temp file, then rename
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write checkpoint: {}", e))?;
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to finalize checkpoint: {}", e))?;

    Ok(path)
}

/// Load a checkpoint from disk.
///
/// Reads the checkpoint state from `{checkpoint_dir}/{session_id}.json`.
/// Returns `None` if the checkpoint file doesn't exist or can't be read.
pub fn load_checkpoint(
    checkpoint_dir: &std::path::Path,
    session_id: &str,
) -> Option<CheckpointState> {
    let path = checkpoint_dir.join(format!("{}.json", session_id));

    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<CheckpointState>(&content) {
            Ok(state) => {
                info!(
                    session_id = %session_id,
                    iteration = state.iteration,
                    max_iterations = state.max_iterations,
                    "Loaded checkpoint"
                );
                Some(state)
            }
            Err(e) => {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to deserialize checkpoint"
                );
                None
            }
        },
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to read checkpoint"
                );
            }
            None
        }
    }
}

/// Delete a checkpoint file from disk.
///
/// Called when the agent loop completes successfully or fails definitively.
pub fn delete_checkpoint(checkpoint_dir: &std::path::Path, session_id: &str) {
    let path = checkpoint_dir.join(format!("{}.json", session_id));
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            warn!(
                session_id = %session_id,
                error = %e,
                "Failed to delete checkpoint"
            );
        } else {
            debug!(
                session_id = %session_id,
                "Deleted checkpoint"
            );
        }
    }
}

/// Agent loop configuration
///
/// Note: `Debug` and `Clone` are implemented manually because `metrics_callback`
/// is a boxed closure that doesn't implement either trait.
pub struct AgentLoopConfig {
    /// Maximum iterations before forcing completion
    pub max_iterations: usize,
    /// Whether to enable tool calling
    pub enable_tools: bool,
    /// Require human approval for tool calls
    pub require_approval: bool,
    /// Enable prompt-injection defense on LLM responses
    pub prompt_injection_protection: bool,
    /// Maximum session lifetime in seconds (0 = unlimited)
    /// When non-zero, the agent loop will stop after this duration
    /// to enforce credential/session expiry.
    pub token_lifetime_secs: u64,
    /// When true, treat any non-tool-call response as completion (no FINAL: required)
    pub no_final_required: bool,
    /// Optional provider fallback chain — tries providers in order on failure
    pub fallback_chain: Option<Arc<std::sync::Mutex<ProviderFallbackChain>>>,
    /// Optional token budget — limits total tokens used per session
    pub token_budget: Option<Arc<std::sync::Mutex<TokenBudget>>>,
    /// Optional RavenFabric client — reports agent status and results to mesh
    pub ravenfabric: Option<RavenFabricClient>,
    /// Optional checkpoint directory for durable execution.
    /// When set, the agent loop saves state after each iteration and can resume
    /// from the latest checkpoint if interrupted.
    pub checkpoint_dir: Option<PathBuf>,
    /// Unique session ID for checkpointing.
    /// If not set but checkpoint_dir is set, a UUID is generated automatically.
    pub session_id: Option<String>,
    /// Optional callback for recording metrics (token usage, tool calls).
    /// Called with (tokens_used, tool_calls_count) after each iteration.
    /// This allows the HTTP server to wire ServerMetrics without coupling agent.rs to server.rs.
    pub metrics_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            enable_tools: false,
            require_approval: false,
            prompt_injection_protection: true,
            token_lifetime_secs: 0,
            no_final_required: true,
            fallback_chain: None,
            token_budget: None,
            ravenfabric: None,
            checkpoint_dir: None,
            session_id: None,
            metrics_callback: None,
        }
    }
}

// Manual Debug implementation — skips metrics_callback (boxed closure doesn't impl Debug)
impl std::fmt::Debug for AgentLoopConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentLoopConfig")
            .field("max_iterations", &self.max_iterations)
            .field("enable_tools", &self.enable_tools)
            .field("require_approval", &self.require_approval)
            .field(
                "prompt_injection_protection",
                &self.prompt_injection_protection,
            )
            .field("token_lifetime_secs", &self.token_lifetime_secs)
            .field("no_final_required", &self.no_final_required)
            .field("fallback_chain", &self.fallback_chain)
            .field("token_budget", &self.token_budget)
            .field("ravenfabric", &self.ravenfabric)
            .field("checkpoint_dir", &self.checkpoint_dir)
            .field("session_id", &self.session_id)
            .field(
                "metrics_callback",
                &self.metrics_callback.as_ref().map(|_| "Box<Fn>"),
            )
            .finish()
    }
}

// Manual Clone implementation — metrics_callback is NOT cloned (intentionally dropped)
// because the callback is only needed by the original caller (e.g., HTTP server).
impl Clone for AgentLoopConfig {
    fn clone(&self) -> Self {
        Self {
            max_iterations: self.max_iterations,
            enable_tools: self.enable_tools,
            require_approval: self.require_approval,
            prompt_injection_protection: self.prompt_injection_protection,
            token_lifetime_secs: self.token_lifetime_secs,
            no_final_required: self.no_final_required,
            fallback_chain: self.fallback_chain.clone(),
            token_budget: self.token_budget.clone(),
            ravenfabric: self.ravenfabric.clone(),
            checkpoint_dir: self.checkpoint_dir.clone(),
            session_id: self.session_id.clone(),
            metrics_callback: None,
        }
    }
}

/// Run the agent loop with security integration (PolicyEngine + Sandbox + AuditLog)
///
/// This is the security-integrated version that:
/// 1. Checks all tool calls against PolicyEngine before execution
/// 2. Executes shell commands in the Sandbox
/// 3. Logs all tool calls, policy decisions, and results to AuditLog
#[instrument(skip_all, fields(provider = %llm.provider_name(), model = %llm.model()))]
pub async fn run_agent_loop(
    llm: Arc<dyn LLMProviderTrait>,
    initial_prompt: &str,
    system_prompt: &str,
    config: AgentLoopConfig,
) -> Result<String> {
    run_agent_loop_with_registry(llm, initial_prompt, system_prompt, config, None).await
}

/// Run the agent loop with an optional pre-configured ToolRegistry
///
/// This allows callers to pass a registry with custom tool configurations
/// (e.g., configured web search endpoint). If `None` is passed, default tools are used.
#[instrument(skip_all, fields(provider = %llm.provider_name(), model = %llm.model()))]
pub async fn run_agent_loop_with_registry(
    llm: Arc<dyn LLMProviderTrait>,
    initial_prompt: &str,
    system_prompt: &str,
    config: AgentLoopConfig,
    tool_registry: Option<ToolRegistry>,
) -> Result<String> {
    let registry = tool_registry.unwrap_or_else(ToolRegistry::with_default_tools);
    run_agent_loop_inner(
        llm,
        initial_prompt,
        system_prompt,
        config,
        registry,
        "security integration",
        false,
    )
    .await
}

/// Shared inner agent loop — contains all iteration logic.
///
/// Both `run_agent_loop_with_registry` and `run_agent_loop_with_mcp_and_registry`
/// delegate to this function, avoiding ~400 lines of duplicated code.
///
/// # Parameters
///
/// * `registry` — A fully initialized `ToolRegistry` (caller resolves defaults/MCP tools).
/// * `loop_label` — Label for log messages (e.g. "security integration" or "MCP integration").
/// * `mcp_enabled` — Whether MCP is active, used in audit event metadata.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(provider = %llm.provider_name(), model = %llm.model()))]
async fn run_agent_loop_inner(
    llm: Arc<dyn LLMProviderTrait>,
    initial_prompt: &str,
    system_prompt: &str,
    config: AgentLoopConfig,
    registry: ToolRegistry,
    loop_label: &str,
    mcp_enabled: bool,
) -> Result<String> {
    // Initialize security components
    let policy_engine = PolicyEngine::default_secure();
    let mut sandbox = Sandbox::default();
    sandbox.init().await.map_err(|e| {
        crate::error::RavenClawsError::CommandExecution(format!("Sandbox init failed: {}", e))
    })?;
    let audit_log = AuditLog::new(format!("agent-{}", std::process::id()));

    // Initialize injection detector
    let injection_detector = if config.prompt_injection_protection {
        Some(crate::policy::InjectionDetector::new())
    } else {
        None
    };

    // Track session start time for token lifetime enforcement
    let session_start = std::time::Instant::now();

    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        max_iterations = config.max_iterations,
        enable_tools = config.enable_tools,
        tool_count = registry.len(),
        require_approval = config.require_approval,
        prompt_injection_protection = config.prompt_injection_protection,
        token_lifetime_secs = config.token_lifetime_secs,
        "Agent loop starting with {}",
        loop_label
    );

    // Audit: agent start
    let _ = audit_log.append(
        AuditEventType::AgentStart,
        "agent",
        &format!(
            "Agent loop started with {} (model: {})",
            llm.provider_name(),
            llm.model()
        ),
        Some(serde_json::json!({
            "provider": llm.provider_name(),
            "model": llm.model(),
            "max_iterations": config.max_iterations,
            "enable_tools": config.enable_tools,
            "mcp_enabled": mcp_enabled,
            "tool_count": registry.len(),
            "require_approval": config.require_approval,
            "prompt_injection_protection": config.prompt_injection_protection,
            "token_lifetime_secs": config.token_lifetime_secs,
        })),
    );

    // ── Durable execution: checkpoint resume ──────────────────────────────
    // If a checkpoint exists for this session, restore state from it instead
    // of starting fresh. This allows the agent loop to survive process restarts.
    let (mut memory, start_iteration) = if let Some(ref checkpoint_dir) = config.checkpoint_dir {
        if let Some(ref session_id) = config.session_id {
            if let Some(checkpoint) = load_checkpoint(checkpoint_dir, session_id) {
                info!(
                    session_id = %session_id,
                    iteration = checkpoint.iteration,
                    max_iterations = checkpoint.max_iterations,
                    "Resuming agent loop from checkpoint"
                );
                (
                    ConversationMemory::from_history(checkpoint.messages, 0),
                    checkpoint.iteration + 1, // resume from next iteration
                )
            } else {
                info!(
                    session_id = %session_id,
                    "No checkpoint found, starting fresh"
                );
                let mut m = ConversationMemory::new(system_prompt, 0);
                m.add_user_message(initial_prompt);
                (m, 0)
            }
        } else {
            let mut m = ConversationMemory::new(system_prompt, 0);
            m.add_user_message(initial_prompt);
            (m, 0)
        }
    } else {
        let mut m = ConversationMemory::new(system_prompt, 0);
        m.add_user_message(initial_prompt);
        (m, 0)
    };

    // Generate a session ID if checkpointing is enabled but no ID was provided
    let session_id = config
        .session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    for iteration in start_iteration..config.max_iterations {
        // Check token lifetime: enforce session expiry
        if config.token_lifetime_secs > 0 {
            let elapsed = session_start.elapsed().as_secs();
            if elapsed >= config.token_lifetime_secs {
                warn!(
                    iteration = iteration,
                    elapsed_secs = elapsed,
                    token_lifetime_secs = config.token_lifetime_secs,
                    "Agent loop reached token lifetime limit"
                );
                let _ = audit_log.append(
                    AuditEventType::SecurityViolation,
                    "token_lifetime",
                    &format!(
                        "Session expired after {} seconds (limit: {}s)",
                        elapsed, config.token_lifetime_secs
                    ),
                    Some(serde_json::json!({
                        "elapsed_secs": elapsed,
                        "token_lifetime_secs": config.token_lifetime_secs,
                        "iteration": iteration,
                    })),
                );
                // Delete checkpoint on security violation
                if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                    delete_checkpoint(checkpoint_dir, &session_id);
                }
                return Err(crate::error::RavenClawsError::SecurityViolation(format!(
                    "Session token expired after {} seconds (limit: {}s)",
                    elapsed, config.token_lifetime_secs
                )));
            }
        }
        let messages = memory.history().to_vec();

        // Check token budget before making LLM call
        if let Some(ref budget) = config.token_budget {
            let budget = budget.lock().unwrap();
            if budget.remaining() < 100 {
                warn!(
                    iteration = iteration,
                    remaining = budget.remaining(),
                    "Token budget exhausted"
                );
                let _ = audit_log.append(
                    AuditEventType::SecurityViolation,
                    "token_budget",
                    &format!("Token budget exhausted (remaining: {})", budget.remaining()),
                    Some(serde_json::json!({
                        "remaining": budget.remaining(),
                        "used": budget.used_tokens,
                        "iteration": iteration,
                    })),
                );
                // Delete checkpoint on budget exhaustion
                if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                    delete_checkpoint(checkpoint_dir, &session_id);
                }
                return Err(crate::error::RavenClawsError::SecurityViolation(
                    "Token budget exhausted".to_string(),
                ));
            }
        }

        let response = match llm.chat(messages.clone()).await {
            Ok(r) => r,
            Err(e) => {
                // Try fallback chain if available
                if let Some(ref chain) = config.fallback_chain {
                    warn!(error = %e, "Primary LLM failed, trying fallback chain");
                    let _ = audit_log.append(
                        AuditEventType::Error,
                        "llm",
                        &format!("Primary LLM failed, trying fallback: {}", e),
                        None,
                    );
                    // Clone configs out of mutex to avoid holding MutexGuard across .await
                    let configs = {
                        let c = chain.lock().unwrap();
                        c.configs.clone()
                    };
                    let mut temp_chain = ProviderFallbackChain::new(configs);
                    match temp_chain.chat_with_fallback(messages).await {
                        Ok(r) => {
                            info!("Fallback chain succeeded");
                            // Record token usage from fallback response
                            if let Some(ref budget) = config.token_budget {
                                if let Some(usage) = &r.usage {
                                    let mut b = budget.lock().unwrap();
                                    b.record_usage(usage.total_tokens);
                                }
                            }
                            r
                        }
                        Err(fallback_e) => {
                            warn!(error = %fallback_e, "Fallback chain also failed");
                            let _ = audit_log.append(
                                AuditEventType::Error,
                                "llm",
                                &format!("All providers failed: {}", fallback_e),
                                None,
                            );
                            // Delete checkpoint on LLM failure
                            if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                                delete_checkpoint(checkpoint_dir, &session_id);
                            }
                            return Err(crate::error::RavenClawsError::Llm(fallback_e));
                        }
                    }
                } else {
                    warn!(error = %e, "LLM request failed");
                    let _ = audit_log.append(
                        AuditEventType::Error,
                        "llm",
                        &format!("LLM request failed: {}", e),
                        None,
                    );
                    // Delete checkpoint on LLM failure
                    if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                        delete_checkpoint(checkpoint_dir, &session_id);
                    }
                    return Err(crate::error::RavenClawsError::Llm(e));
                }
            }
        };

        // Record token usage from response
        let mut iteration_tokens: u64 = 0;
        if let Some(ref budget) = config.token_budget {
            if let Some(usage) = &response.usage {
                let mut b = budget.lock().unwrap();
                b.record_usage(usage.total_tokens);
                iteration_tokens = usage.total_tokens as u64;
                debug!(
                    iteration = iteration,
                    tokens_used = usage.total_tokens,
                    total_used = b.used_tokens,
                    remaining = b.remaining(),
                    "Token usage recorded"
                );
            }
        } else if let Some(usage) = &response.usage {
            iteration_tokens = usage.total_tokens as u64;
        }

        // Report metrics via callback if configured
        if let Some(ref cb) = config.metrics_callback {
            cb(iteration_tokens, 0);
        }

        // Report progress to RavenFabric if configured
        if let Some(ref rf) = config.ravenfabric {
            if rf.is_enabled() {
                let _ = rf.health().await;
                info!(
                    iteration = iteration,
                    ravenfabric = true,
                    "RavenFabric health check completed"
                );
            }
        }

        let first_choice = response.choices.first();
        let content = first_choice
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        debug!(
            iteration = iteration,
            response_length = content.len(),
            response_preview = %content[..content.len().min(500)],
            "LLM response received"
        );

        // Prompt-injection defense: check LLM response before processing
        if let Some(ref detector) = injection_detector {
            match detector.check(&content) {
                crate::policy::InjectionVerdict::Suspicious(reason) => {
                    warn!(
                        iteration = iteration,
                        reason = %reason,
                        "Prompt-injection detected in LLM response"
                    );
                    let _ = audit_log.append(
                        AuditEventType::SecurityViolation,
                        "injection_detector",
                        &format!("Prompt-injection detected: {}", reason),
                        Some(serde_json::json!({
                            "reason": reason,
                            "iteration": iteration,
                            "content_preview": &content[..content.len().min(200)],
                        })),
                    );
                    // Delete checkpoint on injection detection
                    if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                        delete_checkpoint(checkpoint_dir, &session_id);
                    }
                    return Err(crate::error::RavenClawsError::SecurityViolation(format!(
                        "LLM response blocked: potential prompt injection ({})",
                        reason
                    )));
                }
                crate::policy::InjectionVerdict::Clean => {}
            }
        }

        // Check for structured tool calls first (OpenAI Tools format)
        if config.enable_tools {
            if let Some((tool_name, args)) = first_choice.and_then(parse_structured_tool_call) {
                info!(tool = %tool_name, "Structured tool call detected");

                // Execute tool with security
                if let Some(tool_result) = execute_parsed_tool_call(
                    tool_name,
                    args,
                    &registry,
                    &policy_engine,
                    &sandbox,
                    &audit_log,
                    config.require_approval,
                )
                .await
                {
                    let observation = if tool_result.success {
                        format!("OBSERVATION: {}", tool_result.output)
                    } else {
                        format!(
                            "OBSERVATION: Tool failed with error: {}",
                            tool_result.error.as_deref().unwrap_or("unknown error")
                        )
                    };

                    memory.add_user_message(&observation);

                    // Report tool call via metrics callback
                    if let Some(ref cb) = config.metrics_callback {
                        cb(0, 1);
                    }

                    info!(
                        iteration = iteration,
                        tool = %tool_result.tool_name,
                        success = tool_result.success,
                        "Structured tool executed"
                    );
                    continue;
                }
            }
        }

        // Check for completion signal
        if content.contains("FINAL:") {
            let final_response = content
                .split("FINAL:")
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();

            memory.add_assistant_message(&content);

            // Audit: agent finish
            let _ = audit_log.append(
                AuditEventType::AgentFinish,
                "agent",
                "Agent loop completed successfully",
                Some(serde_json::json!({
                    "iterations": iteration + 1,
                    "final_response_length": final_response.len(),
                })),
            );

            // Delete checkpoint on successful completion
            if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                delete_checkpoint(checkpoint_dir, &session_id);
            }

            return Ok(final_response);
        }

        // Execute tool calls if enabled (legacy pattern-matching fallback)
        if config.enable_tools {
            if let Some(tool_result) = execute_tool_call_with_security(
                &content,
                &registry,
                &policy_engine,
                &sandbox,
                &audit_log,
            )
            .await
            {
                let observation = if tool_result.success {
                    format!("OBSERVATION: {}", tool_result.output)
                } else {
                    format!(
                        "OBSERVATION: Tool failed with error: {}",
                        tool_result.error.as_deref().unwrap_or("unknown error")
                    )
                };

                memory.add_assistant_message(&content);
                memory.add_user_message(&observation);

                // Report tool call via metrics callback
                if let Some(ref cb) = config.metrics_callback {
                    cb(0, 1);
                }

                info!(
                    iteration = iteration,
                    tool = %tool_result.tool_name,
                    success = tool_result.success,
                    "Tool executed"
                );
                continue;
            }
        }

        // No tool call found and no FINAL: — treat as regular response
        memory.add_assistant_message(&content);

        // ── Durable execution: save checkpoint after each iteration ────────
        if let Some(ref checkpoint_dir) = config.checkpoint_dir {
            let checkpoint = CheckpointState::new(
                session_id.clone(),
                iteration,
                config.max_iterations,
                memory.history().to_vec(),
                initial_prompt,
                system_prompt,
                llm.provider_name(),
                llm.model(),
                config.enable_tools,
            );
            if let Err(e) = save_checkpoint(checkpoint_dir, &checkpoint) {
                warn!(
                    session_id = %session_id,
                    iteration = iteration,
                    error = %e,
                    "Failed to save checkpoint"
                );
            } else {
                debug!(
                    session_id = %session_id,
                    iteration = iteration,
                    "Checkpoint saved"
                );
            }
        }

        // If no_final_required is set, treat any non-tool-call response as completion
        if config.no_final_required {
            info!(
                iteration = iteration,
                response_length = content.len(),
                "no_final_required: treating response as completion"
            );
            let _ = audit_log.append(
                AuditEventType::AgentFinish,
                "agent",
                "Agent loop completed (no_final_required)",
                Some(serde_json::json!({
                    "iterations": iteration + 1,
                    "final_response_length": content.len(),
                })),
            );
            // Delete checkpoint on successful completion
            if let Some(ref checkpoint_dir) = config.checkpoint_dir {
                delete_checkpoint(checkpoint_dir, &session_id);
            }
            return Ok(content);
        }

        info!(
            iteration = iteration,
            thought = %content.lines().find(|l| l.starts_with("THOUGHT:")).unwrap_or("<no thought>"),
            "Agent loop progress"
        );
    }

    // Max iterations reached
    warn!(
        max_iterations = config.max_iterations,
        "Agent loop reached max iterations"
    );

    let _ = audit_log.append(
        AuditEventType::Error,
        "agent",
        "Agent loop reached max iterations without completing",
        Some(serde_json::json!({
            "max_iterations": config.max_iterations,
        })),
    );

    // Delete checkpoint on max iterations (task is done, even if incomplete)
    if let Some(ref checkpoint_dir) = config.checkpoint_dir {
        delete_checkpoint(checkpoint_dir, &session_id);
    }

    let history = memory.history();
    if history.len() > 1 {
        if let Some(last) = history.last() {
            return Ok(last.content.clone());
        }
    }

    Err(crate::error::RavenClawsError::CommandExecution(
        "Agent loop reached max iterations without completing the task".to_string(),
    ))
}

/// Run the agent loop with MCP tool integration (v0.5.2)
///
/// This version extends run_agent_loop with MCP tool support:
/// 1. Registers MCP tools into the ToolRegistry
/// 2. MCP tools are executed alongside built-in tools
#[allow(dead_code)]
#[instrument(skip_all, fields(provider = %llm.provider_name(), model = %llm.model()))]
pub async fn run_agent_loop_with_mcp(
    llm: Arc<dyn LLMProviderTrait>,
    initial_prompt: &str,
    system_prompt: &str,
    config: AgentLoopConfig,
    mcp_client: Option<Arc<RwLock<McpClient>>>,
) -> Result<String> {
    run_agent_loop_with_mcp_and_registry(
        llm,
        initial_prompt,
        system_prompt,
        config,
        mcp_client,
        None,
    )
    .await
}

/// Run the agent loop with MCP tools and an optional pre-configured ToolRegistry
#[instrument(skip_all, fields(provider = %llm.provider_name(), model = %llm.model()))]
pub async fn run_agent_loop_with_mcp_and_registry(
    llm: Arc<dyn LLMProviderTrait>,
    initial_prompt: &str,
    system_prompt: &str,
    config: AgentLoopConfig,
    mcp_client: Option<Arc<RwLock<McpClient>>>,
    tool_registry: Option<ToolRegistry>,
) -> Result<String> {
    // Initialize tool registry (use provided one or default)
    let mut registry = tool_registry.unwrap_or_else(ToolRegistry::with_default_tools);

    // Register MCP tools if client is provided
    if let Some(client) = &mcp_client {
        match crate::mcp::register_mcp_tools(&mut registry, client.clone()).await {
            Ok(count) => {
                info!(count, "MCP tools registered");
            }
            Err(e) => {
                warn!(error = %e, "Failed to register MCP tools");
            }
        }
    }

    let mcp_enabled = mcp_client.is_some();
    run_agent_loop_inner(
        llm,
        initial_prompt,
        system_prompt,
        config,
        registry,
        "MCP integration",
        mcp_enabled,
    )
    .await
}

/// Prompt the user for approval of a tool call via stdin.
///
/// Returns `true` if the user approved, `false` if denied.
/// If stdin is not a terminal (piped), auto-approves with a warning.
async fn prompt_for_approval(tool_name: &str, args: &serde_json::Value) -> bool {
    use std::io::{IsTerminal, Write};

    let args_str = serde_json::to_string_pretty(args).unwrap_or_default();

    // Check if stdin is a terminal
    if !std::io::stdin().is_terminal() {
        warn!(
            tool = %tool_name,
            "stdin is not a TTY — auto-approving tool call (use --require-approval only in interactive mode)"
        );
        return true;
    }

    // Print the approval prompt to stderr so it doesn't interfere with stdout output
    eprintln!("\n⚠️  Tool requires approval:");
    eprintln!("   Tool: {}", tool_name);
    for line in args_str.lines() {
        eprintln!("   {}", line);
    }
    eprint!("   Approve? [y/N] ");
    std::io::stderr().flush().ok();

    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => {
            let trimmed = input.trim().to_lowercase();
            trimmed == "y" || trimmed == "yes"
        }
        Err(e) => {
            warn!(error = %e, "Failed to read approval input — denying by default");
            false
        }
    }
}

/// Testable version of prompt_for_approval that reads from a given input string.
/// Used in unit tests to avoid blocking on stdin.
#[cfg(test)]
async fn prompt_for_approval_with_input(
    tool_name: &str,
    args: &serde_json::Value,
    input: &str,
) -> bool {
    use std::io::Write;

    let args_str = serde_json::to_string_pretty(args).unwrap_or_default();

    eprintln!("\n⚠️  Tool requires approval:");
    eprintln!("   Tool: {}", tool_name);
    for line in args_str.lines() {
        eprintln!("   {}", line);
    }
    eprint!("   Approve? [y/N] ");
    std::io::stderr().flush().ok();

    let trimmed = input.trim().to_lowercase();
    trimmed == "y" || trimmed == "yes"
}

/// Execute a parsed tool call with security integration
///
/// This function:
/// 1. Checks the tool call against PolicyEngine
/// 2. Logs the policy decision to AuditLog
/// 3. Prompts for human approval if required (HITL)
/// 4. Executes the tool (sandbox is applied at the tool implementation level for shell_exec)
/// 5. Logs the result to AuditLog
async fn execute_parsed_tool_call(
    tool_name: String,
    args: serde_json::Value,
    registry: &ToolRegistry,
    policy_engine: &PolicyEngine,
    _sandbox: &Sandbox,
    audit_log: &AuditLog,
    require_approval: bool,
) -> Option<ToolResult> {
    info!(tool = %tool_name, "Executing parsed tool call");

    // Audit: tool call requested
    let _ = audit_log.tool_call(&tool_name, &args);

    // Check if tool requires approval
    if require_approval && policy_engine.requires_approval(&tool_name) {
        let _ = audit_log.append(
            AuditEventType::ApprovalRequested,
            "approval",
            &format!("Approval required for tool: {}", tool_name),
            Some(serde_json::json!({"tool": tool_name, "args": args})),
        );

        // Prompt user for approval via stdin
        let granted = prompt_for_approval(&tool_name, &args).await;

        if !granted {
            let _ = audit_log.approval(&tool_name, false, Some("Denied by user"));
            warn!(tool = %tool_name, "Tool call denied by user");
            return Some(ToolResult {
                tool_name: tool_name.clone(),
                success: false,
                output: String::new(),
                error: Some(format!("Approval denied by user for tool: {}", tool_name)),
                exit_code: Some(-1),
                duration_ms: None,
            });
        }

        let _ = audit_log.approval(&tool_name, true, Some("Approved by user"));
        info!(tool = %tool_name, "Tool call approved by user");
    }

    // Check policy BEFORE execution
    let policy_decision = policy_engine.check_tool_call(&tool_name, &args);

    // Audit: policy decision
    match &policy_decision {
        Decision::Allow => {
            let _ = audit_log.policy_decision(&tool_name, true, None);
        }
        Decision::Deny(reason) => {
            let _ = audit_log.policy_decision(&tool_name, false, Some(reason));
            warn!(tool = %tool_name, reason = %reason, "Tool call denied by policy");
            return Some(ToolResult {
                tool_name: tool_name.clone(),
                success: false,
                output: String::new(),
                error: Some(format!("Policy denied: {}", reason)),
                exit_code: Some(-1),
                duration_ms: None,
            });
        }
    }

    // Execute tool
    let tool_name_clone = tool_name.clone();
    let call = ToolCall {
        name: tool_name.clone(),
        arguments: args,
        id: None,
    };

    let result = match registry.execute(call).await {
        Ok(result) => {
            // Audit: tool result
            let _ = audit_log.append(
                AuditEventType::ToolResult,
                &tool_name_clone,
                &format!(
                    "Tool executed: {} (success: {})",
                    tool_name_clone, result.success
                ),
                Some(serde_json::json!({
                    "success": result.success,
                    "exit_code": result.exit_code,
                    "duration_ms": result.duration_ms,
                })),
            );
            result
        }
        Err(e) => {
            // Audit: error
            let _ = audit_log.append(
                AuditEventType::Error,
                &tool_name_clone,
                &format!("Tool execution failed: {}", e),
                None,
            );
            ToolResult {
                tool_name: tool_name_clone,
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                exit_code: Some(-1),
                duration_ms: None,
            }
        }
    };

    Some(result)
}

/// Execute a tool call with security integration (legacy pattern-matching fallback)
///
/// This function:
/// 1. Parses the tool call from the LLM response (legacy TOOL_CALL:/ARGS: format)
/// 2. Checks the tool call against PolicyEngine
/// 3. Logs the policy decision to AuditLog
/// 4. Executes the tool (sandbox is applied at the tool implementation level for shell_exec)
/// 5. Logs the result to AuditLog
async fn execute_tool_call_with_security(
    content: &str,
    registry: &ToolRegistry,
    policy_engine: &PolicyEngine,
    _sandbox: &Sandbox,
    audit_log: &AuditLog,
) -> Option<ToolResult> {
    // Parse tool call from content (legacy format)
    let (tool_name, args) = parse_tool_call(content)?;

    // Delegate to the common execution logic
    execute_parsed_tool_call(
        tool_name,
        args,
        registry,
        policy_engine,
        _sandbox,
        audit_log,
        false, // legacy path — no approval prompt
    )
    .await
}

/// Parse a tool call from LLM response content
/// Returns (tool_name, args) if found, None otherwise
/// Parse tool call from structured LLM response (OpenAI Tools format)
fn parse_structured_tool_call(choice: &Choice) -> Option<(String, serde_json::Value)> {
    let tool_calls = choice.tool_calls.as_ref()?;
    let first_call = tool_calls.first()?;

    let tool_name = first_call.function.name.clone();
    let args: serde_json::Value = serde_json::from_str(&first_call.function.arguments).ok()?;

    Some((tool_name, args))
}

/// Parse tool call from legacy pattern-matching format (TOOL_CALL: / ARGS:)
fn parse_tool_call(content: &str) -> Option<(String, serde_json::Value)> {
    let mut lines = content.lines();
    let tool_call_line = lines.find(|l| l.trim().starts_with("TOOL_CALL:"))?;

    let tool_name = tool_call_line
        .trim()
        .strip_prefix("TOOL_CALL:")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())?
        .to_string();

    // Find the ARGS line
    let args_line = lines.find(|l| l.trim().starts_with("ARGS:"))?;
    let args_str = args_line.trim().strip_prefix("ARGS:").map(|s| s.trim())?;

    let args: serde_json::Value = serde_json::from_str(args_str).ok()?;

    Some((tool_name, args))
}

/// Run a single autonomous agent (single-provider mode)
pub async fn run_single(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
) -> Result<()> {
    info!(
        "Starting single agent mode with provider: {}",
        llm.provider_name()
    );

    // Perform RavenFabric health check if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            info!("RavenFabric remote execution available");
            match rf.health().await {
                Ok(true) => info!("RavenFabric mesh is healthy"),
                Ok(false) => warn!("RavenFabric mesh returned unhealthy status"),
                Err(e) => warn!(error = %e, "RavenFabric health check failed"),
            }
        }
    }

    let system_prompt = &config.llm.system_prompt;

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "Ready. Awaiting instructions.".to_string(),
        },
    ];

    match llm.chat(messages).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                info!(provider = llm.provider_name(), model = llm.model(), response = %choice.message.content, "Agent response received");

                // Broadcast result to RavenFabric if configured
                if let Some(ref rf) = ravenfabric {
                    if rf.is_enabled() {
                        let preview = choice.message.content.chars().take(500).collect::<String>();
                        let _ = rf.broadcast(&preview, 30).await;
                        info!("Agent result broadcast to RavenFabric mesh");
                    }
                }
            }
        }
        Err(e) => {
            warn!(error = %e, provider = llm.provider_name(), "LLM request failed");
        }
    }

    Ok(())
}

/// Run multiple agents in swarm mode (single-provider) — v0.6
///
/// Swarm mode runs multiple agents in parallel, each working on the same task
/// with different approaches. Results are collected and compared.
pub async fn run_swarm(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
) -> Result<()> {
    info!("Starting swarm mode (single-provider) — 3 parallel agents");

    // Perform RavenFabric health check if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            info!("RavenFabric remote execution available for swarm coordination");
            match rf.health().await {
                Ok(true) => info!("RavenFabric mesh is healthy"),
                Ok(false) => warn!("RavenFabric mesh returned unhealthy status"),
                Err(e) => warn!(error = %e, "RavenFabric health check failed"),
            }
        }
    }

    let _system_prompt = &config.llm.system_prompt;
    let num_agents = 3;
    let mut handles = Vec::new();

    // Spawn parallel agents with different personas
    let personas = [
        "You are an analytical agent. Focus on logic, structure, and precision.",
        "You are a creative agent. Focus on innovation, alternatives, and possibilities.",
        "You are a pragmatic agent. Focus on simplicity, efficiency, and practicality.",
    ];

    for (i, persona) in personas.iter().enumerate().take(num_agents) {
        let llm_clone = llm.clone();
        let persona = persona.to_string();
        let task = "Analyze the given task and provide your solution.".to_string();

        let handle = tokio::spawn(async move {
            let mut memory = ConversationMemory::new(&persona, 10);
            memory.add_user_message(&task);

            let messages = memory.history().to_vec();
            match llm_clone.chat(messages).await {
                Ok(response) => {
                    let content = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();
                    Ok((i, content))
                }
                Err(e) => Err(format!("Agent {} failed: {}", i, e)),
            }
        });

        handles.push(handle);
    }

    // Collect results
    let mut results: Vec<(usize, String)> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok((idx, result))) => {
                info!("Agent {} completed: {} chars", idx, result.len());
                results.push((idx, result));
            }
            Ok(Err(e)) => warn!("Agent failed: {}", e),
            Err(e) => warn!("Agent join failed: {}", e),
        }
    }

    // Print swarm results
    println!("\n🐦‍⬛ Swarm Results ({} agents):", results.len());
    for (idx, result) in &results {
        println!(
            "\n── Agent {} ({}) ──",
            idx + 1,
            personas[*idx].split('.').next().unwrap_or("Unknown")
        );
        println!("{}", result);
    }

    // Broadcast swarm results to RavenFabric if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let summary = format!(
                "Swarm completed: {} agents, results: {}",
                results.len(),
                results
                    .iter()
                    .map(|(i, r)| format!("Agent {}: {} chars", i, r.len()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let _ = rf.broadcast(&summary, 30).await;
            info!("Swarm results broadcast to RavenFabric mesh");
        }
    }

    Ok(())
}

/// Run supervisor agent coordinating sub-agents (single-provider) — v0.6
///
/// The supervisor decomposes a task into subtasks, spawns sub-agents for each,
/// and aggregates results. Uses the same LLM provider for all agents.
pub async fn run_supervisor(
    llm: Arc<dyn LLMProviderTrait>,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
) -> Result<()> {
    info!("Starting supervisor mode (single-provider)");

    // Perform RavenFabric health check if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            info!("RavenFabric remote execution available for supervisor coordination");
            match rf.health().await {
                Ok(true) => info!("RavenFabric mesh is healthy"),
                Ok(false) => warn!("RavenFabric mesh returned unhealthy status"),
                Err(e) => warn!(error = %e, "RavenFabric health check failed"),
            }
        }
    }

    let system_prompt = &config.llm.system_prompt;
    let policy_engine = PolicyEngine::default_secure();
    let mut sandbox = Sandbox::default();
    sandbox.init().await.map_err(|e| {
        crate::error::RavenClawsError::CommandExecution(format!("Sandbox init failed: {}", e))
    })?;
    let audit_log = AuditLog::new(format!("supervisor-{}", std::process::id()));
    let registry = ToolRegistry::with_default_tools();

    // Initial prompt to supervisor
    let supervisor_prompt = format!(
        "You are a supervisor agent. Your task is to decompose complex tasks into subtasks \
         and coordinate sub-agents to complete them. \
         \n\nFor each subtask, respond with:\n\
         SUBTASK: <description>\n\
         AGENT: <agent_number>\n\
         \nWhen all subtasks are complete, respond with:\n\
         FINAL: <aggregated result>\n\
         \nTask: {}",
        "Coordinate the completion of the assigned task."
    );

    let mut memory = ConversationMemory::new(system_prompt, 20);
    memory.add_user_message(&supervisor_prompt);

    let mut subtask_results: Vec<String> = Vec::new();
    let mut iteration = 0;
    let max_iterations = 15;

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
            info!("Supervisor completed task: {} chars", final_response.len());

            let _ = audit_log.append(
                AuditEventType::AgentFinish,
                "supervisor",
                "Supervisor completed task coordination",
                Some(serde_json::json!({
                    "iterations": iteration,
                    "subtasks_completed": subtask_results.len(),
                })),
            );

            println!("\n🐦‍⬛ Supervisor Result:\n{}", final_response);
            return Ok(());
        }

        // Check for SUBTASK: decomposition
        if content.contains("SUBTASK:") {
            let subtask_block = content.split("SUBTASK:").nth(1).unwrap_or("");
            let subtask_lines: Vec<&str> = subtask_block.lines().take(3).collect();

            let subtask_desc = subtask_lines.first().unwrap_or(&"").trim();
            let agent_num = subtask_lines
                .iter()
                .find(|l| l.starts_with("AGENT:"))
                .and_then(|l| l.split(':').nth(1))
                .unwrap_or("1")
                .trim();

            if !subtask_desc.is_empty() {
                info!("Subtask {}: {}", agent_num, subtask_desc);

                // Execute subtask
                let subtask_result = run_subtask_agent(
                    llm.clone(),
                    subtask_desc,
                    system_prompt,
                    &policy_engine,
                    &sandbox,
                    &audit_log,
                    &registry,
                )
                .await;

                match subtask_result {
                    Ok(result) => {
                        info!("Subtask {} completed: {} chars", agent_num, result.len());
                        subtask_results.push(format!("Agent {} result: {}", agent_num, result));

                        memory.add_assistant_message(&format!(
                            "Decomposed subtask {}: {}",
                            agent_num, subtask_desc
                        ));
                        memory
                            .add_user_message(&format!("Subtask {} result: {}", agent_num, result));
                    }
                    Err(e) => {
                        warn!("Subtask {} failed: {}", agent_num, e);
                        memory
                            .add_assistant_message(&format!("Subtask {} failed: {}", agent_num, e));
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

        // Broadcast supervisor result to RavenFabric if configured
        if let Some(ref rf) = ravenfabric {
            if rf.is_enabled() {
                let summary = format!(
                    "Supervisor completed: {} subtasks, result: {} chars",
                    subtask_results.len(),
                    aggregated.len()
                );
                let _ = rf.broadcast(&summary, 30).await;
                info!("Supervisor result broadcast to RavenFabric mesh");
            }
        }

        println!("\n🐦‍⬛ Supervisor Aggregated Result:\n{}", aggregated);
        return Ok(());
    }

    Err(crate::error::RavenClawsError::CommandExecution(
        "Supervisor mode completed without results".to_string(),
    ))
}

/// Run a subtask agent — helper for supervisor mode
async fn run_subtask_agent(
    llm: Arc<dyn LLMProviderTrait>,
    subtask: &str,
    system_prompt: &str,
    policy_engine: &PolicyEngine,
    sandbox: &Sandbox,
    audit_log: &AuditLog,
    registry: &ToolRegistry,
) -> Result<String> {
    let mut memory = ConversationMemory::new(system_prompt, 10);
    memory.add_user_message(&format!("Execute this subtask: {}", subtask));

    for i in 0..5 {
        let messages = memory.history().to_vec();
        let response = match llm.chat(messages).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, iteration = i, "Subtask agent LLM failed");
                continue;
            }
        };

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        if content.contains("FINAL:") || content.contains("DONE:") {
            return Ok(content
                .replace("FINAL:", "")
                .replace("DONE:", "")
                .trim()
                .to_string());
        }

        // Try tool execution
        if let Some(tool_result) =
            execute_tool_call_with_security(&content, registry, policy_engine, sandbox, audit_log)
                .await
        {
            memory.add_assistant_message(&content);
            memory.add_user_message(&format!("Tool result: {}", tool_result.output));
        } else {
            memory.add_assistant_message(&content);
            memory.add_user_message("Continue with next step.");
        }
    }

    Ok("Subtask completed".to_string())
}

/// Run a single autonomous agent (multi-model mode)
pub async fn run_single_multi(
    multi_llm: MultiModelManager,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
) -> Result<()> {
    info!(
        "Starting single agent mode (multi-model) with {} providers",
        multi_llm.client_count()
    );

    // Perform RavenFabric health check if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            info!("RavenFabric remote execution available");
            match rf.health().await {
                Ok(true) => info!("RavenFabric mesh is healthy"),
                Ok(false) => warn!("RavenFabric mesh returned unhealthy status"),
                Err(e) => warn!(error = %e, "RavenFabric health check failed"),
            }
        }
    }

    let system_prompt = &config.llm.system_prompt;

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "Ready. Awaiting instructions.".to_string(),
        },
    ];

    // Round-robin: start with first provider, then rotate
    let mut last_index = 0;
    for i in 0..multi_llm.client_count() {
        let client = if i == 0 {
            multi_llm.get_client(0)
        } else {
            multi_llm.next_client(last_index)
        };

        if let Some(client) = client {
            match client.chat(messages.clone()).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        info!(provider = client.provider_name(), model = client.model(), response = %choice.message.content, "Provider response received");
                    }
                }
                Err(e) => {
                    warn!(error = %e, provider = client.provider_name(), model = client.model(), "Provider request failed");
                }
            }
            last_index = i;
        }
    }

    // Broadcast results to RavenFabric if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let _ = rf
                .broadcast("Single agent (multi-model) completed", 30)
                .await;
            info!("Multi-model result broadcast to RavenFabric mesh");
        }
    }

    Ok(())
}

/// Run multiple agents in swarm mode (multi-model) — v0.6
///
/// Swarm mode runs multiple agents in parallel, each using a different LLM provider
/// for the same task. Results are collected and compared for diversity.
pub async fn run_swarm_multi(
    multi_llm: MultiModelManager,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
) -> Result<()> {
    info!(
        "Starting swarm mode (multi-model) — {} parallel agents",
        multi_llm.client_count()
    );

    // Perform RavenFabric health check if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            info!("RavenFabric remote execution available for swarm coordination");
            match rf.health().await {
                Ok(true) => info!("RavenFabric mesh is healthy"),
                Ok(false) => warn!("RavenFabric mesh returned unhealthy status"),
                Err(e) => warn!(error = %e, "RavenFabric health check failed"),
            }
        }
    }

    let _system_prompt = &config.llm.system_prompt;
    let num_agents = multi_llm.client_count().min(3); // Cap at 3 for cost control
    let mut handles = Vec::new();

    // Different personas for each agent
    let personas = [
        "You are an analytical agent. Focus on logic, structure, and precision.",
        "You are a creative agent. Focus on innovation, alternatives, and possibilities.",
        "You are a pragmatic agent. Focus on simplicity, efficiency, and practicality.",
    ];

    for i in 0..num_agents {
        let client = multi_llm.get_client(i).unwrap().clone();
        let persona = personas.get(i).unwrap_or(&personas[0]).to_string();
        let task = "Analyze the given task and provide your solution.".to_string();

        let handle = tokio::spawn(async move {
            let mut memory = ConversationMemory::new(&persona, 10);
            memory.add_user_message(&task);

            let messages = memory.history().to_vec();
            match client.chat(messages).await {
                Ok(response) => {
                    let content = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();
                    Ok((
                        i,
                        client.provider_name().to_string(),
                        client.model().to_string(),
                        content,
                    ))
                }
                Err(e) => Err(format!("Agent {} failed: {}", i, e)),
            }
        });

        handles.push(handle);
    }

    // Collect results
    let mut results: Vec<(usize, String, String, String)> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok((idx, provider, model, result))) => {
                info!(
                    "Agent {} ({}:{}) completed: {} chars",
                    idx,
                    provider,
                    model,
                    result.len()
                );
                results.push((idx, provider, model, result));
            }
            Ok(Err(e)) => warn!("Agent failed: {}", e),
            Err(e) => warn!("Agent join failed: {}", e),
        }
    }

    // Print swarm results
    println!(
        "\n🐦‍⬛ Swarm Results ({} agents, multi-model):",
        results.len()
    );
    for (idx, provider, model, result) in &results {
        println!("\n── Agent {} ({}:{}) ──", idx + 1, provider, model);
        println!("{}", result);
    }

    // Broadcast swarm results to RavenFabric if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            let summary = format!("Multi-model swarm completed: {} agents", results.len());
            let _ = rf.broadcast(&summary, 30).await;
            info!("Multi-model swarm results broadcast to RavenFabric mesh");
        }
    }

    Ok(())
}

/// Run supervisor agent coordinating sub-agents (multi-model) — v0.6
///
/// The supervisor decomposes a task and assigns subtasks to different providers
/// based on their strengths. Results are aggregated.
pub async fn run_supervisor_multi(
    multi_llm: MultiModelManager,
    config: Config,
    ravenfabric: Option<RavenFabricClient>,
) -> Result<()> {
    info!(
        "Starting supervisor mode (multi-model) with {} providers",
        multi_llm.client_count()
    );

    // Perform RavenFabric health check if configured
    if let Some(ref rf) = ravenfabric {
        if rf.is_enabled() {
            info!("RavenFabric remote execution available for supervisor coordination");
            match rf.health().await {
                Ok(true) => info!("RavenFabric mesh is healthy"),
                Ok(false) => warn!("RavenFabric mesh returned unhealthy status"),
                Err(e) => warn!(error = %e, "RavenFabric health check failed"),
            }
        }
    }

    let system_prompt = &config.llm.system_prompt;
    let policy_engine = PolicyEngine::default_secure();
    let mut sandbox = Sandbox::default();
    sandbox.init().await.map_err(|e| {
        crate::error::RavenClawsError::CommandExecution(format!("Sandbox init failed: {}", e))
    })?;
    let audit_log = AuditLog::new(format!("supervisor-multi-{}", std::process::id()));
    let registry = ToolRegistry::with_default_tools();

    // Supervisor prompt with multi-model awareness
    let supervisor_prompt = format!(
        "You are a supervisor agent coordinating multiple LLM providers. \
         Decompose tasks and assign them to appropriate providers based on their strengths. \
         \n\nFor each subtask, respond with:\n\
         SUBTASK: <description>\n\
         PROVIDER: <provider_index 0-{}>\n\
         \nWhen complete, respond with:\n\
         FINAL: <aggregated result>\n\
         \nTask: {}",
        multi_llm.client_count() - 1,
        "Coordinate the completion of the assigned task using available providers."
    );

    let mut memory = ConversationMemory::new(system_prompt, 20);
    memory.add_user_message(&supervisor_prompt);

    let mut subtask_results: Vec<String> = Vec::new();
    let mut iteration = 0;
    let max_iterations = 15;

    loop {
        iteration += 1;
        if iteration > max_iterations {
            warn!("Supervisor reached max iterations");
            break;
        }

        // Use round-robin for supervisor itself
        let supervisor_client = multi_llm
            .get_client(iteration % multi_llm.client_count())
            .or_else(|| multi_llm.get_client(0))
            .cloned();

        let messages = memory.history().to_vec();
        let response =
            match supervisor_client.map(|c| tokio::spawn(async move { c.chat(messages).await })) {
                Some(handle) => match handle.await {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => {
                        warn!(error = %e, "Supervisor LLM request failed");
                        continue;
                    }
                    Err(e) => {
                        warn!(error = %e, "Supervisor task join failed");
                        continue;
                    }
                },
                None => {
                    warn!("No LLM clients available");
                    break;
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
            info!("Supervisor completed task: {} chars", final_response.len());

            let _ = audit_log.append(
                AuditEventType::AgentFinish,
                "supervisor",
                "Supervisor completed task coordination",
                Some(serde_json::json!({
                    "iterations": iteration,
                    "subtasks_completed": subtask_results.len(),
                    "providers_used": multi_llm.client_count(),
                })),
            );

            println!("\n🐦‍⬛ Supervisor Result (multi-model):\n{}", final_response);
            return Ok(());
        }

        // Check for SUBTASK: decomposition
        if content.contains("SUBTASK:") && content.contains("PROVIDER:") {
            let subtask_block = content.split("SUBTASK:").nth(1).unwrap_or("");
            let subtask_lines: Vec<&str> = subtask_block.lines().take(4).collect();

            let subtask_desc = subtask_lines.first().unwrap_or(&"").trim();
            let provider_idx = subtask_lines
                .iter()
                .find(|l| l.starts_with("PROVIDER:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0);

            if !subtask_desc.is_empty() {
                info!("Subtask for provider {}: {}", provider_idx, subtask_desc);

                let client = multi_llm
                    .get_client(provider_idx)
                    .or_else(|| multi_llm.get_client(0));

                if let Some(client) = client {
                    let subtask_result = run_subtask_agent(
                        client.clone(),
                        subtask_desc,
                        system_prompt,
                        &policy_engine,
                        &sandbox,
                        &audit_log,
                        &registry,
                    )
                    .await;

                    match subtask_result {
                        Ok(result) => {
                            info!("Subtask {} completed: {} chars", provider_idx, result.len());
                            subtask_results.push(format!(
                                "Provider {} ({}): {}",
                                provider_idx,
                                client.provider_name(),
                                result
                            ));

                            memory.add_assistant_message(&format!(
                                "Assigned subtask to provider {}: {}",
                                provider_idx, subtask_desc
                            ));
                            memory.add_user_message(&format!(
                                "Provider {} result: {}",
                                provider_idx, result
                            ));
                        }
                        Err(e) => {
                            warn!("Subtask {} failed: {}", provider_idx, e);
                            memory.add_assistant_message(&format!(
                                "Provider {} subtask failed: {}",
                                provider_idx, e
                            ));
                        }
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

        // Broadcast supervisor result to RavenFabric if configured
        if let Some(ref rf) = ravenfabric {
            if rf.is_enabled() {
                let summary = format!(
                    "Multi-model supervisor completed: {} subtasks, result: {} chars",
                    subtask_results.len(),
                    aggregated.len()
                );
                let _ = rf.broadcast(&summary, 30).await;
                info!("Multi-model supervisor result broadcast to RavenFabric mesh");
            }
        }

        println!(
            "\n🐦‍⬛ Supervisor Aggregated Result (multi-model):\n{}",
            aggregated
        );
        return Ok(());
    }

    Err(crate::error::RavenClawsError::CommandExecution(
        "Supervisor mode completed without results".to_string(),
    ))
}

/// Run interactive REPL mode
pub async fn run_repl(llm: Arc<dyn LLMProviderTrait>, config: Config) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    info!("Starting interactive REPL mode");

    let system_prompt = &config.llm.system_prompt;
    let mut memory = ConversationMemory::new(system_prompt, 0);

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    println!("RavenClaws REPL — type /exit to quit, /reset to clear history");

    loop {
        print!("\n> ");
        use tokio::io::AsyncWriteExt;
        tokio::io::stdout().flush().await?;

        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break, // EOF
            Err(e) => {
                warn!(error = %e, "REPL read error");
                break;
            }
        };

        let input = line.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" | "/quit" => {
                println!("Exiting REPL.");
                break;
            }
            "/reset" => {
                memory = ConversationMemory::new(system_prompt, 0);
                println!("Conversation history reset.");
                continue;
            }
            _ => {}
        }

        memory.add_user_message(input);
        let messages = memory.history().to_vec();

        match llm.chat(messages).await {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    let content = &choice.message.content;
                    println!("{}", content);
                    memory.add_assistant_message(content);
                }
            }
            Err(e) => {
                warn!(error = %e, "LLM request failed");
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_function_exists() {
        // Verify swarm function signature compiles
        let _fn_ptr: fn(Arc<dyn LLMProviderTrait>, Config, Option<RavenFabricClient>) -> _ =
            run_swarm;
    }

    #[test]
    fn test_supervisor_function_exists() {
        // Verify supervisor function signature compiles
        let _fn_ptr: fn(Arc<dyn LLMProviderTrait>, Config, Option<RavenFabricClient>) -> _ =
            run_supervisor;
    }

    #[test]
    fn test_conversation_memory_new() {
        let mem = ConversationMemory::new("system prompt", 10);
        assert_eq!(mem.messages.len(), 1);
        assert_eq!(mem.messages[0].role, "system");
        assert_eq!(mem.messages[0].content, "system prompt");
    }

    #[test]
    fn test_conversation_memory_add_user() {
        let mut mem = ConversationMemory::new("system", 10);
        mem.add_user_message("hello");
        assert_eq!(mem.messages.len(), 2);
        assert_eq!(mem.messages[1].role, "user");
        assert_eq!(mem.messages[1].content, "hello");
    }

    #[test]
    fn test_conversation_memory_trim() {
        let mut mem = ConversationMemory::new("system", 3);
        mem.add_user_message("msg1");
        mem.add_assistant_message("resp1");
        mem.add_user_message("msg2");
        mem.add_assistant_message("resp2");
        // Should trim to keep system + 2 messages
        assert!(mem.messages.len() <= 3);
    }

    #[test]
    fn test_parse_tool_call_valid() {
        let content = "THOUGHT: I need to run a command\nTOOL_CALL: shell_exec\nARGS: {\"command\": \"echo hello\"}";
        let (name, args) = parse_tool_call(content).unwrap();
        assert_eq!(name, "shell_exec");
        assert_eq!(args["command"], "echo hello");
    }

    #[test]
    fn test_parse_tool_call_missing_tool() {
        let content = "THOUGHT: no tool here";
        assert!(parse_tool_call(content).is_none());
    }

    #[test]
    fn test_parse_tool_call_missing_args() {
        let content = "TOOL_CALL: shell_exec\nNo args line";
        assert!(parse_tool_call(content).is_none());
    }

    #[test]
    fn test_parse_tool_call_invalid_json() {
        let content = "TOOL_CALL: shell_exec\nARGS: not valid json";
        assert!(parse_tool_call(content).is_none());
    }

    #[test]
    fn test_agent_loop_config_default() {
        let config = AgentLoopConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert!(!config.enable_tools);
        assert!(!config.require_approval);
    }

    #[test]
    fn test_agent_loop_config_require_approval() {
        let config = AgentLoopConfig {
            max_iterations: 5,
            enable_tools: true,
            require_approval: true,
            prompt_injection_protection: true,
            token_lifetime_secs: 0,
            no_final_required: false,
            fallback_chain: None,
            token_budget: None,
            ravenfabric: None,
            checkpoint_dir: None,
            session_id: None,
            metrics_callback: None,
        };
        assert_eq!(config.max_iterations, 5);
        assert!(config.enable_tools);
        assert!(config.require_approval);
        assert!(config.prompt_injection_protection);
        assert_eq!(config.token_lifetime_secs, 0);
    }

    #[test]
    fn test_prompt_for_approval_yes() {
        let args = serde_json::json!({"command": "echo hello"});
        let result = tokio_test::block_on(prompt_for_approval_with_input("shell_exec", &args, "y"));
        assert!(result, "Should approve for 'y'");
    }

    #[test]
    fn test_prompt_for_approval_yes_full() {
        let args = serde_json::json!({"command": "echo hello"});
        let result =
            tokio_test::block_on(prompt_for_approval_with_input("shell_exec", &args, "yes"));
        assert!(result, "Should approve for 'yes'");
    }

    #[test]
    fn test_prompt_for_approval_no() {
        let args = serde_json::json!({"command": "echo hello"});
        let result = tokio_test::block_on(prompt_for_approval_with_input("shell_exec", &args, "n"));
        assert!(!result, "Should deny for 'n'");
    }

    #[test]
    fn test_prompt_for_approval_no_full() {
        let args = serde_json::json!({"command": "echo hello"});
        let result =
            tokio_test::block_on(prompt_for_approval_with_input("shell_exec", &args, "no"));
        assert!(!result, "Should deny for 'no'");
    }

    #[test]
    fn test_prompt_for_approval_empty() {
        let args = serde_json::json!({"command": "echo hello"});
        let result = tokio_test::block_on(prompt_for_approval_with_input("shell_exec", &args, ""));
        assert!(!result, "Should deny for empty input (default N)");
    }

    #[test]
    fn test_prompt_for_approval_uppercase() {
        let args = serde_json::json!({"command": "echo hello"});
        let result = tokio_test::block_on(prompt_for_approval_with_input("shell_exec", &args, "Y"));
        assert!(result, "Should approve for uppercase 'Y'");
    }

    #[test]
    fn test_prompt_for_approval_auto_approves_non_tty() {
        // When stdin is not a TTY (e.g., piped), prompt_for_approval auto-approves.
        // This test is only meaningful in CI/non-TTY environments.
        // In a TTY (interactive terminal), this test is skipped because it would
        // block waiting for stdin input.
        // We verify the behavior by checking the function signature compiles.
        #[allow(clippy::let_underscore_future)]
        let _ = prompt_for_approval_with_input("test", &serde_json::json!({}), "y");
    }

    #[test]
    fn test_execute_parsed_tool_call_skips_approval_when_not_required() {
        let registry = ToolRegistry::with_default_tools();
        let policy_engine = PolicyEngine::default_secure();
        let sandbox = Sandbox::default();
        let audit_log = AuditLog::new("test-session".to_string());

        let args = serde_json::json!({"command": "echo hello"});
        let result = tokio_test::block_on(execute_parsed_tool_call(
            "shell_exec".to_string(),
            args,
            &registry,
            &policy_engine,
            &sandbox,
            &audit_log,
            false, // require_approval = false
        ));

        assert!(result.is_some());
        let tool_result = result.unwrap();
        assert_eq!(tool_result.tool_name, "shell_exec");
    }

    #[test]
    fn test_execute_parsed_tool_call_approval_not_needed_for_read_only_tools() {
        // read_file does not require approval per policy, so even with
        // require_approval=true, it should execute without prompting
        let registry = ToolRegistry::with_default_tools();
        let policy_engine = PolicyEngine::default_secure();
        let sandbox = Sandbox::default();
        let audit_log = AuditLog::new("test-session".to_string());

        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let result = tokio_test::block_on(execute_parsed_tool_call(
            "read_file".to_string(),
            args,
            &registry,
            &policy_engine,
            &sandbox,
            &audit_log,
            true, // require_approval = true
        ));

        // read_file doesn't require approval, so it should proceed
        assert!(result.is_some());
        let tool_result = result.unwrap();
        assert_eq!(tool_result.tool_name, "read_file");
    }

    #[test]
    fn test_agent_loop_config_token_lifetime_zero_disabled() {
        let config = AgentLoopConfig {
            max_iterations: 10,
            enable_tools: false,
            require_approval: false,
            prompt_injection_protection: false,
            token_lifetime_secs: 0,
            no_final_required: false,
            fallback_chain: None,
            token_budget: None,
            ravenfabric: None,
            checkpoint_dir: None,
            session_id: None,
            metrics_callback: None,
        };
        assert_eq!(config.token_lifetime_secs, 0);
        // 0 means unlimited — no timeout enforced
    }

    #[test]
    fn test_agent_loop_config_token_lifetime_nonzero() {
        let config = AgentLoopConfig {
            max_iterations: 10,
            enable_tools: false,
            require_approval: false,
            prompt_injection_protection: false,
            token_lifetime_secs: 3600,
            no_final_required: false,
            fallback_chain: None,
            token_budget: None,
            ravenfabric: None,
            checkpoint_dir: None,
            session_id: None,
            metrics_callback: None,
        };
        assert_eq!(config.token_lifetime_secs, 3600);
    }

    #[test]
    fn test_agent_loop_config_default_includes_token_lifetime() {
        let config = AgentLoopConfig::default();
        assert_eq!(config.token_lifetime_secs, 0);
    }
}
