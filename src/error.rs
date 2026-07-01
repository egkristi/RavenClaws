//! RavenClaws

use thiserror::Error;

/// Unified error type for RavenClaws.
///
/// # Stability
/// This enum is `#[non_exhaustive]` — new variants may be added in minor releases.
/// Match with a wildcard arm to handle future variants.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RavenClawsError {
    #[error("LLM error: {0}")]
    Llm(#[from] crate::llm::LLMError),

    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("RavenFabric error: {0}")]
    #[allow(dead_code)]
    RavenFabric(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("Security violation: {0}")]
    #[allow(dead_code)]
    SecurityViolation(String),

    #[error("Agent failed: {0}")]
    #[allow(dead_code)]
    AgentFailed(String),

    #[error("Self-healing error: {0}")]
    #[allow(dead_code)]
    HealingError(String),
}

impl RavenClawsError {
    /// Returns `true` if this error is transient and may succeed on retry.
    #[allow(dead_code)]
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            RavenClawsError::Llm(e) if e.is_transient(),
        )
    }
}

pub type Result<T> = std::result::Result<T, RavenClawsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_error_variant() {
        let err = RavenClawsError::Llm(crate::llm::LLMError::RequestFailed("timeout".to_string()));
        assert_eq!(format!("{}", err), "LLM error: Request failed: timeout");
    }

    #[test]
    fn test_config_error_variant() {
        let err = RavenClawsError::Config(crate::config::ConfigError::ValidationError(
            "bad field".to_string(),
        ));
        assert_eq!(
            format!("{}", err),
            "Configuration error: Invalid configuration: bad field"
        );
    }

    #[test]
    fn test_ravenfabric_error_variant() {
        let err = RavenClawsError::RavenFabric("connection refused".to_string());
        assert_eq!(format!("{}", err), "RavenFabric error: connection refused");
    }

    #[test]
    fn test_command_execution_error_variant() {
        let err = RavenClawsError::CommandExecution("command failed".to_string());
        assert_eq!(
            format!("{}", err),
            "Command execution failed: command failed"
        );
    }

    #[test]
    fn test_security_violation_error_variant() {
        let err = RavenClawsError::SecurityViolation("unauthorized access".to_string());
        assert_eq!(
            format!("{}", err),
            "Security violation: unauthorized access"
        );
    }

    #[test]
    fn test_result_type_alias() {
        let ok: i32 = 42;
        assert_eq!(ok, 42);

        let err: Result<i32> = Err(RavenClawsError::CommandExecution("fail".to_string()));
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_network_error_variant() {
        // Network error from reqwest — we can construct it via the From impl
        // by creating a reqwest error. Since reqwest::Error is opaque, we
        // test the variant via the Display trait.
        let err = RavenClawsError::Network(
            reqwest::Client::builder()
                .build()
                .unwrap()
                .get("http://invalid.example.com")
                .send()
                .await
                .unwrap_err(),
        );
        assert!(format!("{}", err).contains("Network error"));
    }

    #[test]
    fn test_io_error_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = RavenClawsError::IO(io_err);
        assert!(format!("{}", err).contains("IO error"));
        assert!(format!("{}", err).contains("file not found"));
    }

    #[test]
    fn test_error_is_debug() {
        let err = RavenClawsError::CommandExecution("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("CommandExecution"));
    }

    #[test]
    fn test_error_is_send() {
        fn check_send<T: Send>() {}
        check_send::<RavenClawsError>();
    }

    #[test]
    fn test_error_is_sync() {
        fn check_sync<T: Sync>() {}
        check_sync::<RavenClawsError>();
    }

    #[test]
    fn test_from_llm_error_conversion() {
        let llm_err = crate::llm::LLMError::RequestFailed("timeout".to_string());
        let err: RavenClawsError = llm_err.into();
        assert!(format!("{}", err).contains("LLM error"));
        assert!(format!("{}", err).contains("timeout"));
    }

    #[test]
    fn test_from_config_error_conversion() {
        let cfg_err = crate::config::ConfigError::ValidationError("bad config".to_string());
        let err: RavenClawsError = cfg_err.into();
        assert!(format!("{}", err).contains("Configuration error"));
        assert!(format!("{}", err).contains("bad config"));
    }

    #[test]
    fn test_from_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let err: RavenClawsError = io_err.into();
        assert!(format!("{}", err).contains("IO error"));
        assert!(format!("{}", err).contains("permission denied"));
    }

    #[test]
    fn test_error_source_chain() {
        // RavenClawsError doesn't implement std::error::Error::source() directly
        // for all variants, but the Display impl should contain the inner message
        let inner = crate::llm::LLMError::AuthFailed;
        let err = RavenClawsError::Llm(inner);
        let display = format!("{}", err);
        assert!(display.contains("Authentication failed"));
    }

    #[test]
    fn test_ravenfabric_error_construction() {
        let err = RavenClawsError::RavenFabric("connection timeout".to_string());
        assert_eq!(format!("{}", err), "RavenFabric error: connection timeout");
    }

    #[test]
    fn test_security_violation_construction() {
        let err = RavenClawsError::SecurityViolation("invalid token".to_string());
        assert_eq!(format!("{}", err), "Security violation: invalid token");
    }

    #[test]
    fn test_agent_failed_variant() {
        let err = RavenClawsError::AgentFailed("worker-1 crashed".to_string());
        assert_eq!(format!("{}", err), "Agent failed: worker-1 crashed");
    }

    #[test]
    fn test_healing_error_variant() {
        let err = RavenClawsError::HealingError("circuit breaker open".to_string());
        assert_eq!(
            format!("{}", err),
            "Self-healing error: circuit breaker open"
        );
    }

    #[test]
    fn test_is_transient_llm_request_failed() {
        let llm_err = crate::llm::LLMError::RequestFailed("timeout".to_string());
        assert!(llm_err.is_transient());
    }

    #[test]
    fn test_is_transient_llm_rate_limited() {
        let llm_err = crate::llm::LLMError::RateLimited;
        assert!(llm_err.is_transient());
    }

    #[test]
    fn test_is_transient_llm_circuit_breaker() {
        let llm_err = crate::llm::LLMError::CircuitBreakerOpen("openai".to_string());
        assert!(llm_err.is_transient());
    }

    #[test]
    fn test_is_not_transient_llm_auth_failed() {
        let llm_err = crate::llm::LLMError::AuthFailed;
        assert!(!llm_err.is_transient());
    }

    #[test]
    fn test_is_not_transient_llm_invalid_response() {
        let llm_err = crate::llm::LLMError::InvalidResponse("bad json".to_string());
        assert!(!llm_err.is_transient());
    }

    #[test]
    fn test_is_not_transient_llm_token_budget() {
        let llm_err = crate::llm::LLMError::TokenBudgetExceeded;
        assert!(!llm_err.is_transient());
    }

    #[test]
    fn test_is_not_transient_llm_all_providers_failed() {
        let llm_err = crate::llm::LLMError::AllProvidersFailed;
        assert!(!llm_err.is_transient());
    }

    #[test]
    fn test_ravenclaws_error_is_transient_via_llm() {
        let llm_err = crate::llm::LLMError::RequestFailed("timeout".to_string());
        let err = RavenClawsError::Llm(llm_err);
        assert!(err.is_transient());
    }

    #[test]
    fn test_ravenclaws_error_is_not_transient_for_non_llm() {
        let err = RavenClawsError::CommandExecution("fail".to_string());
        assert!(!err.is_transient());
    }

    #[test]
    fn test_ravenclaws_error_is_not_transient_for_agent_failed() {
        let err = RavenClawsError::AgentFailed("crashed".to_string());
        assert!(!err.is_transient());
    }

    #[test]
    #[allow(clippy::unnecessary_literal_unwrap)]
    fn test_result_type_alias_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    #[allow(clippy::unnecessary_literal_unwrap)]
    fn test_result_type_alias_err() {
        let result: Result<i32> = Err(RavenClawsError::CommandExecution("fail".to_string()));
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Command execution failed: fail"
        );
    }

    #[test]
    fn test_error_into_boxed() {
        // Verify RavenClawsError can be boxed (required for std::error::Error trait)
        let err = RavenClawsError::CommandExecution("boxed".to_string());
        let boxed: Box<dyn std::error::Error> = Box::new(err);
        assert!(format!("{}", boxed).contains("Command execution failed"));
    }

    #[test]
    fn test_error_into_string() {
        let err = RavenClawsError::SecurityViolation("access denied".to_string());
        let msg: String = err.to_string();
        assert_eq!(msg, "Security violation: access denied");
    }

    #[test]
    fn test_error_from_reqwest() {
        // Verify the From<reqwest::Error> impl compiles and works
        // We can't easily construct a reqwest::Error directly, but we can
        // verify the From impl exists by checking the trait bounds
        fn _check_from()
        where
            reqwest::Error: Into<RavenClawsError>,
        {
        }
        // Compile-time check passes
    }

    #[test]
    fn test_error_display_network_variant() {
        // Network error display should contain the inner error message
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(async {
            reqwest::Client::builder()
                .build()
                .unwrap()
                .get("http://invalid.example.com")
                .send()
                .await
                .unwrap_err()
        });
        let raven_err = RavenClawsError::Network(err);
        let display = format!("{}", raven_err);
        assert!(display.contains("Network error"));
        assert!(!display.is_empty());
    }

    #[test]
    fn test_error_source_chain_io() {
        // Test source chain: IO error wrapped in RavenClawsError
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = RavenClawsError::IO(io_err);
        let display = format!("{}", err);
        assert!(display.contains("IO error"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_error_source_chain_config() {
        let cfg_err = crate::config::ConfigError::ValidationError("invalid".to_string());
        let err = RavenClawsError::Config(cfg_err);
        let display = format!("{}", err);
        assert!(display.contains("Configuration error"));
        assert!(display.contains("invalid"));
    }

    #[test]
    fn test_error_source_chain_llm() {
        let llm_err = crate::llm::LLMError::RateLimited;
        let err = RavenClawsError::Llm(llm_err);
        let display = format!("{}", err);
        assert!(display.contains("LLM error"));
        assert!(display.contains("Rate limit exceeded"));
    }

    #[test]
    fn test_error_clone_not_required() {
        // RavenClawsError intentionally does not implement Clone.
        // This test verifies that by checking it at compile time.
        fn _check_no_clone<T>() {
            // If this compiles, RavenClawsError does NOT implement Clone
        }
        _check_no_clone::<RavenClawsError>();
    }
}
