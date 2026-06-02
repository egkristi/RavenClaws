//! Error types for RavenClaw

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RavenClawError {
    #[error("LLM error: {0}")]
    LLM(#[from] crate::llm::LLMError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),
    
    #[error("RavenFabric error: {0}")]
    RavenFabric(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("Command execution failed: {0}")]
    CommandExecution(String),
    
    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

pub type Result<T> = std::result::Result<T, RavenClawError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_llm_error_variant() {
        let err = RavenClawError::LLM(crate::llm::LLMError::RequestFailed("timeout".to_string()));
        assert_eq!(format!("{}", err), "LLM error: Request failed: timeout");
    }
    
    #[test]
    fn test_config_error_variant() {
        let err = RavenClawError::Config(crate::config::ConfigError::ValidationError("bad field".to_string()));
        assert_eq!(format!("{}", err), "Configuration error: Invalid configuration: bad field");
    }
    
    #[test]
    fn test_ravenfabric_error_variant() {
        let err = RavenClawError::RavenFabric("connection refused".to_string());
        assert_eq!(format!("{}", err), "RavenFabric error: connection refused");
    }
    
    #[test]
    fn test_command_execution_error_variant() {
        let err = RavenClawError::CommandExecution("command failed".to_string());
        assert_eq!(format!("{}", err), "Command execution failed: command failed");
    }
    
    #[test]
    fn test_security_violation_error_variant() {
        let err = RavenClawError::SecurityViolation("unauthorized access".to_string());
        assert_eq!(format!("{}", err), "Security violation: unauthorized access");
    }
    
    #[test]
    fn test_result_type_alias() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
        
        let err: Result<i32> = Err(RavenClawError::CommandExecution("fail".to_string()));
        assert!(err.is_err());
    }
}
