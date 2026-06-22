//! Tamper-evident audit logging for RavenClaw
//!
//! Every tool call, policy decision, and approval action is recorded
//! in a structured audit log with HMAC chaining for tamper detection.
//!
//! # Architecture
//!
//! Each audit entry contains:
//! - A sequential index
//! - A timestamp
//! - The event type and details
//! - An HMAC-SHA256 of the previous entry's HMAC + current entry data
//!
//! This creates a hash chain: any modification to an entry breaks the chain
//! for all subsequent entries, making tampering detectable.
//!
//! # Security Properties
//!
//! - **Tamper-evident**: HMAC chain links each entry to the previous one
//! - **Append-only**: Entries can only be added, never removed or modified
//! - **Verifiable**: The chain can be verified at any time
//! - **Keyed**: HMAC uses a secret key known only to the audit system

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;
use tracing::info;
use zeroize::Zeroize;

// ── Error types ────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum AuditError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HMAC verification failed at entry {0}")]
    TamperDetected(u64),

    #[error("Audit log is empty")]
    EmptyLog,

    #[error("Audit log not initialized")]
    NotInitialized,
}

// ── Event types ────────────────────────────────────────────────────────────

/// Types of audit events
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// A tool was called
    ToolCall,
    /// A tool execution completed
    ToolResult,
    /// A policy decision was made
    PolicyDecision,
    /// An approval was requested
    ApprovalRequested,
    /// An approval was granted
    ApprovalGranted,
    /// An approval was denied
    ApprovalDenied,
    /// A sandbox violation occurred
    SandboxViolation,
    /// A security violation occurred (e.g., prompt injection detected)
    SecurityViolation,
    /// The agent started
    AgentStart,
    /// The agent finished
    AgentFinish,
    /// An error occurred
    Error,
    /// A configuration change
    ConfigChange,
    /// A custom event
    Custom(String),
}

/// A single audit log entry
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Sequential index (0-based)
    pub index: u64,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// The type of event
    pub event_type: AuditEventType,
    /// The tool or component name
    pub component: String,
    /// A human-readable description
    pub description: String,
    /// Arbitrary metadata (JSON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// The session ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// HMAC-SHA256 of (previous_hmac || index || timestamp || event_type || component || description || metadata)
    pub hmac: String,
}

/// The audit log — an append-only, tamper-evident log
#[allow(dead_code)]
pub struct AuditLog {
    /// The secret key for HMAC
    key: Vec<u8>,
    /// The entries (protected by mutex for thread safety)
    entries: Mutex<Vec<AuditEntry>>,
    /// The session ID
    session_id: String,
    /// Whether to also log to tracing
    trace_logging: bool,
    /// Optional file path for persistence
    file_path: Option<PathBuf>,
}

/// Zeroize the HMAC secret key on drop
impl Drop for AuditLog {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

#[allow(dead_code)]
impl AuditLog {
    /// Create a new audit log with a random key
    pub fn new(session_id: String) -> Self {
        use rand::RngCore;
        let mut key = vec![0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);

        Self {
            key,
            entries: Mutex::new(Vec::new()),
            session_id,
            trace_logging: true,
            file_path: None,
        }
    }

    /// Create a new audit log with a specific key (for testing or recovery)
    pub fn with_key(session_id: String, key: Vec<u8>) -> Self {
        Self {
            key,
            entries: Mutex::new(Vec::new()),
            session_id,
            trace_logging: true,
            file_path: None,
        }
    }

    /// Enable or disable tracing logging
    pub fn set_trace_logging(&mut self, enabled: bool) {
        self.trace_logging = enabled;
    }

    /// Set the file path for persistence
    pub fn set_file_path(&mut self, path: PathBuf) {
        self.file_path = Some(path);
    }

    /// Append an entry to the audit log
    pub fn append(
        &self,
        event_type: AuditEventType,
        component: &str,
        description: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<AuditEntry, AuditError> {
        let mut entries = self.entries.lock().unwrap();

        let index = entries.len() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Compute the HMAC
        let hmac = self.compute_hmac(
            index,
            &timestamp,
            &event_type,
            component,
            description,
            &metadata,
            entries.last(),
        )?;

        let entry = AuditEntry {
            index,
            timestamp,
            event_type: event_type.clone(),
            component: component.to_string(),
            description: description.to_string(),
            metadata,
            session_id: Some(self.session_id.clone()),
            hmac,
        };

        if self.trace_logging {
            info!(
                audit.index = entry.index,
                audit.event = ?entry.event_type,
                audit.component = %entry.component,
                "Audit: {}",
                entry.description
            );
        }

        entries.push(entry.clone());

        // Persist to file if configured
        if let Some(path) = &self.file_path {
            self.persist_to_file(path, &entries)?;
        }

        Ok(entry)
    }

    /// Convenience method for logging a tool call
    pub fn tool_call(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<AuditEntry, AuditError> {
        self.append(
            AuditEventType::ToolCall,
            tool_name,
            &format!("Tool call: {}", tool_name),
            Some(serde_json::json!({"arguments": args})),
        )
    }

    /// Convenience method for logging a tool result
    pub fn tool_result(
        &self,
        tool_name: &str,
        result: &crate::tools::ToolResult,
    ) -> Result<AuditEntry, AuditError> {
        self.append(
            AuditEventType::ToolResult,
            tool_name,
            &format!("Tool result: {} (success: {})", tool_name, result.success),
            Some(serde_json::json!({
                "success": result.success,
                "exit_code": result.exit_code,
                "duration_ms": result.duration_ms,
                "output_length": result.output.len(),
            })),
        )
    }

    /// Convenience method for logging a policy decision
    pub fn policy_decision(
        &self,
        tool_name: &str,
        allowed: bool,
        reason: Option<&str>,
    ) -> Result<AuditEntry, AuditError> {
        self.append(
            AuditEventType::PolicyDecision,
            "policy",
            &format!(
                "Policy decision for '{}': {}",
                tool_name,
                if allowed { "allowed" } else { "denied" }
            ),
            Some(serde_json::json!({
                "tool": tool_name,
                "allowed": allowed,
                "reason": reason,
            })),
        )
    }

    /// Convenience method for logging an approval
    pub fn approval(
        &self,
        tool_name: &str,
        granted: bool,
        reason: Option<&str>,
    ) -> Result<AuditEntry, AuditError> {
        let event_type = if granted {
            AuditEventType::ApprovalGranted
        } else {
            AuditEventType::ApprovalDenied
        };

        self.append(
            event_type,
            "approval",
            &format!(
                "Approval for '{}': {}",
                tool_name,
                if granted { "granted" } else { "denied" }
            ),
            Some(serde_json::json!({
                "tool": tool_name,
                "granted": granted,
                "reason": reason,
            })),
        )
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().unwrap().clone()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Check if the log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }

    /// Verify the integrity of the entire audit log
    pub fn verify(&self) -> Result<(), AuditError> {
        let entries = self.entries.lock().unwrap();

        if entries.is_empty() {
            return Err(AuditError::EmptyLog);
        }

        let mut prev_hmac = String::new();

        for entry in entries.iter() {
            let expected_hmac = self.compute_hmac_for_verification(
                entry.index,
                &entry.timestamp,
                &entry.event_type,
                &entry.component,
                &entry.description,
                &entry.metadata,
                &prev_hmac,
            )?;

            if entry.hmac != expected_hmac {
                return Err(AuditError::TamperDetected(entry.index));
            }

            prev_hmac = entry.hmac.clone();
        }

        Ok(())
    }

    /// Export the audit log as JSON
    pub fn to_json(&self) -> Result<String, AuditError> {
        let entries = self.entries.lock().unwrap();
        Ok(serde_json::to_string_pretty(&*entries)?)
    }

    /// Export the audit log as JSON lines (one entry per line)
    pub fn to_json_lines(&self) -> Result<String, AuditError> {
        let entries = self.entries.lock().unwrap();
        let mut lines = String::new();
        for entry in entries.iter() {
            lines.push_str(&serde_json::to_string(entry)?);
            lines.push('\n');
        }
        Ok(lines)
    }

    // ── Private helpers ────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn compute_hmac(
        &self,
        index: u64,
        timestamp: &str,
        event_type: &AuditEventType,
        component: &str,
        description: &str,
        metadata: &Option<serde_json::Value>,
        prev_entry: Option<&AuditEntry>,
    ) -> Result<String, AuditError> {
        let prev_hmac = prev_entry.map(|e| e.hmac.as_str()).unwrap_or("");

        self.compute_hmac_for_verification(
            index,
            timestamp,
            event_type,
            component,
            description,
            metadata,
            prev_hmac,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_hmac_for_verification(
        &self,
        index: u64,
        timestamp: &str,
        event_type: &AuditEventType,
        component: &str,
        description: &str,
        metadata: &Option<serde_json::Value>,
        prev_hmac: &str,
    ) -> Result<String, AuditError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|e| {
            AuditError::Serialization(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                e.to_string(),
            )))
        })?;

        mac.update(prev_hmac.as_bytes());
        mac.update(&index.to_le_bytes());
        mac.update(timestamp.as_bytes());
        mac.update(serde_json::to_string(event_type)?.as_bytes());
        mac.update(component.as_bytes());
        mac.update(description.as_bytes());

        if let Some(m) = metadata {
            mac.update(serde_json::to_string(m)?.as_bytes());
        }

        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    fn persist_to_file(&self, path: &PathBuf, entries: &[AuditEntry]) -> Result<(), AuditError> {
        let json = serde_json::to_string_pretty(entries)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_log() -> AuditLog {
        AuditLog::with_key("test-session".to_string(), vec![0u8; 32])
    }

    #[test]
    fn test_audit_log_empty() {
        let log = create_test_log();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_audit_log_append() {
        let log = create_test_log();
        let entry = log
            .append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        assert_eq!(entry.index, 0);
        assert_eq!(entry.component, "agent");
        assert_eq!(entry.event_type, AuditEventType::AgentStart);
        assert!(!entry.hmac.is_empty());
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_audit_log_multiple_entries() {
        let log = create_test_log();

        log.append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        log.append(
            AuditEventType::ToolCall,
            "shell_exec",
            "Executing command",
            Some(serde_json::json!({"command": "echo hello"})),
        )
        .unwrap();

        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_audit_log_verify_valid() {
        let log = create_test_log();

        log.append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        log.append(
            AuditEventType::ToolCall,
            "shell_exec",
            "Executing command",
            None,
        )
        .unwrap();

        log.append(
            AuditEventType::ToolResult,
            "shell_exec",
            "Command completed",
            None,
        )
        .unwrap();

        assert!(log.verify().is_ok());
    }

    #[test]
    fn test_audit_log_verify_tampered() {
        let log = create_test_log();

        log.append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        log.append(
            AuditEventType::ToolCall,
            "shell_exec",
            "Executing command",
            None,
        )
        .unwrap();

        // Tamper with the second entry
        {
            let mut entries = log.entries.lock().unwrap();
            entries[1].description = "Tampered!".to_string();
        }

        let result = log.verify();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuditError::TamperDetected(1)));
    }

    #[test]
    fn test_audit_log_verify_empty() {
        let log = create_test_log();
        let result = log.verify();
        assert!(matches!(result.unwrap_err(), AuditError::EmptyLog));
    }

    #[test]
    fn test_audit_log_tool_call() {
        let log = create_test_log();
        let args = serde_json::json!({"command": "echo hello"});
        let entry = log.tool_call("shell_exec", &args).unwrap();

        assert_eq!(entry.event_type, AuditEventType::ToolCall);
        assert_eq!(entry.component, "shell_exec");
    }

    #[test]
    fn test_audit_log_policy_decision() {
        let log = create_test_log();
        let entry = log.policy_decision("shell_exec", true, None).unwrap();

        assert_eq!(entry.event_type, AuditEventType::PolicyDecision);
        assert_eq!(entry.component, "policy");
    }

    #[test]
    fn test_audit_log_approval_granted() {
        let log = create_test_log();
        let entry = log
            .approval("shell_exec", true, Some("User approved"))
            .unwrap();

        assert_eq!(entry.event_type, AuditEventType::ApprovalGranted);
    }

    #[test]
    fn test_audit_log_approval_denied() {
        let log = create_test_log();
        let entry = log
            .approval("shell_exec", false, Some("User denied"))
            .unwrap();

        assert_eq!(entry.event_type, AuditEventType::ApprovalDenied);
    }

    #[test]
    fn test_audit_log_to_json() {
        let log = create_test_log();

        log.append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        let json = log.to_json().unwrap();
        assert!(json.contains("Agent started"));
        assert!(json.contains("agent_start"));
    }

    #[test]
    fn test_audit_log_to_json_lines() {
        let log = create_test_log();

        log.append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        log.append(
            AuditEventType::ToolCall,
            "shell_exec",
            "Executing command",
            None,
        )
        .unwrap();

        let lines = log.to_json_lines().unwrap();
        let line_count = lines.lines().count();
        assert_eq!(line_count, 2);
    }

    #[test]
    fn test_audit_log_session_id() {
        let log = AuditLog::new("my-session-123".to_string());
        let entry = log
            .append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        assert_eq!(entry.session_id.unwrap(), "my-session-123");
    }

    #[test]
    fn test_audit_log_different_keys_produce_different_hmacs() {
        let log1 = AuditLog::with_key("test".to_string(), vec![1u8; 32]);
        let log2 = AuditLog::with_key("test".to_string(), vec![2u8; 32]);

        let e1 = log1
            .append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        let e2 = log2
            .append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        assert_ne!(e1.hmac, e2.hmac);
    }

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            index: 0,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            event_type: AuditEventType::ToolCall,
            component: "shell_exec".to_string(),
            description: "Executed command".to_string(),
            metadata: Some(serde_json::json!({"command": "echo hello"})),
            session_id: Some("session-1".to_string()),
            hmac: "abc123".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("shell_exec"));
        assert!(json.contains("tool_call"));
        assert!(json.contains("echo hello"));
    }

    #[test]
    fn test_audit_error_tamper_detected() {
        let err = AuditError::TamperDetected(5);
        assert_eq!(format!("{}", err), "HMAC verification failed at entry 5");
    }

    #[test]
    fn test_audit_error_empty() {
        let err = AuditError::EmptyLog;
        assert_eq!(format!("{}", err), "Audit log is empty");
    }

    #[test]
    fn test_audit_event_type_custom() {
        let event = AuditEventType::Custom("my_event".to_string());
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "{\"custom\":\"my_event\"}");
    }

    #[test]
    fn test_audit_log_with_metadata() {
        let log = create_test_log();
        let metadata = serde_json::json!({
            "key1": "value1",
            "key2": 42,
            "nested": {"a": 1}
        });

        let entry = log
            .append(
                AuditEventType::ConfigChange,
                "config",
                "Configuration changed",
                Some(metadata),
            )
            .unwrap();

        assert!(entry.metadata.is_some());
        let meta = entry.metadata.unwrap();
        assert_eq!(meta["key1"], "value1");
        assert_eq!(meta["key2"], 42);
    }

    #[test]
    fn test_audit_log_trace_logging_disabled() {
        let mut log = create_test_log();
        log.set_trace_logging(false);

        let entry = log
            .append(AuditEventType::AgentStart, "agent", "Agent started", None)
            .unwrap();

        assert_eq!(entry.index, 0);
    }

    #[test]
    fn test_audit_log_chain_integrity() {
        let log = create_test_log();

        // Add several entries
        for i in 0..10 {
            log.append(
                AuditEventType::Custom(format!("event_{}", i)),
                "test",
                &format!("Entry {}", i),
                None,
            )
            .unwrap();
        }

        // Verify chain
        assert!(log.verify().is_ok());

        // Tamper with entry 3
        {
            let mut entries = log.entries.lock().unwrap();
            entries[3].description = "MODIFIED".to_string();
        }

        // Verification should fail
        assert!(log.verify().is_err());
    }
}
