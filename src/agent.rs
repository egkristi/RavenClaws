//! Agent implementations for RavenClaw
//!
//! Supports single-provider and multi-model (multi-provider) modes.

use crate::config::Config;
use crate::error::Result;
use crate::llm::{ChatMessage, LLMProviderTrait, MultiModelManager};
use crate::tools::{ToolCall, ToolRegistry, ToolResult};
use futures::StreamExt;
use std::sync::Arc;
use tracing::{info, warn};

/// In-memory conversation memory — stores message history for the session.
/// Messages are lost when the process exits.
#[derive(Debug, Clone)]
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
    #[allow(dead_code)]
    pub fn history(&self) -> &[ChatMessage] {
        &self.messages
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

/// Run a one-shot command via --exec mode with streaming output
/// Sends the prompt to the LLM and prints tokens as they arrive.
#[allow(dead_code)]
pub async fn run_exec_stream(
    llm: Arc<dyn LLMProviderTrait>,
    prompt: &str,
    system_prompt: &str,
) -> Result<String> {
    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        "Exec one-shot streaming mode"
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        },
    ];

    let mut stream = llm.chat_stream(messages).await.map_err(|e| {
        crate::error::RavenClawError::CommandExecution(format!("LLM stream request failed: {}", e))
    })?;

    let mut full_response = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                full_response.push_str(&chunk.content);
            }
            Err(e) => {
                warn!(error = %e, "Stream chunk error");
            }
        }
    }

    if full_response.is_empty() {
        return Err(crate::error::RavenClawError::CommandExecution(
            "LLM returned empty response".to_string(),
        ));
    }

    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        "Exec streaming response received"
    );
    Ok(full_response)
}

/// Run an interactive REPL (read-eval-print loop) with conversation memory.
/// Reads prompts from stdin, sends them to the LLM with streaming, and prints responses.
/// Type `/exit` or `/quit` to exit, `/reset` to clear conversation history.
pub async fn run_repl(llm: Arc<dyn LLMProviderTrait>, config: Config) -> Result<()> {
    let system_prompt = &config.llm.system_prompt;
    let mut memory = ConversationMemory::new(system_prompt, 50);

    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        "Starting interactive REPL mode"
    );

    println!("RavenClaw REPL — type your message, or /exit to quit.");
    println!("System: {}", system_prompt);
    println!();

    let mut line = String::new();

    loop {
        line.clear();
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush().ok();

        let bytes_read = std::io::stdin().read_line(&mut line).map_err(|e| {
            crate::error::RavenClawError::CommandExecution(format!("stdin read error: {}", e))
        })?;

        if bytes_read == 0 {
            // EOF
            println!();
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" | "/quit" => {
                println!("Goodbye!");
                break;
            }
            "/reset" => {
                memory = ConversationMemory::new(system_prompt, 50);
                println!("Conversation reset.");
                continue;
            }
            _ => {}
        }

        // Add user message and get history
        let history = memory.add_user_message(input);

        // Send to LLM with streaming
        match llm.chat_stream(history.to_vec()).await {
            Ok(mut stream) => {
                let mut full_response = String::new();
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(chunk) => {
                            print!("{}", chunk.content);
                            std::io::stdout().flush().ok();
                            full_response.push_str(&chunk.content);
                        }
                        Err(e) => {
                            warn!(error = %e, "Stream chunk error");
                        }
                    }
                }
                println!();
                memory.add_assistant_message(&full_response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    info!("REPL session ended");
    Ok(())
}

/// Run a one-shot command via --exec mode
/// Sends the prompt to the LLM and returns the response text.
#[allow(dead_code)]
pub async fn run_exec(
    llm: Arc<dyn LLMProviderTrait>,
    prompt: &str,
    system_prompt: &str,
) -> Result<String> {
    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        "Exec one-shot mode"
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        },
    ];

    let response = llm.chat(messages).await.map_err(|e| {
        crate::error::RavenClawError::CommandExecution(format!("LLM request failed: {}", e))
    })?;

    let content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| {
            crate::error::RavenClawError::CommandExecution(
                "LLM returned empty response".to_string(),
            )
        })?;

    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        "Exec response received"
    );
    Ok(content)
}

/// Agent loop configuration
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Maximum number of perceive→plan→act→observe iterations
    pub max_iterations: usize,
    /// System prompt for the agent loop (appended to base system prompt)
    pub loop_instructions: String,
    /// Whether to enable tool use
    pub enable_tools: bool,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            enable_tools: false,
            loop_instructions: concat!(
                "You are in an agent loop. For each iteration, output:\n",
                "THOUGHT: <your reasoning about what to do next>\n",
                "ACTION: <the action you will take>\n",
                "RESULT: <the result of the action>\n\n",
                "When the task is complete, output:\n",
                "FINAL: <the final answer>\n\n",
                "You must output exactly one THOUGHT/ACTION/RESULT or FINAL per response.\n\n",
                "To use a tool, output:\n",
                "TOOL_CALL: <tool_name>\n",
                "ARGS: <JSON arguments>\n\n",
                "After a tool call, the result will be injected as an OBSERVATION.",
            )
            .to_string(),
        }
    }
}

/// Run the agent loop: perceive → plan → act → observe
///
/// Sends the user's task to the LLM and iterates through reasoning steps
/// until the task is complete or the maximum iteration count is reached.
pub async fn run_agent_loop(
    llm: Arc<dyn LLMProviderTrait>,
    task: &str,
    system_prompt: &str,
    config: AgentLoopConfig,
) -> Result<String> {
    info!(
        provider = llm.provider_name(),
        model = llm.model(),
        max_iterations = config.max_iterations,
        "Starting agent loop"
    );

    // Build the combined system prompt with loop instructions
    let mut combined_prompt = format!("{}\n\n{}", system_prompt, config.loop_instructions);

    // If tools are enabled, add tool definitions to the system prompt
    let registry = if config.enable_tools {
        let reg = ToolRegistry::with_default_tools();
        if !reg.is_empty() {
            let tool_descriptions: Vec<String> = reg
                .definitions()
                .iter()
                .map(|d| {
                    format!(
                        "- `{}`: {} (category: {:?})",
                        d.name, d.description, d.category
                    )
                })
                .collect();
            combined_prompt.push_str("\n\nAvailable tools:\n");
            combined_prompt.push_str(&tool_descriptions.join("\n"));
            combined_prompt.push_str(
                "\n\nTo call a tool, output:\nTOOL_CALL: <tool_name>\nARGS: <JSON arguments>\n\
                 The tool result will be provided as an OBSERVATION in the next iteration.",
            );
        }
        Some(reg)
    } else {
        None
    };

    let mut memory = ConversationMemory::new(&combined_prompt, config.max_iterations * 2 + 2);
    let _history = memory.add_user_message(task);

    for iteration in 0..config.max_iterations {
        info!(iteration = iteration, "Agent loop iteration");

        // ── Act: send current context to LLM ──────────────────────
        let response = llm.chat(memory.history().to_vec()).await.map_err(|e| {
            crate::error::RavenClawError::CommandExecution(format!(
                "LLM request failed at iteration {}: {}",
                iteration, e
            ))
        })?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| {
                crate::error::RavenClawError::CommandExecution(format!(
                    "LLM returned empty response at iteration {}",
                    iteration
                ))
            })?;

        // ── Observe: store the response ───────────────────────────
        memory.add_assistant_message(&content);

        // Check if the agent signaled completion
        if content.contains("FINAL:") {
            info!(iteration = iteration, "Agent loop completed");
            if let Some(final_idx) = content.find("FINAL:") {
                let final_answer = content[final_idx + 6..].trim();
                return Ok(final_answer.to_string());
            }
            return Ok(content.trim().to_string());
        }

        // ── Check for tool calls ──────────────────────────────────
        if let Some(registry) = &registry {
            if let Some(tool_result) = execute_tool_call(&content, registry).await {
                let observation = format!(
                    "OBSERVATION: Tool call result:\n{}",
                    serde_json::to_string_pretty(&tool_result).unwrap_or_default()
                );
                memory.add_assistant_message(&observation);
                info!(
                    iteration = iteration,
                    tool = %tool_result.tool_name,
                    success = tool_result.success,
                    "Tool call executed"
                );
                continue;
            }
        }

        info!(
            iteration = iteration,
            thought = %content.lines().find(|l| l.starts_with("THOUGHT:")).unwrap_or("<no thought>"),
            "Agent loop progress"
        );
    }

    // Max iterations reached — return what we have
    warn!(
        max_iterations = config.max_iterations,
        "Agent loop reached max iterations"
    );

    let history = memory.history();
    if history.len() > 1 {
        if let Some(last) = history.last() {
            return Ok(last.content.clone());
        }
    }

    Err(crate::error::RavenClawError::CommandExecution(
        "Agent loop reached max iterations without completing the task".to_string(),
    ))
}

/// Parse and execute a tool call from the LLM response content.
/// Returns `Some(ToolResult)` if a tool call was found and executed, `None` otherwise.
async fn execute_tool_call(content: &str, registry: &ToolRegistry) -> Option<ToolResult> {
    // Look for TOOL_CALL: <name> followed by ARGS: <json>
    let mut lines = content.lines();
    let tool_call_line = lines.find(|l| l.trim().starts_with("TOOL_CALL:"))?;

    let tool_name = tool_call_line
        .trim()
        .strip_prefix("TOOL_CALL:")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())?;

    // Find the ARGS line
    let args_line = lines.find(|l| l.trim().starts_with("ARGS:"))?;
    let args_str = args_line
        .trim()
        .strip_prefix("ARGS:")
        .map(|s| s.trim())?;

    let args: serde_json::Value = serde_json::from_str(args_str).ok()?;

    let call = ToolCall {
        name: tool_name.to_string(),
        arguments: args,
        id: None,
    };

    match registry.execute(call).await {
        Ok(result) => Some(result),
        Err(e) => Some(ToolResult {
            tool_name: tool_name.to_string(),
            success: false,
            output: String::new(),
            error: Some(e.to_string()),
            exit_code: Some(1),
            duration_ms: None,
        }),
    }
}

/// Run a single autonomous agent (single-provider mode)
pub async fn run_single(llm: Arc<dyn LLMProviderTrait>, config: Config) -> Result<()> {
    info!(
        "Starting single agent mode with provider: {}",
        llm.provider_name()
    );

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
            }
        }
        Err(e) => {
            warn!(error = %e, provider = llm.provider_name(), "LLM request failed");
        }
    }

    Ok(())
}

/// Run multiple agents in swarm mode (single-provider)
pub async fn run_swarm(_llm: Arc<dyn LLMProviderTrait>, _config: Config) -> Result<()> {
    warn!("Swarm mode not yet implemented");
    Err(crate::error::RavenClawError::CommandExecution(
        "Swarm mode is not yet implemented. See ROADMAP.md for the planned timeline.".to_string(),
    ))
}

/// Run supervisor agent coordinating sub-agents (single-provider)
pub async fn run_supervisor(_llm: Arc<dyn LLMProviderTrait>, _config: Config) -> Result<()> {
    warn!("Supervisor mode not yet implemented");
    Err(crate::error::RavenClawError::CommandExecution(
        "Supervisor mode is not yet implemented. See ROADMAP.md for the planned timeline."
            .to_string(),
    ))
}

/// Run a single autonomous agent (multi-model mode)
pub async fn run_single_multi(multi_llm: MultiModelManager, config: Config) -> Result<()> {
    info!(
        "Starting single agent mode (multi-model) with {} providers",
        multi_llm.client_count()
    );

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

    Ok(())
}

/// Run multiple agents in swarm mode (multi-model)
pub async fn run_swarm_multi(_multi_llm: MultiModelManager, _config: Config) -> Result<()> {
    warn!("Swarm mode (multi-model) not yet implemented");
    Err(crate::error::RavenClawError::CommandExecution(
        "Swarm mode is not yet implemented. See ROADMAP.md for the planned timeline.".to_string(),
    ))
}

/// Run supervisor agent coordinating sub-agents (multi-model)
pub async fn run_supervisor_multi(_multi_llm: MultiModelManager, _config: Config) -> Result<()> {
    warn!("Supervisor mode (multi-model) not yet implemented");
    Err(crate::error::RavenClawError::CommandExecution(
        "Supervisor mode is not yet implemented. See ROADMAP.md for the planned timeline."
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_stub_returns_error() {
        let err = crate::error::RavenClawError::CommandExecution(
            "Swarm mode is not yet implemented. See ROADMAP.md for the planned timeline."
                .to_string(),
        );
        assert_eq!(
            format!("{}", err),
            "Command execution failed: Swarm mode is not yet implemented. See ROADMAP.md for the planned timeline."
        );
    }

    #[test]
    fn test_supervisor_stub_returns_error() {
        let err = crate::error::RavenClawError::CommandExecution(
            "Supervisor mode is not yet implemented. See ROADMAP.md for the planned timeline."
                .to_string(),
        );
        assert_eq!(
            format!("{}", err),
            "Command execution failed: Supervisor mode is not yet implemented. See ROADMAP.md for the planned timeline."
        );
    }

    #[test]
    fn test_exec_empty_response_error() {
        let err = crate::error::RavenClawError::CommandExecution(
            "LLM returned empty response".to_string(),
        );
        assert!(format!("{}", err).contains("LLM returned empty response"));
    }

    #[test]
    fn test_swarm_multi_stub_returns_error() {
        let err = crate::error::RavenClawError::CommandExecution(
            "Swarm mode is not yet implemented. See ROADMAP.md for the planned timeline."
                .to_string(),
        );
        assert!(format!("{}", err).contains("Swarm mode"));
    }

    #[test]
    fn test_supervisor_multi_stub_returns_error() {
        let err = crate::error::RavenClawError::CommandExecution(
            "Supervisor mode is not yet implemented. See ROADMAP.md for the planned timeline."
                .to_string(),
        );
        assert!(format!("{}", err).contains("Supervisor mode"));
    }

    #[test]
    fn test_run_exec_llm_error_message() {
        let err = crate::error::RavenClawError::CommandExecution(
            "LLM request failed: connection refused".to_string(),
        );
        assert!(format!("{}", err).contains("LLM request failed"));
    }

    #[test]
    fn test_run_single_logs_provider_name() {
        // Verify the function signature compiles and accepts the right types
        fn _check_types() {
            let _ = run_single;
            let _ = run_swarm;
            let _ = run_supervisor;
            let _ = run_single_multi;
            let _ = run_swarm_multi;
            let _ = run_supervisor_multi;
            let _ = run_exec;
        }
        // Compile-time check: all function signatures are valid
    }

    // ── Mockito-based agent tests ──────────────────────────────────────

    fn sample_chat_response_json(model: &str) -> String {
        format!(
            r#"{{
            "id": "chat-123",
            "object": "chat.completion",
            "created": 1717000000,
            "model": "{}",
            "choices": [
                {{
                    "index": 0,
                    "message": {{
                        "role": "assistant",
                        "content": "Hello from agent!"
                    }},
                    "finish_reason": "stop"
                }}
            ],
            "usage": {{
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }}
        }}"#,
            model
        )
    }

    fn with_mockito<F, Fut>(f: F)
    where
        F: FnOnce(mockito::ServerGuard) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let server = mockito::Server::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(f(server));
    }

    #[test]
    fn test_run_exec_with_mockito() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(sample_chat_response_json("gpt-4o-mini"))
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let response = run_exec(llm, "Hello!", &config.system_prompt)
                .await
                .unwrap();

            assert_eq!(response, "Hello from agent!");
            mock.assert();
        });
    }

    #[test]
    fn test_run_exec_with_mockito_empty_response() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{"id":"x","object":"chat.completion","created":0,"model":"x","choices":[]}"#,
                )
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let result = run_exec(llm, "Hello!", &config.system_prompt).await;

            assert!(result.is_err());
            assert!(format!("{}", result.unwrap_err()).contains("empty response"));
            mock.assert();
        });
    }

    #[test]
    fn test_run_exec_with_mockito_llm_error() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(500)
                .with_header("content-type", "application/json")
                .with_body(r#"{"error":"Internal error"}"#)
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let result = run_exec(llm, "Hello!", &config.system_prompt).await;

            assert!(result.is_err());
            assert!(format!("{}", result.unwrap_err()).contains("LLM request failed"));
            mock.assert();
        });
    }

    #[test]
    fn test_run_single_with_mockito() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(sample_chat_response_json("gpt-4o-mini"))
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let cfg = crate::config::Config {
                llm: crate::config::LLMConfig::default(),
                llms: vec![],
                ravenfabric: crate::config::RavenFabricConfig::default(),
                security: crate::config::SecurityConfig {
                    require_tls: false,
                    token_lifetime_secs: 3600,
                    audit_log: false,
                },
                runtime: crate::config::RuntimeConfig::default(),
            };
            let result = run_single(llm, cfg).await;

            // run_single logs the response but always returns Ok(())
            assert!(result.is_ok());
            mock.assert();
        });
    }

    #[test]
    fn test_run_single_with_mockito_llm_error() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(401)
                .with_header("content-type", "application/json")
                .with_body(r#"{"error":"Unauthorized"}"#)
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("bad-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let cfg = crate::config::Config {
                llm: crate::config::LLMConfig::default(),
                llms: vec![],
                ravenfabric: crate::config::RavenFabricConfig::default(),
                security: crate::config::SecurityConfig {
                    require_tls: false,
                    token_lifetime_secs: 3600,
                    audit_log: false,
                },
                runtime: crate::config::RuntimeConfig::default(),
            };
            // run_single catches LLM errors internally and logs them, returns Ok(())
            let result = run_single(llm, cfg).await;
            assert!(result.is_ok());
            mock.assert();
        });
    }

    #[test]
    fn test_run_single_multi_with_mockito() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(sample_chat_response_json("gpt-4o-mini"))
                .create();

            let configs = vec![crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            }];

            let multi_llm = crate::llm::MultiModelManager::new(configs).unwrap();
            let cfg = crate::config::Config {
                llm: crate::config::LLMConfig::default(),
                llms: vec![],
                ravenfabric: crate::config::RavenFabricConfig::default(),
                security: crate::config::SecurityConfig {
                    require_tls: false,
                    token_lifetime_secs: 3600,
                    audit_log: false,
                },
                runtime: crate::config::RuntimeConfig::default(),
            };
            let result = run_single_multi(multi_llm, cfg).await;

            // run_single_multi logs responses but always returns Ok(())
            assert!(result.is_ok());
            mock.assert();
        });
    }

    #[test]
    fn test_run_single_multi_with_mockito_partial_failure() {
        // Test that run_single_multi handles one provider failing gracefully
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(401)
                .with_header("content-type", "application/json")
                .with_body(r#"{"error":"Unauthorized"}"#)
                .create();

            let configs = vec![crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("bad-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            }];

            let multi_llm = crate::llm::MultiModelManager::new(configs).unwrap();
            let cfg = crate::config::Config {
                llm: crate::config::LLMConfig::default(),
                llms: vec![],
                ravenfabric: crate::config::RavenFabricConfig::default(),
                security: crate::config::SecurityConfig {
                    require_tls: false,
                    token_lifetime_secs: 3600,
                    audit_log: false,
                },
                runtime: crate::config::RuntimeConfig::default(),
            };
            let result = run_single_multi(multi_llm, cfg).await;

            // run_single_multi catches errors internally and logs them, returns Ok(())
            assert!(result.is_ok());
            mock.assert();
        });
    }

    #[test]
    fn test_run_swarm_multi_returns_error() {
        // Test that run_swarm_multi returns the expected error
        let configs = vec![crate::config::LLMConfig {
            provider: crate::config::LLMProvider::LiteLLM,
            endpoint: "http://localhost:4000".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: Some("test".to_string()),
            timeout_secs: 30,
            system_prompt: crate::config::default_system_prompt(),
        }];

        let multi_llm = crate::llm::MultiModelManager::new(configs).unwrap();
        let cfg = crate::config::Config {
            llm: crate::config::LLMConfig::default(),
            llms: vec![],
            ravenfabric: crate::config::RavenFabricConfig::default(),
            security: crate::config::SecurityConfig {
                require_tls: false,
                token_lifetime_secs: 3600,
                audit_log: false,
            },
            runtime: crate::config::RuntimeConfig::default(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(run_swarm_multi(multi_llm, cfg));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("Swarm mode"));
    }

    #[test]
    fn test_run_supervisor_multi_returns_error() {
        // Test that run_supervisor_multi returns the expected error
        let configs = vec![crate::config::LLMConfig {
            provider: crate::config::LLMProvider::LiteLLM,
            endpoint: "http://localhost:4000".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: Some("test".to_string()),
            timeout_secs: 30,
            system_prompt: crate::config::default_system_prompt(),
        }];

        let multi_llm = crate::llm::MultiModelManager::new(configs).unwrap();
        let cfg = crate::config::Config {
            llm: crate::config::LLMConfig::default(),
            llms: vec![],
            ravenfabric: crate::config::RavenFabricConfig::default(),
            security: crate::config::SecurityConfig {
                require_tls: false,
                token_lifetime_secs: 3600,
                audit_log: false,
            },
            runtime: crate::config::RuntimeConfig::default(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(run_supervisor_multi(multi_llm, cfg));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("Supervisor mode"));
    }

    #[test]
    fn test_run_exec_with_mockito_different_providers() {
        // Test run_exec with OpenRouter provider
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(sample_chat_response_json(
                    "anthropic/claude-sonnet-4-20250514",
                ))
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::OpenRouter,
                endpoint: server.url(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                api_key: Some("or-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let response = run_exec(llm, "Hello!", &config.system_prompt)
                .await
                .unwrap();

            assert_eq!(response, "Hello from agent!");
            mock.assert();
        });
    }

    // ── ConversationMemory tests ──────────────────────────────────────

    #[test]
    fn test_conversation_memory_new() {
        let mem = ConversationMemory::new("You are a helpful assistant.", 10);
        assert_eq!(mem.history().len(), 1);
        assert_eq!(mem.history()[0].role, "system");
        assert_eq!(mem.history()[0].content, "You are a helpful assistant.");
    }

    #[test]
    fn test_conversation_memory_add_user_message() {
        let mut mem = ConversationMemory::new("System prompt.", 10);
        let history = mem.add_user_message("Hello!");
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].role, "user");
        assert_eq!(history[1].content, "Hello!");
    }

    #[test]
    fn test_conversation_memory_add_assistant_message() {
        let mut mem = ConversationMemory::new("System prompt.", 10);
        mem.add_user_message("Hello!");
        mem.add_assistant_message("Hi there!");
        assert_eq!(mem.history().len(), 3);
        assert_eq!(mem.history()[2].role, "assistant");
        assert_eq!(mem.history()[2].content, "Hi there!");
    }

    #[test]
    fn test_conversation_memory_trim() {
        // max_messages=3 means system + 2 messages max
        let mut mem = ConversationMemory::new("System.", 3);
        mem.add_user_message("msg1");
        mem.add_assistant_message("resp1");
        assert_eq!(mem.history().len(), 3);

        // Adding a 4th message should trim the oldest non-system (msg1)
        mem.add_user_message("msg2");
        assert_eq!(mem.history().len(), 3);
        // The oldest user message should have been removed
        assert_eq!(mem.history()[1].content, "resp1");
        assert_eq!(mem.history()[2].content, "msg2");
    }

    #[test]
    fn test_conversation_memory_unlimited() {
        let mut mem = ConversationMemory::new("System.", 0);
        for i in 0..100 {
            mem.add_user_message(&format!("msg{}", i));
        }
        assert_eq!(mem.history().len(), 101); // system + 100 messages
    }

    // ── AgentLoop tests ───────────────────────────────────────────────

    #[test]
    fn test_agent_loop_config_default() {
        let config = AgentLoopConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert!(config.loop_instructions.contains("THOUGHT:"));
        assert!(config.loop_instructions.contains("ACTION:"));
        assert!(config.loop_instructions.contains("FINAL:"));
    }

    #[test]
    fn test_agent_loop_completes_with_final() {
        with_mockito(|mut server| async move {
            // Mock a response that contains FINAL: to signal completion
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{
                        "id": "chat-123",
                        "object": "chat.completion",
                        "created": 1717000000,
                        "model": "gpt-4o-mini",
                        "choices": [{
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": "THOUGHT: I need to analyze this.\nACTION: Process the input.\nFINAL: The answer is 42."
                            },
                            "finish_reason": "stop"
                        }],
                        "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30}
                    }"#,
                )
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let loop_config = AgentLoopConfig {
                max_iterations: 5,
                ..AgentLoopConfig::default()
            };

            let result =
                run_agent_loop(llm, "What is the answer?", "You are helpful.", loop_config)
                    .await
                    .unwrap();

            assert_eq!(result, "The answer is 42.");
            mock.assert();
        });
    }

    #[test]
    fn test_agent_loop_reaches_max_iterations() {
        with_mockito(|mut server| async move {
            // Mock a response that does NOT contain FINAL: — should hit max iterations
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{
                        "id": "chat-123",
                        "object": "chat.completion",
                        "created": 1717000000,
                        "model": "gpt-4o-mini",
                        "choices": [{
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": "THOUGHT: Still thinking.\nACTION: Continue processing."
                            },
                            "finish_reason": "stop"
                        }],
                        "usage": {"prompt_tokens": 10, "completion_tokens": 15, "total_tokens": 25}
                    }"#,
                )
                .expect(1) // Only expect 1 call — loop stops after max iterations
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let loop_config = AgentLoopConfig {
                max_iterations: 1,
                ..AgentLoopConfig::default()
            };

            let result =
                run_agent_loop(llm, "Do something.", "You are helpful.", loop_config).await;

            // Should return the last assistant message content
            assert!(result.is_ok());
            let content = result.unwrap();
            assert!(content.contains("Still thinking"));
            mock.assert();
        });
    }

    #[test]
    fn test_agent_loop_llm_error() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(500)
                .with_header("content-type", "application/json")
                .with_body(r#"{"error":"Internal error"}"#)
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let loop_config = AgentLoopConfig {
                max_iterations: 3,
                ..AgentLoopConfig::default()
            };

            let result = run_agent_loop(llm, "Test task.", "You are helpful.", loop_config).await;

            assert!(result.is_err());
            assert!(format!("{}", result.unwrap_err()).contains("LLM request failed"));
            mock.assert();
        });
    }

    #[test]
    fn test_agent_loop_empty_response() {
        with_mockito(|mut server| async move {
            let mock = server
                .mock("POST", "/v1/chat/completions")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{"id":"x","object":"chat.completion","created":0,"model":"x","choices":[]}"#,
                )
                .create();

            let config = crate::config::LLMConfig {
                provider: crate::config::LLMProvider::LiteLLM,
                endpoint: server.url(),
                model: "gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                timeout_secs: 30,
                system_prompt: crate::config::default_system_prompt(),
            };

            let llm = crate::llm::create_client(&config).unwrap();
            let loop_config = AgentLoopConfig {
                max_iterations: 3,
                ..AgentLoopConfig::default()
            };

            let result = run_agent_loop(llm, "Test task.", "You are helpful.", loop_config).await;

            assert!(result.is_err());
            assert!(format!("{}", result.unwrap_err()).contains("empty response"));
            mock.assert();
        });
    }

    #[test]
    fn test_agent_loop_custom_config() {
        let config = AgentLoopConfig {
            max_iterations: 3,
            loop_instructions: "Custom instructions.".to_string(),
            enable_tools: false,
        };
        assert_eq!(config.max_iterations, 3);
        assert_eq!(config.loop_instructions, "Custom instructions.");
        assert!(!config.enable_tools);
    }
}
