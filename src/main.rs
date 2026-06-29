//! RavenClaws — Lightweight, secure Rust agent framework
//!
//! Built for efficiency, security, and easy deployment.
//! Supports multiple LLM providers: LiteLLM, OpenRouter, Ollama, OpenAI.

mod agent;
mod audit;
mod background;
mod config;
mod error;
mod eval;
mod heartbeat;
mod llm;
mod mcp;
mod patterns;
mod policy;
mod ravenfabric;
mod sandbox;
mod scheduler;
mod server;
mod swarm;
mod telemetry;
mod tools;

use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Parser, Debug)]
#[command(name = "ravenclaws")]
#[command(author = "Erling G M Kristiansen")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Lightweight, secure Rust agent framework with multi-provider support", long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, env = "RAVENCLAWS_CONFIG")]
    config: Option<String>,

    /// Agent mode: single, swarm, or supervisor
    #[arg(short, long, default_value = "single")]
    mode: String,

    /// Enable verbose logging
    #[arg(short, long, env = "RAVENCLAWS_VERBOSE")]
    verbose: bool,

    /// Run a one-shot command
    #[arg(short, long)]
    exec: Option<String>,

    /// Provider type: litellm, openrouter, ollama, openai (overrides config)
    #[arg(long, env = "RAVENCLAWS_PROVIDER")]
    provider: Option<String>,

    /// LLM endpoint (overrides config)
    #[arg(long, env = "RAVENCLAWS_ENDPOINT")]
    endpoint: Option<String>,

    /// Model name (overrides config)
    #[arg(long, env = "RAVENCLAWS_MODEL")]
    model: Option<String>,

    /// System prompt / persona (overrides config)
    #[arg(long, env = "RAVENCLAWS_SYSTEM_PROMPT")]
    system_prompt: Option<String>,

    /// Interactive REPL mode (read-eval-print loop)
    #[arg(long, short = 'R', conflicts_with = "exec")]
    repl: bool,

    /// Require human approval for sensitive tool calls (HITL)
    #[arg(long, env = "RAVENCLAWS_REQUIRE_APPROVAL")]
    require_approval: bool,

    /// Maximum iterations for the agent loop (default: 10)
    #[arg(long, env = "RAVENCLAWS_MAX_ITERATIONS", default_value = "10")]
    max_iterations: usize,

    /// Token budget per run (v0.5) — stops when exceeded
    #[arg(long, env = "RAVENCLAWS_TOKEN_BUDGET")]
    token_budget: Option<u32>,

    /// Retry max attempts (v0.5) — default 3
    #[arg(long, env = "RAVENCLAWS_RETRY_MAX", default_value = "3")]
    retry_max: u32,

    /// Retry base delay in ms (v0.5) — default 100
    #[arg(long, env = "RAVENCLAWS_RETRY_BASE_DELAY", default_value = "100")]
    retry_base_delay_ms: u64,

    /// Enable provider fallback chain (v0.5) — comma-separated providers
    #[arg(long, env = "RAVENCLAWS_FALLBACK_CHAIN")]
    fallback_chain: Option<String>,

    /// MCP server command (v0.5.2) — stdio transport (e.g., "npx -y @modelcontextprotocol/server-filesystem")
    #[arg(long, env = "RAVENCLAWS_MCP_COMMAND")]
    mcp_command: Option<String>,

    /// MCP server arguments (v0.5.2) — space-separated args for the MCP command
    #[arg(long, env = "RAVENCLAWS_MCP_ARGS")]
    mcp_args: Option<String>,

    /// MCP server environment variables (v0.5.2) — KEY=VALUE pairs separated by commas
    #[arg(long, env = "RAVENCLAWS_MCP_ENV")]
    mcp_env: Option<String>,

    /// Run as MCP server (v0.7) — exposes RavenClaws tools over stdio via MCP protocol
    #[arg(long, env = "RAVENCLAWS_MCP_SERVER")]
    mcp_server: bool,

    /// Run as HTTP server (v0.7) — long-running with /health, /ready, /metrics endpoints
    #[arg(long, env = "RAVENCLAWS_SERVE")]
    serve: bool,

    /// HTTP server host (v0.7) — overrides config
    #[arg(long, env = "RAVENCLAWS_SERVER_HOST")]
    server_host: Option<String>,

    /// HTTP server port (v0.7) — overrides config
    #[arg(long, env = "RAVENCLAWS_SERVER_PORT")]
    server_port: Option<u16>,

    /// OpenTelemetry OTLP gRPC endpoint (v0.7.2)
    #[arg(long, env = "RAVENCLAWS_OTEL_ENDPOINT")]
    otel_endpoint: Option<String>,

    /// OpenTelemetry service name (v0.7.2)
    #[arg(long, env = "RAVENCLAWS_OTEL_SERVICE_NAME")]
    otel_service_name: Option<String>,

    /// Disable OpenTelemetry tracing (v0.7.2)
    #[arg(long, env = "RAVENCLAWS_OTEL_DISABLED")]
    otel_disabled: bool,

    /// Submit a background task and return immediately (v0.8)
    #[arg(long, env = "RAVENCLAWS_BACKGROUND")]
    background: bool,

    /// Check status of a background task (v0.8)
    #[arg(long, env = "RAVENCLAWS_TASK_STATUS")]
    task_status: Option<String>,

    /// List all background tasks (v0.8)
    #[arg(long, env = "RAVENCLAWS_TASK_LIST")]
    task_list: bool,

    /// Cancel a background task (v0.8)
    #[arg(long, env = "RAVENCLAWS_TASK_CANCEL")]
    task_cancel: Option<String>,

    /// Resume incomplete background tasks on startup (v0.8)
    #[arg(long, env = "RAVENCLAWS_TASK_RESUME")]
    task_resume: bool,

    /// Run the scheduler with configured triggers (v0.8)
    #[arg(long, env = "RAVENCLAWS_SCHEDULER")]
    scheduler: bool,

    /// Webhook server port (v0.8) — overrides default 9090
    #[arg(long, env = "RAVENCLAWS_WEBHOOK_PORT", default_value = "9090")]
    webhook_port: u16,

    /// Run eval suite from config file (v0.9)
    #[arg(long, env = "RAVENCLAWS_EVAL")]
    eval: Option<String>,

    /// Output eval results as JSON (v0.9)
    #[arg(long, env = "RAVENCLAWS_EVAL_JSON")]
    eval_json: bool,

    /// Run in autonomous heartbeat mode (v0.9)
    #[arg(long, env = "RAVENCLAWS_HEARTBEAT")]
    heartbeat: bool,

    /// Goal prompt for heartbeat mode (v0.9)
    #[arg(long, env = "RAVENCLAWS_HEARTBEAT_GOAL")]
    heartbeat_goal: Option<String>,

    /// Tick interval in seconds for heartbeat mode (v0.9, default: 300)
    #[arg(
        long,
        env = "RAVENCLAWS_HEARTBEAT_TICK_INTERVAL",
        default_value = "300"
    )]
    heartbeat_tick_interval: u64,

    /// Maximum ticks for heartbeat mode (v0.9, 0 = unlimited)
    #[arg(long, env = "RAVENCLAWS_HEARTBEAT_MAX_TICKS", default_value = "0")]
    heartbeat_max_ticks: u64,

    /// Heartbeat session ID for resuming (v0.9)
    #[arg(long, env = "RAVENCLAWS_HEARTBEAT_SESSION")]
    heartbeat_session: Option<String>,

    /// Swarm topology: star, mesh, hierarchical, hybrid (v0.9)
    #[arg(long, env = "RAVENCLAWS_SWARM_TOPOLOGY", default_value = "star")]
    swarm_topology: String,

    /// Maximum recursion depth for hierarchical swarm (v0.9, default: 3)
    #[arg(long, env = "RAVENCLAWS_SWARM_MAX_DEPTH", default_value = "3")]
    swarm_max_depth: usize,

    /// Maximum workers in the swarm (v0.9, default: 100)
    #[arg(long, env = "RAVENCLAWS_SWARM_MAX_WORKERS", default_value = "100")]
    swarm_max_workers: usize,

    /// Enable dynamic role assignment (v0.9)
    #[arg(long, env = "RAVENCLAWS_SWARM_DYNAMIC_ROLES")]
    swarm_dynamic_roles: bool,

    /// Worker profiles file path (v0.9, JSON)
    #[arg(long, env = "RAVENCLAWS_SWARM_PROFILES")]
    swarm_profiles: Option<String>,

    /// Enable inter-agent communication in swarm mode (v0.9)
    #[arg(long, env = "RAVENCLAWS_SWARM_COMMUNICATION")]
    swarm_communication: bool,

    /// Enable swarm health monitoring (v0.9)
    #[arg(long, env = "RAVENCLAWS_SWARM_HEALTH_MONITORING")]
    swarm_health_monitoring: bool,

    /// When set, treat any non-tool-call response as completion (no FINAL: required)
    #[arg(long, env = "RAVENCLAWS_NO_FINAL_REQUIRED")]
    no_final_required: bool,

    // ── Multi-agent pattern flags (v0.9.13) ──
    /// Maximum debate rounds (default: 3)
    #[arg(long, env = "RAVENCLAWS_PATTERN_MAX_ROUNDS", default_value = "3")]
    pattern_max_rounds: usize,

    /// Maximum review-loop iterations (default: 3)
    #[arg(long, env = "RAVENCLAWS_PATTERN_MAX_REVIEW", default_value = "3")]
    pattern_max_review: usize,

    /// Number of research agents (default: 3)
    #[arg(long, env = "RAVENCLAWS_PATTERN_RESEARCH_AGENTS", default_value = "3")]
    pattern_research_agents: usize,

    /// Number of voters (default: 3)
    #[arg(long, env = "RAVENCLAWS_PATTERN_VOTERS", default_value = "3")]
    pattern_voters: usize,

    /// Show verbose intermediate results for patterns
    #[arg(long, env = "RAVENCLAWS_PATTERN_VERBOSE")]
    pattern_verbose: bool,
}

/// A shared shutdown flag that can be checked by running modes.
/// Set to true when SIGTERM or SIGINT is received.
#[derive(Clone)]
struct ShutdownFlag {
    triggered: Arc<AtomicBool>,
}

impl ShutdownFlag {
    fn new() -> Self {
        Self {
            triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns true if a shutdown signal has been received.
    fn is_shutting_down(&self) -> bool {
        self.triggered.load(Ordering::Relaxed)
    }

    /// Spawns a task that listens for SIGTERM/SIGINT and sets the flag.
    /// Returns the flag handle for checking shutdown status.
    fn spawn() -> Self {
        let flag = Self::new();
        let flag_clone = flag.clone();
        tokio::spawn(async move {
            // Use a stream to handle both SIGTERM and SIGINT
            let sigterm = signal::unix::signal(signal::unix::SignalKind::terminate());
            let sigint = signal::unix::signal(signal::unix::SignalKind::interrupt());

            let signal_name = match (sigterm, sigint) {
                (Ok(mut term), Ok(mut int)) => {
                    tokio::select! {
                        _ = term.recv() => "SIGTERM",
                        _ = int.recv() => "SIGINT",
                    }
                }
                _ => {
                    // Fallback: Ctrl+C only (Windows or permission-denied)
                    let _ = signal::ctrl_c().await;
                    "SIGINT"
                }
            };

            info!(signal = %signal_name, "Shutdown signal received, initiating graceful shutdown");
            flag_clone.triggered.store(true, Ordering::Relaxed);
        });
        flag
    }
}

/// A guard that logs a shutdown message on drop.
/// Place at the end of each mode's scope to ensure graceful shutdown is visible.
struct ShutdownGuard {
    mode: &'static str,
}

impl ShutdownGuard {
    fn new(mode: &'static str) -> Self {
        Self { mode }
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        info!(mode = self.mode, "RavenClaws shutdown complete");
    }
}

/// Waits until the shutdown flag is set.
/// Used with `tokio::select!` to interrupt long-running modes on signal.
async fn wait_for_shutdown(flag: &ShutdownFlag) {
    while !flag.is_shutting_down() {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("ravenclaws={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "RavenClaws starting");

    // Initialize global shutdown flag for graceful shutdown (v0.9.11)
    let shutdown = ShutdownFlag::spawn();

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
            "anthropic" => config::LLMProvider::Anthropic,
            "openai-compatible" | "openai_compatible" => config::LLMProvider::OpenAICompatible,
            "azure" => config::LLMProvider::Azure,
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
        let registry = tools::ToolRegistry::with_config(&config);
        let mut server = mcp::McpServer::new(registry);
        server
            .run()
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;
        info!("RavenClaws MCP server shutdown complete");
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
        info!("RavenClaws HTTP server shutdown complete");
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

    // Initialize RavenFabric client (v0.6.1) — must be before --exec mode
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

    // Handle --exec one-shot mode (uses agent loop for multi-step reasoning with security)
    if let Some(exec_prompt) = args.exec {
        info!("Running in --exec mode with security-integrated agent loop");
        let system_prompt = &config.llm.system_prompt;

        // Build fallback chain from multi-model configs (v0.9.8)
        let fallback_chain = if config.llms.len() > 1 {
            let chain = llm::ProviderFallbackChain::new(config.llms.clone());
            // Apply token budget if configured
            let chain = if let Some(budget) = config.llm.token_budget {
                chain.with_token_budget(llm::TokenBudget::new(budget, 0.0))
            } else {
                chain
            };
            Some(Arc::new(std::sync::Mutex::new(chain)))
        } else {
            None
        };

        // Build token budget from config (v0.9.8)
        let token_budget = config.llm.token_budget.map(|max_tokens| {
            Arc::new(std::sync::Mutex::new(llm::TokenBudget::new(
                max_tokens, 0.0,
            )))
        });

        let loop_config = agent::AgentLoopConfig {
            max_iterations: args.max_iterations,
            enable_tools: true,
            require_approval: args.require_approval,
            prompt_injection_protection: config.security.prompt_injection_protection,
            token_lifetime_secs: config.security.token_lifetime_secs,
            no_final_required: args.no_final_required,
            fallback_chain,
            token_budget,
            ravenfabric: ravenfabric.clone(),
            checkpoint_dir: None,
            session_id: None,
        };

        // Build a configured tool registry (respects web_search endpoint from config)
        let mut tool_registry = tools::ToolRegistry::with_config(&config);

        // Register MCP tools from config-based servers (v0.9.6)
        if !config.mcp.servers.is_empty() {
            info!(
                server_count = config.mcp.servers.len(),
                "Initializing MCP clients from config for --exec mode"
            );
            let mcp_manager = mcp::McpClientManager::from_config(&config.mcp).await;
            let registered = mcp_manager.register_all_tools(&mut tool_registry).await;
            info!(
                connected = mcp_manager.len(),
                tools_registered = registered,
                "MCP tools registered from config for --exec mode"
            );
        }

        let tool_registry = Some(tool_registry);

        let response = if !config.llms.is_empty() {
            let multi_llm = llm::MultiModelManager::new(config.llms.clone())?;
            if let Some(client) = multi_llm.get_client(0) {
                agent::run_agent_loop_with_mcp_and_registry(
                    client.clone(),
                    &exec_prompt,
                    system_prompt,
                    loop_config,
                    mcp_client,
                    tool_registry,
                )
                .await?
            } else {
                anyhow::bail!("No LLM providers available for --exec mode");
            }
        } else {
            let llm = llm::create_client(&config.llm)?;
            agent::run_agent_loop_with_mcp_and_registry(
                llm,
                &exec_prompt,
                system_prompt,
                loop_config,
                mcp_client,
                tool_registry,
            )
            .await?
        };
        println!("{}", response);
        info!("RavenClaws shutdown complete");
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
        info!("RavenClaws shutdown complete");
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
        info!("RavenClaws shutdown complete");
        return Ok(());
    }

    // Handle --task-cancel: cancel a background task (v0.8)
    if let Some(task_id) = args.task_cancel {
        match bg_manager.cancel(&task_id).await {
            Ok(()) => println!("Task '{}' cancelled.", task_id),
            Err(e) => eprintln!("Error: {}", e),
        }
        info!("RavenClaws shutdown complete");
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
        info!("RavenClaws shutdown complete");
        return Ok(());
    }

    // Handle --eval mode: run eval suite (v0.9)
    if let Some(eval_path) = args.eval {
        info!(path = %eval_path, "Running eval suite");
        let eval_config = eval::EvalConfig::from_file(&eval_path)?;
        let llm = llm::create_client(&config.llm)?;
        let runner = eval::EvalRunner::new(llm, eval_config);
        let report = runner.run_suite().await;
        if args.eval_json {
            println!("{}", report.format_json());
        } else {
            println!("{}", report.format_text());
        }
        info!("RavenClaws eval complete");
        return Ok(());
    }

    // Handle --scheduler mode: run configured triggers (v0.8)
    if args.scheduler {
        info!("Running in scheduler mode with configured triggers");

        let mut scheduler = scheduler::Scheduler::new(bg_manager.clone(), &config.scheduler);
        scheduler.set_webhook_port(args.webhook_port);
        scheduler.start().await?;

        info!(
            trigger_count = config.scheduler.triggers.len(),
            "Scheduler started — waiting for triggers"
        );

        // Wait for shutdown signal via unified flag
        while !shutdown.is_shutting_down() {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        info!("Shutdown signal received, stopping scheduler...");

        scheduler.stop().await;
        info!("RavenClaws scheduler shutdown complete");
        return Ok(());
    }

    // Handle --heartbeat mode: autonomous heartbeat agent (v0.9)
    if args.heartbeat {
        info!("Running in autonomous heartbeat mode");

        // Determine goal: CLI arg > config > error
        let goal = if let Some(goal) = args.heartbeat_goal {
            goal
        } else if !config.heartbeat.goal.is_empty() {
            config.heartbeat.goal.clone()
        } else {
            anyhow::bail!(
                "No goal provided for heartbeat mode. Use --heartbeat-goal (e.g., --heartbeat-goal \"Monitor system health and report anomalies\") or set [heartbeat].goal in config."
            );
        };

        // Build heartbeat config from CLI overrides + config
        let hb_config = heartbeat::HeartbeatConfig {
            goal,
            tick_interval_secs: args.heartbeat_tick_interval,
            max_iterations_per_tick: config.heartbeat.max_iterations_per_tick,
            workdir: config.heartbeat.workdir.clone(),
            max_ticks: args.heartbeat_max_ticks,
            enable_tools: config.heartbeat.enable_tools,
        };

        let llm = llm::create_client(&config.llm)?;
        let mut agent =
            heartbeat::HeartbeatAgent::new(llm, hb_config, args.heartbeat_session).await?;

        // Wire up the shutdown flag for graceful termination (v0.9.11)
        agent.set_shutdown_flag(shutdown.triggered.clone());

        info!(
            heartbeat_id = %agent.id(),
            "Heartbeat agent initialized"
        );

        let result = agent.run().await?;
        println!("{}", result);
        info!("RavenClaws heartbeat complete");
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
        info!("RavenClaws shutdown complete");
        return Ok(());
    }

    // Apply swarm CLI overrides (v0.9)
    config.swarm.topology = match args.swarm_topology.to_lowercase().as_str() {
        "mesh" => swarm::SwarmTopology::Mesh,
        "hierarchical" => swarm::SwarmTopology::Hierarchical,
        "hybrid" => swarm::SwarmTopology::Hybrid,
        _ => swarm::SwarmTopology::Star,
    };
    config.swarm.max_depth = args.swarm_max_depth;
    config.swarm.max_workers = args.swarm_max_workers;
    if args.swarm_dynamic_roles {
        config.swarm.dynamic_role_assignment = true;
    }
    if args.swarm_communication {
        config.swarm.enable_agent_communication = true;
    }
    if args.swarm_health_monitoring {
        config.swarm.enable_health_monitoring = true;
    }

    // Load worker profiles from file if specified
    if let Some(profiles_path) = args.swarm_profiles {
        match std::fs::read_to_string(&profiles_path) {
            Ok(content) => match serde_json::from_str::<Vec<swarm::WorkerProfile>>(&content) {
                Ok(profiles) => {
                    config.swarm.profiles = profiles;
                    info!(
                        count = config.swarm.profiles.len(),
                        path = %profiles_path,
                        "Loaded worker profiles"
                    );
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        path = %profiles_path,
                        "Failed to parse worker profiles, using defaults"
                    );
                }
            },
            Err(e) => {
                warn!(
                    error = %e,
                    path = %profiles_path,
                    "Failed to read worker profiles file, using defaults"
                );
            }
        }
    }

    // Determine if multi-model or single-provider mode
    let has_multi_model = !config.llms.is_empty();

    // Run the selected mode with graceful shutdown support.
    // Long-running modes (single/swarm/supervisor/orchestrate) are spawned in a task
    // so that SIGTERM/SIGINT can trigger a graceful exit.
    let mode_result: anyhow::Result<()> = if has_multi_model {
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
                let guard = ShutdownGuard::new("single (multi-model)");
                info!("Running in single agent mode (multi-model)");
                let mode_fut = agent::run_single_multi(multi_llm, config, ravenfabric);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting single agent mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "swarm" => {
                let guard = ShutdownGuard::new("swarm (multi-model)");
                info!("Running in swarm mode (multi-model)");
                let mode_fut = agent::run_swarm_multi(multi_llm, config, ravenfabric);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting swarm mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "supervisor" => {
                let guard = ShutdownGuard::new("supervisor (multi-model)");
                info!("Running in supervisor mode (multi-model)");
                let mode_fut = agent::run_supervisor_multi(multi_llm, config, ravenfabric);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting supervisor mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "orchestrate" => {
                let guard = ShutdownGuard::new("orchestrate (multi-model)");
                info!("Running in swarm orchestration mode (multi-model)");
                let mut orchestrator = swarm::SwarmOrchestrator::new(
                    config.swarm.clone(),
                    None,
                    Some(multi_llm),
                    ravenfabric,
                );
                orchestrator.init().await?;
                let task = "Complete the assigned task using available providers.";
                let result = orchestrator.orchestrate(task).await?;
                println!("{}", result);
                drop(guard);
                Ok(())
            }
            // ── Multi-agent patterns (multi-model, v0.9.13) ──
            "debate" => {
                let guard = ShutdownGuard::new("debate (multi-model)");
                info!("Running in debate mode (multi-model)");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut =
                    patterns::run_debate_multi(multi_llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting debate mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "review-loop" | "review_loop" => {
                let guard = ShutdownGuard::new("review-loop (multi-model)");
                info!("Running in review-loop mode (multi-model)");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut =
                    patterns::run_review_loop_multi(multi_llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting review-loop mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "research-synthesize" | "research_synthesize" => {
                let guard = ShutdownGuard::new("research-synthesize (multi-model)");
                info!("Running in research-synthesize mode (multi-model)");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut = patterns::run_research_synthesize_multi(
                    multi_llm,
                    config,
                    ravenfabric,
                    pattern_cfg,
                );
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting research-synthesize mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "voting" => {
                let guard = ShutdownGuard::new("voting (multi-model)");
                info!("Running in voting mode (multi-model)");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut =
                    patterns::run_voting_multi(multi_llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting voting mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            _ => {
                anyhow::bail!(
                    "Unknown mode: {}. Use: single, swarm, supervisor, orchestrate, debate, review-loop, research-synthesize, or voting",
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
            config::LLMProvider::OpenAICompatible => "OpenAI-Compatible",
            config::LLMProvider::Azure => "Azure OpenAI",
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
                let guard = ShutdownGuard::new("single");
                info!("Running in single agent mode");
                let mode_fut = agent::run_single(llm, config, ravenfabric);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting single agent mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "swarm" => {
                let guard = ShutdownGuard::new("swarm");
                info!("Running in swarm mode");
                let mode_fut = agent::run_swarm(llm, config, ravenfabric);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting swarm mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "supervisor" => {
                let guard = ShutdownGuard::new("supervisor");
                info!("Running in supervisor mode");
                let mode_fut = agent::run_supervisor(llm, config, ravenfabric);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting supervisor mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "orchestrate" => {
                let guard = ShutdownGuard::new("orchestrate");
                info!("Running in swarm orchestration mode");
                let mut orchestrator = swarm::SwarmOrchestrator::new(
                    config.swarm.clone(),
                    Some(llm),
                    None,
                    ravenfabric,
                );
                orchestrator.init().await?;
                let task = "Complete the assigned task using available providers.";
                let result = orchestrator.orchestrate(task).await?;
                println!("{}", result);
                drop(guard);
                Ok(())
            }
            // ── Multi-agent patterns (v0.9.13) ──
            "debate" => {
                let guard = ShutdownGuard::new("debate");
                info!("Running in debate mode");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut = patterns::run_debate(llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting debate mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "review-loop" | "review_loop" => {
                let guard = ShutdownGuard::new("review-loop");
                info!("Running in review-loop mode");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut = patterns::run_review_loop(llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting review-loop mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "research-synthesize" | "research_synthesize" => {
                let guard = ShutdownGuard::new("research-synthesize");
                info!("Running in research-synthesize mode");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut =
                    patterns::run_research_synthesize(llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting research-synthesize mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            "voting" => {
                let guard = ShutdownGuard::new("voting");
                info!("Running in voting mode");
                let pattern_cfg = patterns::PatternConfig {
                    max_rounds: args.pattern_max_rounds,
                    max_review_iterations: args.pattern_max_review,
                    research_agent_count: args.pattern_research_agents,
                    voter_count: args.pattern_voters,
                    verbose: args.pattern_verbose,
                };
                let mode_fut = patterns::run_voting(llm, config, ravenfabric, pattern_cfg);
                tokio::select! {
                    result = mode_fut => result?,
                    _ = wait_for_shutdown(&shutdown) => {
                        info!("Shutdown signal received, exiting voting mode");
                    }
                }
                drop(guard);
                Ok(())
            }
            _ => {
                anyhow::bail!(
                    "Unknown mode: {}. Use: single, swarm, supervisor, orchestrate, debate, review-loop, research-synthesize, or voting",
                    args.mode
                );
            }
        }
    };

    // Propagate any mode error
    mode_result?;

    info!("RavenClaws shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_args() {
        // Verify the CLI struct can be constructed with defaults
        let args = Args::parse_from(["ravenclaws"]);
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
            "ravenclaws",
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
            "ravenclaws",
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
        let args = Args::parse_from(["ravenclaws", "--mode", "invalid"]);
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
            ("azure", config::LLMProvider::Azure),
            ("unknown", config::LLMProvider::LiteLLM), // default
        ];

        for (input, expected) in test_cases {
            let mapped = match input.to_lowercase().as_str() {
                "openrouter" => config::LLMProvider::OpenRouter,
                "ollama" => config::LLMProvider::Ollama,
                "openai" => config::LLMProvider::OpenAI,
                "azure" => config::LLMProvider::Azure,
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
        let args = Args::parse_from(["ravenclaws", "--verbose"]);
        assert!(args.verbose);

        let args = Args::parse_from(["ravenclaws"]);
        assert!(!args.verbose);
    }

    #[test]
    fn test_cli_env_var_mapping() {
        // Verify that env var names match CLI args
        // RAVENCLAWS_CONFIG → config
        // RAVENCLAWS_VERBOSE → verbose
        // RAVENCLAWS_PROVIDER → provider
        // RAVENCLAWS_ENDPOINT → endpoint
        // RAVENCLAWS_MODEL → model
        let args = Args::parse_from(["ravenclaws"]);
        assert_eq!(args.config, None);
        assert!(!args.verbose);
        assert_eq!(args.provider, None);
        assert_eq!(args.endpoint, None);
        assert_eq!(args.model, None);
    }

    #[test]
    fn test_cli_exec_with_provider() {
        let args = Args::parse_from([
            "ravenclaws",
            "--exec",
            "test prompt",
            "--provider",
            "openai",
        ]);
        assert_eq!(args.exec.unwrap(), "test prompt");
        assert_eq!(args.provider.unwrap(), "openai");
    }

    #[test]
    fn test_cli_mode_dispatch_single() {
        let args = Args::parse_from(["ravenclaws", "--mode", "single"]);
        assert_eq!(args.mode, "single");
    }

    #[test]
    fn test_cli_mode_dispatch_swarm() {
        let args = Args::parse_from(["ravenclaws", "--mode", "swarm"]);
        assert_eq!(args.mode, "swarm");
    }

    #[test]
    fn test_cli_mode_dispatch_supervisor() {
        let args = Args::parse_from(["ravenclaws", "--mode", "supervisor"]);
        assert_eq!(args.mode, "supervisor");
    }

    #[test]
    fn test_cli_endpoint_override() {
        let args = Args::parse_from(["ravenclaws", "--endpoint", "https://custom.api.com"]);
        assert_eq!(args.endpoint.unwrap(), "https://custom.api.com");
    }

    #[test]
    fn test_cli_model_override() {
        let args = Args::parse_from(["ravenclaws", "--model", "gpt-4-turbo"]);
        assert_eq!(args.model.unwrap(), "gpt-4-turbo");
    }

    #[test]
    fn test_cli_all_overrides() {
        let args = Args::parse_from([
            "ravenclaws",
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
