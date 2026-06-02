//! Agent implementations for RavenClaw
//!
//! Supports single-provider and multi-model (multi-provider) modes.

use std::sync::Arc;
use crate::config::Config;
use crate::error::Result;
use crate::llm::{ChatMessage, LLMProviderTrait, MultiModelManager};
use tracing::{info, warn};

/// Run a single autonomous agent (single-provider mode)
pub async fn run_single(llm: Arc<dyn LLMProviderTrait>, config: Config) -> Result<()> {
    info!("Starting single agent mode with provider: {}", llm.provider_name());
    
    let system_prompt = "You are RavenClaw, a lightweight autonomous agent. \
        Be concise, efficient, and secure. Always validate inputs and outputs.";
    
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
pub async fn run_swarm(llm: Arc<dyn LLMProviderTrait>, config: Config) -> Result<()> {
    info!("Starting swarm mode with max {} agents", config.runtime.max_agents);
    
    // TODO: Implement swarm coordination via RavenFabric
    warn!("Swarm mode not yet implemented");
    
    Ok(())
}

/// Run supervisor agent coordinating sub-agents (single-provider)
pub async fn run_supervisor(llm: Arc<dyn LLMProviderTrait>, config: Config) -> Result<()> {
    info!("Starting supervisor mode");
    
    // TODO: Implement supervisor logic
    warn!("Supervisor mode not yet implemented");
    
    Ok(())
}

/// Run a single autonomous agent (multi-model mode)
pub async fn run_single_multi(multi_llm: MultiModelManager, config: Config) -> Result<()> {
    info!("Starting single agent mode (multi-model) with {} providers", multi_llm.client_count());
    
    let system_prompt = "You are RavenClaw, a lightweight autonomous agent. \
        Be concise, efficient, and secure. Always validate inputs and outputs.";
    
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
    
    // Test each configured provider
    for i in 0..multi_llm.client_count() {
        if let Some(client) = multi_llm.get_client(i) {
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
        }
    }
    
    Ok(())
}

/// Run multiple agents in swarm mode (multi-model)
pub async fn run_swarm_multi(multi_llm: MultiModelManager, config: Config) -> Result<()> {
    info!("Starting swarm mode (multi-model) with {} providers, max {} agents", 
          multi_llm.client_count(), config.runtime.max_agents);
    
    // TODO: Implement swarm coordination with provider selection strategy
    // - Round-robin for load balancing
    // - Model-specific routing (cheap models for simple tasks, expensive for complex)
    // - Fallback chain on errors
    warn!("Swarm mode (multi-model) not yet implemented");
    
    Ok(())
}

/// Run supervisor agent coordinating sub-agents (multi-model)
pub async fn run_supervisor_multi(multi_llm: MultiModelManager, config: Config) -> Result<()> {
    info!("Starting supervisor mode (multi-model)");
    
    // TODO: Implement supervisor logic with intelligent provider selection
    warn!("Supervisor mode (multi-model) not yet implemented");
    
    Ok(())
}
