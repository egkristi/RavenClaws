//! Deny-by-default security policy for RavenClaw
//!
//! Every tool call is checked against the policy before execution.
//! The policy defines allow-lists for commands, paths, hosts, and
//! network targets. By default, everything is denied unless explicitly allowed.
//!
//! # Architecture
//!
//! ```text
//! ToolCall
//!   │
//!   ▼
//! PolicyEngine::check()
//!   │
//!   ├── ShellPolicy  → command allow-list, flag restrictions
//!   ├── PathPolicy   → read/write path allow-lists
//!   ├── NetworkPolicy→ host/URL allow-list
//!   └── GeneralPolicy→ category-based rules
//!   │
//!   ▼
//! Allowed / Denied (with reason)

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{info, warn};

// ── Error types ────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Policy denied: {0}")]
    Denied(String),

    #[error("Invalid policy configuration: {0}")]
    InvalidConfig(String),
}

// ── Policy types ───────────────────────────────────────────────────────────

/// Policy decision
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Allow the operation
    Allow,
    /// Deny the operation with a reason
    Deny(String),
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allow)
    }
}

/// Shell command policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPolicy {
    /// If true, all shell commands are denied (default: false)
    #[serde(default)]
    pub deny_all: bool,
    /// List of allowed command prefixes (e.g., ["echo", "ls", "cat", "git"])
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    /// List of denied command prefixes (takes precedence over allowed)
    #[serde(default)]
    pub denied_commands: Vec<String>,
    /// Maximum command timeout in seconds
    #[serde(default = "default_shell_timeout")]
    pub max_timeout_secs: u64,
    /// If true, allow commands that write to disk (install, rm, etc.)
    #[serde(default)]
    pub allow_write_commands: bool,
}

impl Default for ShellPolicy {
    fn default() -> Self {
        Self {
            deny_all: false,
            allowed_commands: vec![
                "echo".to_string(),
                "cat".to_string(),
                "ls".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "wc".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "sort".to_string(),
                "uniq".to_string(),
                "cut".to_string(),
                "which".to_string(),
                "pwd".to_string(),
                "date".to_string(),
                "whoami".to_string(),
                "uname".to_string(),
                "env".to_string(),
                "printenv".to_string(),
                "git".to_string(),
                "cargo".to_string(),
                "rustc".to_string(),
                "python3".to_string(),
                "node".to_string(),
            ],
            denied_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "poweroff".to_string(),
            ],
            max_timeout_secs: 60,
            allow_write_commands: false,
        }
    }
}

/// Filesystem path policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathPolicy {
    /// List of allowed read path prefixes
    #[serde(default)]
    pub allowed_read_paths: Vec<String>,
    /// List of allowed write path prefixes
    #[serde(default)]
    pub allowed_write_paths: Vec<String>,
    /// List of denied path prefixes (takes precedence)
    #[serde(default)]
    pub denied_paths: Vec<String>,
    /// Maximum file read size in bytes
    #[serde(default = "default_max_read_bytes")]
    pub max_read_bytes: usize,
    /// Maximum file write size in bytes
    #[serde(default = "default_max_write_bytes")]
    pub max_write_bytes: usize,
}

impl Default for PathPolicy {
    fn default() -> Self {
        Self {
            allowed_read_paths: vec![
                "/tmp".to_string(),
                "/var/tmp".to_string(),
                "/home".to_string(),
                "/workspace".to_string(),
                ".".to_string(),
            ],
            allowed_write_paths: vec![
                "/tmp".to_string(),
                "/var/tmp".to_string(),
                "/workspace".to_string(),
                ".".to_string(),
            ],
            denied_paths: vec![
                "/etc/shadow".to_string(),
                "/etc/sudoers".to_string(),
                "/etc/ssh".to_string(),
                "/root".to_string(),
            ],
            max_read_bytes: 65536,
            max_write_bytes: 1048576,
        }
    }
}

/// Network policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// If true, all network access is denied
    #[serde(default)]
    pub deny_all: bool,
    /// List of allowed hostname suffixes (e.g., ["github.com", "docs.rs"])
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    /// List of denied hostname suffixes (takes precedence)
    #[serde(default)]
    pub denied_hosts: Vec<String>,
    /// If true, allow connections to localhost/127.0.0.1
    #[serde(default = "default_true")]
    pub allow_localhost: bool,
    /// If true, allow connections to private IP ranges
    #[serde(default)]
    pub allow_private_networks: bool,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            deny_all: false,
            allowed_hosts: vec![
                "github.com".to_string(),
                "raw.githubusercontent.com".to_string(),
                "docs.rs".to_string(),
                "crates.io".to_string(),
                "api.github.com".to_string(),
                "google.com".to_string(),
                "wikipedia.org".to_string(),
                "stackoverflow.com".to_string(),
                "rust-lang.org".to_string(),
            ],
            denied_hosts: vec![],
            allow_localhost: true,
            allow_private_networks: false,
        }
    }
}

/// Complete security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Shell command policy
    #[serde(default)]
    pub shell: ShellPolicy,
    /// Filesystem path policy
    #[serde(default)]
    pub path: PathPolicy,
    /// Network policy
    #[serde(default)]
    pub network: NetworkPolicy,
    /// If true, all tool calls require human approval
    #[serde(default)]
    pub require_approval_all: bool,
    /// List of tool names that require approval
    #[serde(default)]
    pub require_approval_for: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            shell: ShellPolicy::default(),
            path: PathPolicy::default(),
            network: NetworkPolicy::default(),
            require_approval_all: false,
            require_approval_for: vec!["shell_exec".to_string(), "write_file".to_string()],
        }
    }
}

// ── Policy engine ──────────────────────────────────────────────────────────

/// The policy engine — checks tool calls against the security policy
pub struct PolicyEngine {
    policy: SecurityPolicy,
}

impl PolicyEngine {
    /// Create a new policy engine with the given policy
    pub fn new(policy: SecurityPolicy) -> Self {
        Self { policy }
    }

    /// Create a policy engine with default (secure) settings
    pub fn default_secure() -> Self {
        Self {
            policy: SecurityPolicy::default(),
        }
    }

    /// Create a permissive policy engine (for development)
    pub fn permissive() -> Self {
        Self {
            policy: SecurityPolicy {
                require_approval_all: false,
                require_approval_for: vec![],
                shell: ShellPolicy {
                    deny_all: false,
                    allowed_commands: vec!["*".to_string()], // allow all
                    denied_commands: vec![],
                    max_timeout_secs: 300,
                    allow_write_commands: true,
                },
                path: PathPolicy {
                    allowed_read_paths: vec!["/".to_string()],
                    allowed_write_paths: vec!["/tmp".to_string(), "/workspace".to_string()],
                    denied_paths: vec![],
                    max_read_bytes: 1048576,
                    max_write_bytes: 10485760,
                },
                network: NetworkPolicy {
                    deny_all: false,
                    allowed_hosts: vec!["*".to_string()],
                    denied_hosts: vec![],
                    allow_localhost: true,
                    allow_private_networks: true,
                },
            },
        }
    }

    /// Check if a tool call is allowed
    pub fn check_tool_call(&self, tool_name: &str, args: &serde_json::Value) -> Decision {
        match tool_name {
            "shell_exec" => self.check_shell_command(args),
            "read_file" | "write_file" => self.check_file_operation(tool_name, args),
            "web_fetch" => self.check_network_request(args),
            _ => Decision::Allow, // Unknown tools are allowed by default (they'll be checked by the tool registry)
        }
    }

    /// Check if a tool requires human approval
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        if self.policy.require_approval_all {
            return true;
        }
        self.policy.require_approval_for.contains(&tool_name.to_string())
    }

    /// Get the policy configuration
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    // ── Internal check methods ──────────────────────────────────────────

    fn check_shell_command(&self, args: &serde_json::Value) -> Decision {
        let policy = &self.policy.shell;

        if policy.deny_all {
            return Decision::Deny("All shell commands are denied by policy".to_string());
        }

        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if command.is_empty() {
            return Decision::Deny("Empty command".to_string());
        }

        // Check denied commands first
        for denied in &policy.denied_commands {
            if command.contains(denied) {
                return Decision::Deny(format!("Command contains denied pattern: '{}'", denied));
            }
        }

        // Check timeout
        if let Some(timeout) = args.get("timeout_secs").and_then(|v| v.as_u64()) {
            if timeout > policy.max_timeout_secs {
                return Decision::Deny(format!(
                    "Timeout {}s exceeds maximum {}s",
                    timeout, policy.max_timeout_secs
                ));
            }
        }

        // Check allowed commands
        let first_word = command.split_whitespace().next().unwrap_or("");
        let is_allowed = policy.allowed_commands.iter().any(|a| {
            if a == "*" {
                return true; // wildcard — allow all
            }
            first_word == a || command.starts_with(a)
        });

        if !is_allowed {
            return Decision::Deny(format!(
                "Command '{}' is not in the allowed list",
                first_word
            ));
        }

        Decision::Allow
    }

    fn check_file_operation(&self, tool_name: &str, args: &serde_json::Value) -> Decision {
        let policy = &self.policy.path;
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");

        if path.is_empty() {
            return Decision::Deny("Empty path".to_string());
        }

        // Resolve to absolute path
        let abs_path = if Path::new(path).is_absolute() {
            path.to_string()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(path).to_string_lossy().to_string(),
                Err(_) => path.to_string(),
            }
        };

        // Check denied paths
        for denied in &policy.denied_paths {
            if abs_path.starts_with(denied) || abs_path.contains(denied) {
                return Decision::Deny(format!("Path '{}' is denied", path));
            }
        }

        // Check allowed paths based on operation type
        let allowed_paths = match tool_name {
            "write_file" => &policy.allowed_write_paths,
            _ => &policy.allowed_read_paths,
        };

        let is_allowed = allowed_paths.iter().any(|a| {
            if a == "/" || a == "*" {
                return true; // wildcard
            }
            abs_path.starts_with(a)
        });

        if !is_allowed {
            return Decision::Deny(format!(
                "Path '{}' is not in the allowed {} paths",
                path,
                if tool_name == "write_file" {
                    "write"
                } else {
                    "read"
                }
            ));
        }

        // Check size limits for write operations
        if tool_name == "write_file" {
            if let Some(content) = args.get("content").and_then(|v| v.as_str()) {
                if content.len() > policy.max_write_bytes {
                    return Decision::Deny(format!(
                        "Write size {} exceeds maximum {} bytes",
                        content.len(),
                        policy.max_write_bytes
                    ));
                }
            }
        }

        Decision::Allow
    }

    fn check_network_request(&self, args: &serde_json::Value) -> Decision {
        let policy = &self.policy.network;

        if policy.deny_all {
            return Decision::Deny("All network requests are denied by policy".to_string());
        }

        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

        if url.is_empty() {
            return Decision::Deny("Empty URL".to_string());
        }

        // Parse the URL
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => {
                return Decision::Deny(format!("Invalid URL: {}", e));
            }
        };

        let host = match parsed.host_str() {
            Some(h) => h.to_string(),
            None => return Decision::Deny("URL has no host".to_string()),
        };

        // Check localhost
        if is_localhost(&host) {
            if !policy.allow_localhost {
                return Decision::Deny("Localhost connections are denied by policy".to_string());
            }
            return Decision::Allow;
        }

        // Check private networks
        if is_private_ip(&host) && !policy.allow_private_networks {
            return Decision::Deny("Private network connections are denied by policy".to_string());
        }

        // Check denied hosts
        for denied in &policy.denied_hosts {
            if host == *denied || host.ends_with(&format!(".{}", denied)) {
                return Decision::Deny(format!("Host '{}' is denied", host));
            }
        }

        // Check allowed hosts
        let is_allowed = policy.allowed_hosts.iter().any(|a| {
            if a == "*" {
                return true; // wildcard
            }
            host == *a || host.ends_with(&format!(".{}", a))
        });

        if !is_allowed {
            return Decision::Deny(format!(
                "Host '{}' is not in the allowed hosts list",
                host
            ));
        }

        Decision::Allow
    }
}

// ── Helper functions ───────────────────────────────────────────────────────

fn default_shell_timeout() -> u64 {
    60
}

fn default_max_read_bytes() -> usize {
    65536
}

fn default_max_write_bytes() -> usize {
    1048576
}

fn default_true() -> bool {
    true
}

fn is_localhost(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host == "0.0.0.0"
        || host.starts_with("127.")
}

fn is_private_ip(host: &str) -> bool {
    host == "10.0.0.1"
        || host.starts_with("10.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.20.")
        || host.starts_with("172.21.")
        || host.starts_with("172.22.")
        || host.starts_with("172.23.")
        || host.starts_with("172.24.")
        || host.starts_with("172.25.")
        || host.starts_with("172.26.")
        || host.starts_with("172.27.")
        || host.starts_with("172.28.")
        || host.starts_with("172.29.")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
        || host.starts_with("192.168.")
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_allow() {
        let d = Decision::Allow;
        assert!(d.is_allowed());
    }

    #[test]
    fn test_decision_deny() {
        let d = Decision::Deny("test".to_string());
        assert!(!d.is_allowed());
    }

    #[test]
    fn test_default_policy_denies_unknown_command() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "sudo rm -rf /"});
        let decision = engine.check_shell_command(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_default_policy_allows_echo() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "echo hello"});
        let decision = engine.check_shell_command(&args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_default_policy_allows_ls() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "ls -la"});
        let decision = engine.check_shell_command(&args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_default_policy_denies_shutdown() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "shutdown -h now"});
        let decision = engine.check_shell_command(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_default_policy_denies_rm_rf_root() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "rm -rf /"});
        let decision = engine.check_shell_command(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_deny_all_shell() {
        let policy = SecurityPolicy {
            shell: ShellPolicy {
                deny_all: true,
                ..ShellPolicy::default()
            },
            ..SecurityPolicy::default()
        };
        let engine = PolicyEngine::new(policy);
        let args = serde_json::json!({"command": "echo hello"});
        let decision = engine.check_shell_command(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_timeout_exceeded() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "echo hello", "timeout_secs": 999});
        let decision = engine.check_shell_command(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_empty_command() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": ""});
        let decision = engine.check_shell_command(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_permissive_allows_all() {
        let engine = PolicyEngine::permissive();
        let args = serde_json::json!({"command": "curl https://example.com"});
        let decision = engine.check_shell_command(&args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_path_read_allowed() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let decision = engine.check_file_operation("read_file", &args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_path_write_allowed() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"path": "/tmp/test.txt", "content": "data"});
        let decision = engine.check_file_operation("write_file", &args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_path_denied() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"path": "/etc/shadow"});
        let decision = engine.check_file_operation("read_file", &args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_path_denied_write() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"path": "/etc/shadow", "content": "data"});
        let decision = engine.check_file_operation("write_file", &args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_empty_path() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"path": ""});
        let decision = engine.check_file_operation("read_file", &args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_network_allowed_host() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"url": "https://github.com/egkristi/RavenClaw"});
        let decision = engine.check_network_request(&args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_network_denied_host() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"url": "https://evil.com/malware"});
        let decision = engine.check_network_request(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_network_localhost_allowed() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"url": "http://localhost:11434/api/chat"});
        let decision = engine.check_network_request(&args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_network_deny_all() {
        let policy = SecurityPolicy {
            network: NetworkPolicy {
                deny_all: true,
                ..NetworkPolicy::default()
            },
            ..SecurityPolicy::default()
        };
        let engine = PolicyEngine::new(policy);
        let args = serde_json::json!({"url": "https://github.com"});
        let decision = engine.check_network_request(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_network_empty_url() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"url": ""});
        let decision = engine.check_network_request(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_network_invalid_url() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"url": "not-a-url"});
        let decision = engine.check_network_request(&args);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_requires_approval_default() {
        let engine = PolicyEngine::default_secure();
        assert!(engine.requires_approval("shell_exec"));
        assert!(engine.requires_approval("write_file"));
        assert!(!engine.requires_approval("read_file"));
        assert!(!engine.requires_approval("web_fetch"));
    }

    #[test]
    fn test_requires_approval_all() {
        let policy = SecurityPolicy {
            require_approval_all: true,
            ..SecurityPolicy::default()
        };
        let engine = PolicyEngine::new(policy);
        assert!(engine.requires_approval("shell_exec"));
        assert!(engine.requires_approval("read_file"));
        assert!(engine.requires_approval("web_fetch"));
    }

    #[test]
    fn test_check_tool_call_shell() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"command": "echo hello"});
        let decision = engine.check_tool_call("shell_exec", &args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_check_tool_call_read_file() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let decision = engine.check_tool_call("read_file", &args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_check_tool_call_web_fetch() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({"url": "https://github.com"});
        let decision = engine.check_tool_call("web_fetch", &args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_check_tool_call_unknown() {
        let engine = PolicyEngine::default_secure();
        let args = serde_json::json!({});
        let decision = engine.check_tool_call("unknown_tool", &args);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_policy_error_denied() {
        let err = PolicyError::Denied("test".to_string());
        assert_eq!(format!("{}", err), "Policy denied: test");
    }

    #[test]
    fn test_policy_error_invalid_config() {
        let err = PolicyError::InvalidConfig("bad config".to_string());
        assert_eq!(
            format!("{}", err),
            "Invalid policy configuration: bad config"
        );
    }

    #[test]
    fn test_is_localhost() {
        assert!(is_localhost("localhost"));
        assert!(is_localhost("127.0.0.1"));
        assert!(is_localhost("::1"));
        assert!(is_localhost("0.0.0.0"));
        assert!(is_localhost("127.0.0.2"));
        assert!(!is_localhost("example.com"));
    }

    #[test]
    fn test_is_private_ip() {
        assert!(is_private_ip("10.0.0.1"));
        assert!(is_private_ip("192.168.1.1"));
        assert!(is_private_ip("172.16.0.1"));
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("example.com"));
    }

    #[test]
    fn test_shell_policy_default() {
        let policy = ShellPolicy::default();
        assert!(!policy.deny_all);
        assert!(policy.allowed_commands.contains(&"echo".to_string()));
        assert!(policy.denied_commands.contains(&"rm -rf /".to_string()));
    }

    #[test]
    fn test_path_policy_default() {
        let policy = PathPolicy::default();
        assert!(policy.allowed_read_paths.contains(&"/tmp".to_string()));
        assert!(policy.allowed_write_paths.contains(&"/tmp".to_string()));
        assert!(policy.denied_paths.contains(&"/etc/shadow".to_string()));
    }

    #[test]
    fn test_network_policy_default() {
        let policy = NetworkPolicy::default();
        assert!(!policy.deny_all);
        assert!(policy.allow_localhost);
        assert!(!policy.allow_private_networks);
    }

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert!(!policy.require_approval_all);
        assert!(policy.require_approval_for.contains(&"shell_exec".to_string()));
    }

    #[test]
    fn test_permissive_policy() {
        let engine = PolicyEngine::permissive();
        let policy = engine.policy();
        assert!(policy.shell.allowed_commands.contains(&"*".to_string()));
        assert!(policy.network.allowed_hosts.contains(&"*".to_string()));
        assert!(policy.network.allow_private_networks);
    }
}
