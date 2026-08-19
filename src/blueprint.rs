//! # Declarative Agent Blueprints
//!
//! A blueprint is a reusable, shareable, serializable description of an agent:
//! its persona (system prompt), LLM provider/model, the tools it may use, its
//! security policy, and its runtime behavior. Blueprints are the declarative
//! counterpart to the imperative `Config` struct — a single TOML/JSON document
//! that fully describes *what* an agent is, ready to be versioned, shared, and
//! instantiated.
//!
//! ## Example (TOML)
//!
//! ```toml
//! name = "researcher"
//! description = "Fetches and summarizes web sources"
//! version = "1.0.0"
//!
//! [persona]
//! system_prompt = "You are a thorough research assistant."
//!
//! [llm]
//! provider = "openai-compatible"
//! endpoint = "http://localhost:11434"
//! model = "llama3.1"
//!
//! tools = ["web_fetch", "web_search"]
//! require_approval = false
//! max_iterations = 5
//! ```
//!
//! ## Stability
//! `AgentBlueprint` is `#[non_exhaustive]` — new fields may be added in minor releases.

use serde::Deserialize;

use crate::config::{Config, LLMConfig, SecurityConfig};

/// A declarative description of an agent, deserialized from TOML or JSON.
///
/// # Stability
/// This struct is `#[non_exhaustive]` — new fields may be added in minor releases.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
#[allow(dead_code)] // fields are public library API; the binary reads a subset
pub struct AgentBlueprint {
    /// Human-readable name (e.g. "researcher", "code-reviewer").
    pub name: String,

    /// Optional one-line description of what the agent does.
    #[serde(default)]
    pub description: Option<String>,

    /// Optional semantic version of the blueprint itself.
    #[serde(default)]
    pub version: Option<String>,

    /// The persona — the system prompt that defines the agent's role.
    #[serde(default)]
    pub persona: BlueprintPersona,

    /// The primary LLM provider/model. When omitted, falls back to the ambient
    /// `Config.llm`.
    #[serde(default)]
    pub llm: Option<LLMConfig>,

    /// Tool names the agent may call (e.g. `["web_fetch", "shell_exec"]`).
    /// Empty means "use the default tool registry".
    #[serde(default)]
    pub tools: Vec<String>,

    /// Whether sensitive tool calls require human approval (HITL).
    #[serde(default)]
    pub require_approval: bool,

    /// Maximum agent-loop iterations.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Optional security override (allow-lists, TLS, etc.). When omitted, uses
    /// `SecurityConfig::default()`.
    #[serde(default)]
    pub security: Option<SecurityConfig>,
}

fn default_max_iterations() -> usize {
    10
}

/// The persona component of a blueprint.
///
/// # Stability
/// This struct is `#[non_exhaustive]` — new fields may be added in minor releases.
#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
pub struct BlueprintPersona {
    /// The system prompt / persona.
    #[serde(default)]
    pub system_prompt: String,
}

impl Default for AgentBlueprint {
    fn default() -> Self {
        Self {
            name: "assistant".to_string(),
            description: None,
            version: None,
            persona: BlueprintPersona::default(),
            llm: None,
            tools: Vec::new(),
            require_approval: false,
            max_iterations: default_max_iterations(),
            security: None,
        }
    }
}

impl AgentBlueprint {
    /// Parse a blueprint from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, BlueprintError> {
        toml::from_str(toml_str).map_err(|e| BlueprintError::Parse(e.to_string()))
    }

    /// Parse a blueprint from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, BlueprintError> {
        serde_json::from_str(json_str).map_err(|e| BlueprintError::Parse(e.to_string()))
    }

    /// Validate the blueprint's internal consistency.
    ///
    /// Returns an error if the name is empty or the max_iterations is zero.
    pub fn validate(&self) -> Result<(), BlueprintError> {
        if self.name.trim().is_empty() {
            return Err(BlueprintError::Validation(
                "blueprint name must not be empty".to_string(),
            ));
        }
        if self.max_iterations == 0 {
            return Err(BlueprintError::Validation(
                "max_iterations must be >= 1".to_string(),
            ));
        }
        Ok(())
    }

    /// Materialize the blueprint into a runnable `Config`, optionally layering
    /// over a base configuration for fields the blueprint doesn't override
    /// (provider fallback, runtime, telemetry, etc.).
    pub fn to_config(&self, base: Option<&Config>) -> Config {
        let base = base.cloned().unwrap_or_default();

        let mut cfg = Config {
            // Override the LLM when the blueprint specifies one; otherwise keep
            // the base single-provider config.
            llm: self.llm.clone().unwrap_or(base.llm),
            // Override the system prompt from the persona.
            ..base
        };
        cfg.llm.system_prompt = self.persona.system_prompt.clone();

        // Override security if provided.
        if let Some(security) = &self.security {
            cfg.security = security.clone();
        }

        cfg
    }

    /// The tool names this blueprint exposes. Empty means "use defaults".
    #[allow(dead_code)] // library accessor; the binary uses `to_config()`
    pub fn enabled_tools(&self) -> &[String] {
        &self.tools
    }
}

/// Errors that can occur when parsing, serializing, or validating a blueprint.
#[derive(Debug, thiserror::Error)]
pub enum BlueprintError {
    #[error("Failed to parse blueprint: {0}")]
    Parse(String),
    #[error("Invalid blueprint: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
name = "researcher"
description = "Fetches and summarizes web sources"
version = "1.0.0"
tools = ["web_fetch", "web_search"]
require_approval = false
max_iterations = 5

[persona]
system_prompt = "You are a thorough research assistant."

[llm]
provider = "openai-compatible"
endpoint = "http://localhost:11434"
model = "llama3.1"
"#
    }

    #[test]
    fn test_parse_toml() {
        let bp = AgentBlueprint::from_toml(sample_toml()).unwrap();
        assert_eq!(bp.name, "researcher");
        assert_eq!(bp.description.as_deref(), Some("Fetches and summarizes web sources"));
        assert_eq!(bp.version.as_deref(), Some("1.0.0"));
        assert_eq!(bp.persona.system_prompt, "You are a thorough research assistant.");
        assert_eq!(bp.tools, vec!["web_fetch", "web_search"]);
        assert_eq!(bp.max_iterations, 5);
        assert!(!bp.require_approval);
        assert!(bp.validate().is_ok());
    }

    #[test]
    fn test_defaults() {
        let bp = AgentBlueprint::default();
        assert_eq!(bp.name, "assistant");
        assert_eq!(bp.max_iterations, 10);
        assert!(bp.llm.is_none());
        assert!(bp.security.is_none());
        assert!(bp.tools.is_empty());
    }

    #[test]
    fn test_validate_rejects_empty_name() {
        let bp = AgentBlueprint {
            name: "   ".to_string(),
            ..Default::default()
        };
        assert!(bp.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_zero_iterations() {
        let bp = AgentBlueprint {
            max_iterations: 0,
            ..Default::default()
        };
        assert!(bp.validate().is_err());
    }

    #[test]
    fn test_from_json() {
        let json = r#"{
            "name": "researcher",
            "persona": { "system_prompt": "You are a researcher." },
            "llm": {
                "provider": "ollama",
                "endpoint": "http://localhost:11434",
                "model": "llama3.1"
            },
            "tools": ["web_fetch"],
            "max_iterations": 4
        }"#;
        let bp = AgentBlueprint::from_json(json).unwrap();
        assert_eq!(bp.name, "researcher");
        assert_eq!(bp.persona.system_prompt, "You are a researcher.");
        assert_eq!(bp.tools, vec!["web_fetch"]);
        assert_eq!(bp.max_iterations, 4);
    }

    #[test]
    fn test_to_config_overrides_llm_and_persona() {
        let bp = AgentBlueprint::from_toml(sample_toml()).unwrap();
        let cfg = bp.to_config(None);

        assert_eq!(cfg.llm.system_prompt, "You are a thorough research assistant.");
        assert_eq!(cfg.llm.model, "llama3.1");
        assert_eq!(cfg.llm.provider, crate::config::LLMProvider::OpenAICompatible);
        assert_eq!(cfg.llm.endpoint, "http://localhost:11434");
    }

    #[test]
    fn test_to_config_keeps_base_when_no_llm() {
        let bp = AgentBlueprint {
            name: "plain".to_string(),
            ..Default::default()
        };
        let base = Config::default();
        let cfg = bp.to_config(Some(&base));
        // No LLM override → falls back to base.llm (default endpoint).
        assert_eq!(cfg.llm.endpoint, base.llm.endpoint);
    }

    #[test]
    fn test_enabled_tools() {
        let bp = AgentBlueprint::from_toml(sample_toml()).unwrap();
        assert_eq!(bp.enabled_tools(), &["web_fetch", "web_search"]);
    }
}
