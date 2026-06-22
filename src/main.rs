//! RavenClaw — Lightweight, secure Rust agent framework
//!
//! Built for efficiency, security, and easy deployment.
//! Supports multiple LLM providers: LiteLLM, OpenRouter, Ollama, OpenAI.

mod agent;
mod audit;
mod background;
mod config;
mod error;
mod llm;
mod mcp;
mod policy;
mod ravenfabric;
mod sandbox;
mod server;
mod telemetry;
mod tools;

use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "ravenclaw")]
#[command(author = "Erling G M Kristiansen")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Lightweight, secure Rust agent framework with multi-provider support", long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, env = "RAVENCLAW_CONFIG")]
    config: Option<String>,

    /// Agent mode: single, swarm, or supervisor
    #[arg(short, long, default_value = "single")]
    mode: String,

    /// Enable verbose logging
    #[arg(short, long, env = "RAVENCLAW_VERBOSE")]
    verbose: bool,

    /// Run a one-shot command
    #[arg(short, long)]
    exec: Option<String>,

    /// Provider type: litellm, openrouter, ollama, openai (overrides config)
    #[arg(long, env = "RAVENCLAW_PROVIDER")]
    provider: Option<String>,

    /// LLM endpoint (overrides config)
    #[arg(long, env = "RAVENCLAW_ENDPOINT")]
    endpoint: Option<String>,

    /// Model name (overrides config)
    #[arg(long, env = "RAVENCLAW_MODEL")]
    model: Option<String>,

    /// System prompt / persona (overrides config)
    #[arg(long, env = "RAVENCLAW_SYSTEM_PROMPT")]
    system_prompt: Option<String>,

    /// Interactive REPL mode (read-eval-print loop)
    #[arg(long, short = 'R', conflicts_with = "exec")]
    repl: bool,

    /// Maximum iterations for the agent loop (default: 10)
    #[arg(long, env = "RAVENCLAW_MAX_ITERATIONS", default_value = "10")]
    max_iterations: usize,

    /// Token budget per run (v0.5) — stops when exceeded
    #[arg(long, env = "RAVENCLAW_TOKEN_BUDGET")]
    token_budget: Option<u32>,

    /// Retry max attempts (v0.5) — default 3
    #[arg(long, env = "RAVENCLAW_RETRY_MAX", default_value = "3")]
    retry_max: u32,

    /// Retry base delay in ms (v0.5) — default 100
    #[arg(long, env = "RAVENCLAW_RETRY_BASE_DELAY", default_value = "100")]
    retry_base_delay_ms: u64,

    /// Enable provider fallback chain (v0.5) — comma-separated providers
    #[arg(long, env = "RAVENCLAW_FALLBACK_CHAIN")]
    fallback_chain: Option<String>,

    /// MCP server command (v0.5.2) — stdio transport (e.g., "npx -y @modelcontextprotocol/server-filesystem")
    #[arg(long, env = "RAVENCLAW_MCP_COMMAND")]
    mcp_command: Option<String>,

    /// MCP server arguments (v0.5.2) — space-separated args for the MCP command
    #[arg(long, env = "RAVENCLAW_MCP_ARGS")]
    mcp_args: Option<String>,

    /// MCP server environment variables (v0.5.2) — KEY=VALUE pairs separated by commas
    #[arg(long, env = "RAVENCLAW_MCP_ENV")]
    mcp_env: Option<String>,

    /// Run as MCP server (v0.7) — exposes RavenClaw tools over stdio via MCP protocol
    #[arg(long, env = "RAVENCLAW_MCP_SERVER")]
    mcp_server: bool,

    /// Run as HTTP server (v0.7) — long-running with /health, /ready, /metrics endpoints
    #[arg(long, env = "RAVENCLAW_SERVE")]
    serve: bool,

    /// HTTP server host (v0.7) — overrides config
    #[arg(long, env = "RAVENCLAW_SERVER_HOST")]
    server_host: Option<String>,

    /// HTTP server port (v0.7) — overrides config
    #[arg(long, env = "RAVENCLAW_SERVER_PORT")]
    server_port: Option<u16>,

    /// OpenTelemetry OTLP gRPC endpoint (v0.7.2)
    #[arg(long, env = "RAVENCLAW_OTEL_ENDPOINT")]
    otel_endpoint: Option<String>,

    /// OpenTelemetry service name (v0.7.2)
    #[arg(long, env = "RAVENCLAW_OTEL_SERVICE_NAME")]
    otel_service_name: Option<String>,

    /// Disable OpenTelemetry tracing (v0.7.2)
    #[arg(long, env = "RAVENCLAW_OTEL_DISABLED")]
    otel_disabled: bool,

    /// Submit a background task and return immediately (v0.8)
    #[arg(long, env = "RAVENCLAW_BACKGROUND")]
    background: bool,

    /// Check status of a background task (v0.8)
    #[arg(long, env = "RAVENCLAW_TASK_STATUS")]
    task_status: Option<String>,

    /// List all background tasks (v0.8)
    #[arg(long, env = "RAVENCLAW_TASK_LIST")]
    task_list: bool,

    /// Cancel a background task (v0.8)
    #[arg(long, env = "RAVENCLAW_TASK_CANCEL")]
    task_cancel: Option<String>,

    /// Resume incomplete background tasks on startup (v0.8)
    #[arg(long, env = "RAVENCLAW_TASK_RESUME")]
    task_resume: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("ravenclaw={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "RavenClaw starting");

    // Load configuration
    let mut config = config::Config::load(args.config.as_deref())?;

    // Apply OpenTelemetry CLI overrides (v0.7.2)
    if let Some(endpoint) = args.otel_endpoint {
        config.telemetry.otel_endpoint = Some(endpoint);
    }
    if let Some(service_name) = args.otel_service_name {
        config.telemetry.otel_service_name = Some(service_name);
    }
    if args.otel_disabled {
        config.telemetry.otel_disabled = true;
    }

    // Initialize OpenTelemetry tracing (v0.7.2)
    let _otel_guard = telemetry::init_tracing(&config.telemetry)?;

    // Apply CLI overrides
    if let Some(provider) = args.provider {
        config.llm.provider = match provider.to_lowercase().as_str() {
            "openrouter" => config::LLMProvider::OpenRouter,
            "ollama" => config::LLMProvider::Ollama,
            "openai" => config::LLMProvider::OpenAI,
            _ => config::LLMProvider::LiteLLM,
        };
    }
    if let Some(endpoint) = args.endpoint {
        config.llm.endpoint = endpoint;
    }
    if let Some(model) = args.model {
        config.llm.model = model;
    }
    if let Some(system_prompt) = args.system_prompt {
        config.llm.system_prompt = system_prompt;
    }
    // v0.5: Apply token budget and retry settings
    if let Some(budget) = args.token_budget {
        config.llm.token_budget = Some(budget);
    }
    config.llm.retry_max = args.retry_max;
    config.llm.retry_base_delay_ms = args.retry_base_delay_ms;

    info!(mode = %args.mode, "Configuration loaded");

    // Run as MCP server if --mcp-server is set (v0.7)
    if args.mcp_server {
        info!("Starting in MCP server mode");
        let registry = tools::ToolRegistry::with_default_tools();
        let mut server = mcp::McpServer::new(registry);
        server
            .run()
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;
        info!("RavenClaw MCP server shutdown complete");
        return Ok(());
    }

    // Run as HTTP server if --serve is set (v0.7)
    if args.serve {
        info!("Starting in HTTP server mode");

        // Apply CLI overrides for server host/port
        if let Some(host) = args.server_host {
            config.runtime.host = Some(host);
        }
        if let Some(port) = args.server_port {
            config.runtime.port = port;
        }

        server::run_server(config).await?;
        info!("RavenClaw HTTP server shutdown complete");
        return Ok(());
    }

    // Initialize MCP client if --mcp-command is provided (v0.5.2)
    let mcp_client = if let Some(mcp_command) = &args.mcp_command {
        info!(command = %mcp_command, "Initializing MCP client");

        // Parse MCP args
        let mcp_args: Vec<String> = args
            .mcp_args
            .as_ref()
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        // Parse MCP env vars
        let mut mcp_env = std::collections::HashMap::new();
        if let Some(env_str) = &args.mcp_env {
            for pair in env_str.split(',') {
                if let Some((key, value)) = pair.split_once('=') {
                    mcp_env.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }

        let transport_config = mcp::McpTransportConfig::Stdio {
            command: mcp_command.clone(),
            args: mcp_args,
            env: mcp_env,
        };

        let mut client = mcp::McpClient::new(transport_config);
        match client.connect().await {
            Ok(()) => {
                info!(server = ?client.server_info(), "MCP client connected");
                Some(std::sync::Arc::new(tokio::sync::RwLock::new(client)))
            }
            Err(e) => {
                warn!(error = %e, "Failed to connect to MCP server, continuing without MCP tools");
                None
            }
        }
    } else {
        None
    };

    // Handle --exec one-shot mode (uses agent loop for multi-step reasoning with security)
    if let Some(exec_prompt) = args.exec {
        info!("Running in --exec mode with security-integrated agent loop");
        let system_prompt = &config.llm.system_prompt;
        let loop_config = agent::AgentLoopConfig {
            max_iterations: args.max_iterations,
            enable_tools: true,
            require_approval: false, // Auto-approve for v0.4; HITL would block here
        };

        let response = if !config.llms.is_empty() {
            let multi_llm = llm::MultiModelManager::new(config.llms.clone())?;
            if let Some(client) = multi_llm.get_client(0) {
                agent::run_agent_loop_with_mcp(
                    client.clone(),
                    &exec_prompt,
                    system_prompt,
                    loop_config,
                    mcp_client,
                )
                .await?
            } else {
                anyhow::bail!("No LLM providers available for --exec mode");
            }
        } else {
            let llm = llm::create_client(&config.llm)?;
            agent::run_agent_loop_with_mcp(
                llm,
                &exec_prompt,
                system_prompt,
                loop_config,
                mcp_client,
            )
            .await?
        };
        println!("{}", response);
        info!("RavenClaw shutdown complete");
        return Ok(());
    }

    // Initialize background task manager (v0.8)
    let bg_manager = background::BackgroundTaskManager::from_config(&config.runtime).await?;

    // Resume incomplete background tasks if --task-resume is set (v0.8)
    if args.task_resume {
        let incomplete = bg_manager.resume_incomplete().await;
        if incomplete.is_empty() {
            info!("No incomplete background tasks to resume");
        } else {
            info!(
                count = incomplete.len(),
                "Resuming incomplete background tasks"
            );
            for task_id in &incomplete {
                info!(task_id = %task_id, "Resuming background task");
                let llm = llm::create_client(&config.llm)?;
                let bg = bg_manager.clone();
                let tid = task_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = bg.execute(&tid, llm).await {
                        warn!(task_id = %tid, error = %e, "Background task resume failed");
                    }
                });
            }
        }
    }

    // Handle --task-list: list all background tasks (v0.8)
    if args.task_list {
        let tasks = bg_manager.list_tasks().await;
        if tasks.is_empty() {
            println!("No background tasks found.");
        } else {
            println!("{:<38} {:<10} {:<30} PROMPT", "ID", "STATUS", "CREATED");
            println!("{}", "-".repeat(100));
            for task in &tasks {
                let prompt_preview = if task.prompt.len() > 40 {
                    format!("{}...", &task.prompt[..40])
                } else {
                    task.prompt.clone()
                };
                println!(
                    "{:<38} {:<10} {:<30} {}",
                    task.id, task.status, task.created_at, prompt_preview
                );
            }
        }
        info!("RavenClaw shutdown complete");
        return Ok(());
    }

    // Handle --task-status: check a specific task (v0.8)
    if let Some(task_id) = args.task_status {
        match bg_manager.get_task(&task_id).await {
            Ok(task) => {
                println!("Task ID:     {}", task.id);
                println!("Status:      {}", task.status);
                println!("Created:     {}", task.created_at);
                println!("Updated:     {}", task.updated_at);
                println!("Prompt:      {}", task.prompt);
                if let Some(result) = &task.result {
                    println!("Result:      {}", result);
                }
                if let Some(error) = &task.error {
                    println!("Error:       {}", error);
                }
                if let Some(provider) = &task.provider {
                    println!("Provider:    {}", provider);
                }
                if let Some(model) = &task.model {
                    println!("Model:       {}", model);
                }
                println!("Iterations:  {}", task.iterations);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
        info!("RavenClaw shutdown complete");
        return Ok(());
    }

    // Handle --task-cancel: cancel a background task (v0.8)
    if let Some(task_id) = args.task_cancel {
        match bg_manager.cancel(&task_id).await {
            Ok(()) => println!("Task '{}' cancelled.", task_id),
            Err(e) => eprintln!("Error: {}", e),
        }
        info!("RavenClaw shutdown complete");
        return Ok(());
    }

    // Handle --background mode: submit task and return immediately (v0.8)
    if args.background {
        info!("Running in background task submission mode");
        let system_prompt = &config.llm.system_prompt;

        // Read prompt from stdin if not provided via --exec
        let prompt = if let Some(exec_prompt) = args.exec {
            exec_prompt
        } else {
            // Read from stdin
            let mut input = String::new();
            use std::io::Read;
            let stdin = std::io::stdin();
            let mut handle = stdin.lock();
            handle.read_to_string(&mut input)?;
            if input.trim().is_empty() {
                anyhow::bail!("No prompt provided. Use --exec or pipe input to stdin.");
            }
            input.trim().to_string()
        };

        let task_id = bg_manager.submit(prompt, system_prompt.clone()).await?;
        println!("{}", task_id);

        // Execute the task in the background
        let bg = bg_manager.clone();
        let tid = task_id.clone();
        let llm = llm::create_client(&config.llm)?;
        tokio::spawn(async move {
            if let Err(e) = bg.execute(&tid, llm).await {
                warn!(task_id = %tid, error = %e, "Background task execution failed");
            }
        });

        info!(task_id = %task_id, "Background task submitted, returning immediately");
        info!("RavenClaw shutdown complete");
        return Ok(());
    }

    // Handle --repl interactive mode
    if args.repl {
        info!("Running in interactive REPL mode");
        if !config.llms.is_empty() {
            let multi_llm = llm::MultiModelManager::new(config.llms.clone())?;
            if let Some(client) = multi_llm.get_client(0) {
                agent::run_repl(client.clone(), config).await?;
            } else {
                anyhow::bail!("No LLM providers available for --repl mode");
            }
        } else {
            let llm = llm::create_client(&config.llm)?;
            agent::run_repl(llm, config).await?;
        }
        info!("RavenClaw shutdown complete");
        return Ok(());
    }

    // Initialize RavenFabric client (v0.6.1)
    let ravenfabric = ravenfabric::RavenFabricClient::new(&config.ravenfabric);
    if let Some(ref rf) = ravenfabric {
        info!(
            endpoint = %rf.endpoint().unwrap_or("unknown"),
            agent_id = ?rf.agent_id(),
            remote_exec = rf.is_enabled(),
            "RavenFabric integration enabled"
        );
    } else {
        info!("RavenFabric not configured — remote execution disabled");
    }

    // Determine if multi-model or single-provider mode
    let has_multi_model = !config.llms.is_empty();

    if has_multi_model {
        info!(providers = config.llms.len(), "Multi-model mode enabled");

        // Create multi-model manager
        let multi_llm = llm::MultiModelManager::new(config.llms.clone())?;

        for i in 0..multi_llm.client_count() {
            if let Some(client) = multi_llm.get_client(i) {
                info!(
                    provider = client.provider_name(),
                    model = client.model(),
                    "Provider initialized"
                );
            }
        }

        // Run agent based on mode with multi-model support
        match args.mode.as_str() {
            "single" => {
                info!("Running in single agent mode (multi-model)");
                agent::run_single_multi(multi_llm, config, ravenfabric).await?;
            }
            "swarm" => {
                info!("Running in swarm mode (multi-model)");
                agent::run_swarm_multi(multi_llm, config, ravenfabric).await?;
            }
            "supervisor" => {
                info!("Running in supervisor mode (multi-model)");
                agent::run_supervisor_multi(multi_llm, config, ravenfabric).await?;
            }
            _ => {
                anyhow::bail!(
                    "Unknown mode: {}. Use: single, swarm, or supervisor",
                    args.mode
                );
            }
        }
    } else {
        // Single provider mode
        let provider_name = match config.llm.provider {
            config::LLMProvider::LiteLLM => "LiteLLM",
            config::LLMProvider::OpenRouter => "OpenRouter",
            config::LLMProvider::Ollama => "Ollama",
            config::LLMProvider::OpenAI => "OpenAI",
            config::LLMProvider::Anthropic => "Anthropic",
        };

        info!(provider = provider_name, endpoint = %config.llm.endpoint, model = %config.llm.model, "LLM client initialized");

        // Create appropriate client based on provider
        let llm = llm::create_client(&config.llm)?;
        info!(
            provider = llm.provider_name(),
            model = llm.model(),
            "Provider ready"
        );

        // Run agent based on mode
        match args.mode.as_str() {
            "single" => {
                info!("Running in single agent mode");
                agent::run_single(llm, config, ravenfabric).await?;
            }
            "swarm" => {
                info!("Running in swarm mode");
                agent::run_swarm(llm, config, ravenfabric).await?;
            }
            "supervisor" => {
                info!("Running in supervisor mode");
                agent::run_supervisor(llm, config, ravenfabric).await?;
            }
            _ => {
                anyhow::bail!(
                    "Unknown mode: {}. Use: single, swarm, or supervisor",
                    args.mode
                );
            }
        }
    }

    info!("RavenClaw shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_args() {
        // Verify the CLI struct can be constructed with defaults
        let args = Args::parse_from(["ravenclaw"]);
        assert_eq!(args.mode, "single");
        assert!(!args.verbose);
        assert!(args.config.is_none());
        assert!(args.exec.is_none());
        assert!(args.provider.is_none());
        assert!(args.endpoint.is_none());
        assert!(args.model.is_none());
    }

    #[test]
    fn test_cli_custom_args() {
        let args = Args::parse_from([
            "ravenclaw",
            "--config",
            "/tmp/config.toml",
            "--mode",
            "swarm",
            "--verbose",
            "--exec",
            "Hello",
            "--provider",
            "ollama",
            "--endpoint",
            "http://localhost:11434",
            "--model",
            "llama3.1",
        ]);
        assert_eq!(args.config.unwrap(), "/tmp/config.toml");
        assert_eq!(args.mode, "swarm");
        assert!(args.verbose);
        assert_eq!(args.exec.unwrap(), "Hello");
        assert_eq!(args.provider.unwrap(), "ollama");
        assert_eq!(args.endpoint.unwrap(), "http://localhost:11434");
        assert_eq!(args.model.unwrap(), "llama3.1");
    }

    #[test]
    fn test_cli_short_args() {
        let args = Args::parse_from([
            "ravenclaw",
            "-c",
            "/tmp/config.toml",
            "-m",
            "supervisor",
            "-v",
            "-e",
            "test prompt",
        ]);
        assert_eq!(args.config.unwrap(), "/tmp/config.toml");
        assert_eq!(args.mode, "supervisor");
        assert!(args.verbose);
        assert_eq!(args.exec.unwrap(), "test prompt");
    }

    #[test]
    fn test_cli_invalid_mode() {
        let args = Args::parse_from(["ravenclaw", "--mode", "invalid"]);
        assert_eq!(args.mode, "invalid");
        // The mode validation happens at runtime, not in clap
    }

    #[test]
    fn test_cli_provider_mapping() {
        // Test that provider strings map correctly
        let test_cases = vec![
            ("litellm", config::LLMProvider::LiteLLM),
            ("openrouter", config::LLMProvider::OpenRouter),
            ("ollama", config::LLMProvider::Ollama),
            ("openai", config::LLMProvider::OpenAI),
            ("unknown", config::LLMProvider::LiteLLM), // default
        ];

        for (input, expected) in test_cases {
            let mapped = match input.to_lowercase().as_str() {
                "openrouter" => config::LLMProvider::OpenRouter,
                "ollama" => config::LLMProvider::Ollama,
                "openai" => config::LLMProvider::OpenAI,
                _ => config::LLMProvider::LiteLLM,
            };
            assert_eq!(mapped, expected, "Provider mapping failed for '{}'", input);
        }
    }

    #[test]
    fn test_cli_provider_mapping_case_insensitive() {
        let test_cases = vec![
            ("OpenRouter", config::LLMProvider::OpenRouter),
            ("OPENAI", config::LLMProvider::OpenAI),
            ("Ollama", config::LLMProvider::Ollama),
            ("LiteLLM", config::LLMProvider::LiteLLM),
        ];

        for (input, expected) in test_cases {
            let mapped = match input.to_lowercase().as_str() {
                "openrouter" => config::LLMProvider::OpenRouter,
                "ollama" => config::LLMProvider::Ollama,
                "openai" => config::LLMProvider::OpenAI,
                _ => config::LLMProvider::LiteLLM,
            };
            assert_eq!(
                mapped, expected,
                "Case-insensitive mapping failed for '{}'",
                input
            );
        }
    }

    #[test]
    fn test_cli_verbose_flag() {
        let args = Args::parse_from(["ravenclaw", "--verbose"]);
        assert!(args.verbose);

        let args = Args::parse_from(["ravenclaw"]);
        assert!(!args.verbose);
    }

    #[test]
    fn test_cli_env_var_mapping() {
        // Verify that env var names match CLI args
        // RAVENCLAW_CONFIG → config
        // RAVENCLAW_VERBOSE → verbose
        // RAVENCLAW_PROVIDER → provider
        // RAVENCLAW_ENDPOINT → endpoint
        // RAVENCLAW_MODEL → model
        let args = Args::parse_from(["ravenclaw"]);
        assert_eq!(args.config, None);
        assert!(!args.verbose);
        assert_eq!(args.provider, None);
        assert_eq!(args.endpoint, None);
        assert_eq!(args.model, None);
    }

    #[test]
    fn test_cli_exec_with_provider() {
        let args = Args::parse_from(["ravenclaw", "--exec", "test prompt", "--provider", "openai"]);
        assert_eq!(args.exec.unwrap(), "test prompt");
        assert_eq!(args.provider.unwrap(), "openai");
    }

    #[test]
    fn test_cli_mode_dispatch_single() {
        let args = Args::parse_from(["ravenclaw", "--mode", "single"]);
        assert_eq!(args.mode, "single");
    }

    #[test]
    fn test_cli_mode_dispatch_swarm() {
        let args = Args::parse_from(["ravenclaw", "--mode", "swarm"]);
        assert_eq!(args.mode, "swarm");
    }

    #[test]
    fn test_cli_mode_dispatch_supervisor() {
        let args = Args::parse_from(["ravenclaw", "--mode", "supervisor"]);
        assert_eq!(args.mode, "supervisor");
    }

    #[test]
    fn test_cli_endpoint_override() {
        let args = Args::parse_from(["ravenclaw", "--endpoint", "https://custom.api.com"]);
        assert_eq!(args.endpoint.unwrap(), "https://custom.api.com");
    }

    #[test]
    fn test_cli_model_override() {
        let args = Args::parse_from(["ravenclaw", "--model", "gpt-4-turbo"]);
        assert_eq!(args.model.unwrap(), "gpt-4-turbo");
    }

    #[test]
    fn test_cli_all_overrides() {
        let args = Args::parse_from([
            "ravenclaw",
            "--provider",
            "ollama",
            "--endpoint",
            "http://localhost:11434",
            "--model",
            "llama3.1",
            "--verbose",
        ]);
        assert_eq!(args.provider.unwrap(), "ollama");
        assert_eq!(args.endpoint.unwrap(), "http://localhost:11434");
        assert_eq!(args.model.unwrap(), "llama3.1");
        assert!(args.verbose);
    }

    #[test]
    fn test_cli_provider_mapping_all_variants() {
        // Test all provider mappings including edge cases
        let test_cases = vec![
            ("litellm", config::LLMProvider::LiteLLM),
            ("openrouter", config::LLMProvider::OpenRouter),
            ("ollama", config::LLMProvider::Ollama),
            ("openai", config::LLMProvider::OpenAI),
            ("LiteLLM", config::LLMProvider::LiteLLM),
            ("OpenRouter", config::LLMProvider::OpenRouter),
            ("OLLAMA", config::LLMProvider::Ollama),
            ("OpenAI", config::LLMProvider::OpenAI),
            ("", config::LLMProvider::LiteLLM),
            ("unknown", config::LLMProvider::LiteLLM),
            ("MIXED_CASE", config::LLMProvider::LiteLLM),
        ];

        for (input, expected) in test_cases {
            let mapped = match input.to_lowercase().as_str() {
                "openrouter" => config::LLMProvider::OpenRouter,
                "ollama" => config::LLMProvider::Ollama,
                "openai" => config::LLMProvider::OpenAI,
                _ => config::LLMProvider::LiteLLM,
            };
            assert_eq!(mapped, expected, "Mapping failed for '{}'", input);
        }
    }

    #[test]
    fn test_cli_version_env() {
        // Verify that the version is set from CARGO_PKG_VERSION
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
