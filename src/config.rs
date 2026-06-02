//! Configuration management for RavenClaw
//!
//! Secure by default: no credentials in config files, use environment variables.
//! Supports multiple LLM providers: LiteLLM, OpenRouter, Ollama, OpenAI.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load config: {0}")]
    LoadError(String),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
}

/// LLM Provider type — determines which backend to use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    LiteLLM,
    OpenRouter,
    Ollama,
    OpenAI,
}

impl Default for LLMProvider {
    fn default() -> Self {
        LLMProvider::LiteLLM
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// LiteLLM configuration (single provider mode)
    #[serde(default)]
    pub llm: LLMConfig,
    
    /// Multiple LLM configurations (multi-model mode)
    #[serde(default)]
    pub llms: Vec<LLMConfig>,
    
    /// RavenFabric configuration
    #[serde(default)]
    pub ravenfabric: RavenFabricConfig,
    
    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,
    
    /// Runtime settings
    #[serde(default)]
    pub runtime: RuntimeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LLMConfig {
    /// Provider type: litellm, openrouter, ollama, openai
    #[serde(default)]
    pub provider: LLMProvider,
    
    /// Endpoint URL (e.g., http://litellm:4000, http://localhost:11434, https://api.openai.com)
    #[serde(default)]
    pub endpoint: String,
    
    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,
    
    /// API key (prefer env var)
    #[serde(default)]
    pub api_key: Option<String>,
    
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RavenFabricConfig {
    /// RavenFabric endpoint
    #[serde(default)]
    pub endpoint: Option<String>,
    
    /// Agent ID for identification
    #[serde(default)]
    pub agent_id: Option<String>,
    
    /// Enable remote command execution
    #[serde(default = "default_true")]
    pub remote_exec: bool,
    
    /// Allowed remote hosts (whitelist)
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    /// Require TLS for all connections
    #[serde(default = "default_true")]
    pub require_tls: bool,
    
    /// Maximum token lifetime in seconds
    #[serde(default = "default_token_lifetime")]
    pub token_lifetime_secs: u64,
    
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub audit_log: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    /// Working directory
    #[serde(default = "default_workdir")]
    pub workdir: String,
    
    /// Maximum concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    
    /// Health check interval in seconds
    #[serde(default = "default_health_interval")]
    pub health_interval_secs: u64,
}

fn default_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

fn default_token_lifetime() -> u64 {
    3600
}

fn default_workdir() -> String {
    "/workspace".to_string()
}

fn default_max_agents() -> usize {
    10
}

fn default_health_interval() -> u64 {
    60
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::LiteLLM,
            endpoint: String::new(),
            model: default_model(),
            api_key: None,
            timeout_secs: default_timeout(),
        }
    }
}

impl Config {
    /// Load configuration from file and environment
    pub fn load(config_path: Option<&str>) -> Result<Self, ConfigError> {
        // Start with defaults from environment
        dotenvy::dotenv().ok();
        
        let mut config_builder = config::Config::builder();
        
        // Load from file if provided
        if let Some(path) = config_path {
            config_builder = config_builder.add_source(config::File::with_name(path).required(false));
        }
        
        // Load from environment (RAVENCLAW_* prefix)
        config_builder = config_builder.add_source(
            config::Environment::with_prefix("RAVENCLAW").separator("__")
        );
        
        let config = config_builder
            .build()
            .map_err(|e| ConfigError::LoadError(e.to_string()))?;
        
        let mut cfg: Config = config
            .try_deserialize()
            .map_err(|e| ConfigError::LoadError(e.to_string()))?;
        
        // Override sensitive values from environment
        // Single provider mode
        if let Ok(key) = std::env::var("LITELLM_API_KEY") {
            cfg.llm.api_key = Some(key);
        }
        if let Ok(provider) = std::env::var("RAVENCLAW__LLM__PROVIDER") {
            cfg.llm.provider = match provider.to_lowercase().as_str() {
                "openrouter" => LLMProvider::OpenRouter,
                "ollama" => LLMProvider::Ollama,
                "openai" => LLMProvider::OpenAI,
                _ => LLMProvider::LiteLLM,
            };
        }
        if let Ok(endpoint) = std::env::var("RAVENCLAW__LLM__ENDPOINT") {
            cfg.llm.endpoint = endpoint;
        }
        if let Ok(model) = std::env::var("RAVENCLAW__LLM__MODEL") {
            cfg.llm.model = model;
        }
        
        // Multi-provider mode
        if let Ok(keys) = std::env::var("RAVENCLAW__LLMS") {
            // Parse JSON array of LLM configs from env
            if let Ok(llms) = serde_json::from_str::<Vec<LLMConfig>>(&keys) {
                cfg.llms = llms;
            }
        }
        
        if let Ok(endpoint) = std::env::var("RAVENFABRIC_ENDPOINT") {
            cfg.ravenfabric.endpoint = Some(endpoint);
        }
        
        // Validate
        cfg.validate()?;
        
        Ok(cfg)
    }
    
    /// Validate configuration
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate single provider config
        if !self.llm.endpoint.is_empty() {
            self.validate_llm_config(&self.llm)?;
        }
        
        // Validate multi-provider configs
        for (i, llm) in self.llms.iter().enumerate() {
            self.validate_llm_config(llm)
                .map_err(|e| ConfigError::ValidationError(format!("LLM[{}]: {}", i, e)))?;
        }
        
        // At least one provider must be configured
        if self.llm.endpoint.is_empty() && self.llms.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one LLM provider must be configured (llm or llms)".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn validate_llm_config(&self, llm: &LLMConfig) -> Result<(), ConfigError> {
        if llm.endpoint.is_empty() && llm.provider != LLMProvider::OpenAI && llm.provider != LLMProvider::OpenRouter {
            // OpenAI and OpenRouter have fixed endpoints
            return Err(ConfigError::ValidationError(
                "LLM endpoint is required for this provider".to_string()
            ));
        }
        
        if self.security.require_tls && !llm.endpoint.is_empty() {
            if !llm.endpoint.starts_with("https://") {
                // Allow localhost for development
                if !llm.endpoint.contains("localhost") && !llm.endpoint.contains("127.0.0.1") && !llm.endpoint.contains("0.0.0.0") {
                    return Err(ConfigError::ValidationError(
                        "TLS required but endpoint is not HTTPS".to_string()
                    ));
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        std::env::set_var("LITELLM_API_KEY", "test-key");
        std::env::set_var("RAVENCLAW__LLM__ENDPOINT", "http://localhost:4000");
        
        let config = Config::load(None).unwrap();
        assert_eq!(config.llm.model, "gpt-4o-mini");
        assert_eq!(config.llm.timeout_secs, 30);
        assert!(config.security.require_tls);
    }
}
