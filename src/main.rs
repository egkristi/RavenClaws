//! RavenClaw — Lightweight, secure Rust agent framework
//!
//! Built for efficiency, security, and easy deployment.
//! Supports multiple LLM providers: LiteLLM, OpenRouter, Ollama, OpenAI.

mod agent;
mod config;
mod error;
mod llm;

use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "ravenclaw")]
#[command(author = "Erling G M Kristiansen")]
#[command(version = "0.1.0")]
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| format!("ravenclaw={}", log_level).into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "RavenClaw starting");

    // Load configuration
    let mut config = config::Config::load(args.config.as_deref())?;
    
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
    
    info!(mode = %args.mode, "Configuration loaded");

    // Determine if multi-model or single-provider mode
    let has_multi_model = !config.llms.is_empty();
    
    if has_multi_model {
        info!(providers = config.llms.len(), "Multi-model mode enabled");
        
        // Create multi-model manager
        let multi_llm = llm::MultiModelManager::new(config.llms.clone())?;
        
        for (i, client) in (0..multi_llm.client_count()).map(|i| multi_llm.get_client(i).unwrap()) {
            info!(provider = client.provider_name(), model = client.model(), "Provider initialized");
        }
        
        // Run agent based on mode with multi-model support
        match args.mode.as_str() {
            "single" => {
                info!("Running in single agent mode (multi-model)");
                agent::run_single_multi(multi_llm, config).await?;
            }
            "swarm" => {
                info!("Running in swarm mode (multi-model)");
                agent::run_swarm_multi(multi_llm, config).await?;
            }
            "supervisor" => {
                info!("Running in supervisor mode (multi-model)");
                agent::run_supervisor_multi(multi_llm, config).await?;
            }
            _ => {
                anyhow::bail!("Unknown mode: {}. Use: single, swarm, or supervisor", args.mode);
            }
        }
    } else {
        // Single provider mode
        let provider_name = match config.llm.provider {
            config::LLMProvider::LiteLLM => "LiteLLM",
            config::LLMProvider::OpenRouter => "OpenRouter",
            config::LLMProvider::Ollama => "Ollama",
            config::LLMProvider::OpenAI => "OpenAI",
        };
        
        info!(provider = provider_name, endpoint = %config.llm.endpoint, model = %config.llm.model, "LLM client initialized");
        
        // Create appropriate client based on provider
        let llm = llm::create_client(&config.llm)?;
        info!(provider = llm.provider_name(), model = llm.model(), "Provider ready");
        
        // Run agent based on mode
        match args.mode.as_str() {
            "single" => {
                info!("Running in single agent mode");
                agent::run_single(llm, config).await?;
            }
            "swarm" => {
                info!("Running in swarm mode");
                agent::run_swarm(llm, config).await?;
            }
            "supervisor" => {
                info!("Running in supervisor mode");
                agent::run_supervisor(llm, config).await?;
            }
            _ => {
                anyhow::bail!("Unknown mode: {}. Use: single, swarm, or supervisor", args.mode);
            }
        }
    }

    info!("RavenClaw shutdown complete");
    Ok(())
}
