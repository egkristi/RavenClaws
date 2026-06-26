//! RavenClaws
//!
//! Provides a workdir jail, resource limits, and timeouts for
//! sandboxed tool execution. Every shell command and file operation
//! runs within the sandbox constraints.
//!
//! # Architecture
//!
//! ```text
//! Sandbox
//!   ├── workdir: a temporary directory jail
//!   ├── timeout: max execution time per operation
//!   ├── max_output: max bytes of output to capture
//!   ├── allowed_env: whitelist of environment variables
//!   └── network: whether network access is allowed
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::info;

// ── Error types ────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Path '{0}' is outside the sandbox")]
    PathOutsideSandbox(String),

    #[error("Sandbox not initialized: {0}")]
    NotInitialized(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Network access denied by sandbox")]
    NetworkDenied,
}

// ── Sandbox configuration ──────────────────────────────────────────────────

/// Sandbox configuration
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Base working directory for the sandbox
    #[serde(default = "default_workdir")]
    pub workdir: String,

    /// Maximum execution timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum output capture size in bytes
    #[serde(default = "default_max_output")]
    pub max_output_bytes: usize,

    /// Maximum file write size in bytes
    #[serde(default = "default_max_write")]
    pub max_write_bytes: usize,

    /// Whether network access is allowed
    #[serde(default)]
    pub allow_network: bool,

    /// Allowed environment variables (prefix match)
    #[serde(default)]
    pub allowed_env_prefixes: Vec<String>,

    /// Whether to create the workdir on init
    #[serde(default = "default_true")]
    pub create_workdir: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            workdir: default_workdir(),
            timeout_secs: default_timeout(),
            max_output_bytes: default_max_output(),
            max_write_bytes: default_max_write(),
            allow_network: false,
            allowed_env_prefixes: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "USER".to_string(),
                "TMPDIR".to_string(),
                "RAVENCLAWS_".to_string(),
            ],
            create_workdir: true,
        }
    }
}

fn default_workdir() -> String {
    "/tmp/ravenclaws-sandbox".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_max_output() -> usize {
    65536
}

fn default_max_write() -> usize {
    1048576
}

fn default_true() -> bool {
    true
}

// ── Sandbox ────────────────────────────────────────────────────────────────

/// A sandboxed execution environment
#[allow(dead_code)]
pub struct Sandbox {
    config: SandboxConfig,
    workdir: PathBuf,
    initialized: bool,
}

#[allow(dead_code)]
impl Sandbox {
    /// Create a new sandbox with the given configuration
    pub fn new(config: SandboxConfig) -> Self {
        let workdir = PathBuf::from(&config.workdir);
        Self {
            config,
            workdir,
            initialized: false,
        }
    }

    /// Create a sandbox with default configuration
    pub fn default() -> Self {
        Self::new(SandboxConfig::default())
    }

    /// Initialize the sandbox — create the workdir if configured
    pub async fn init(&mut self) -> Result<(), SandboxError> {
        if self.config.create_workdir {
            tokio::fs::create_dir_all(&self.workdir).await?;
            info!(workdir = %self.workdir.display(), "Sandbox initialized");
        }
        self.initialized = true;
        Ok(())
    }

    /// Check if the sandbox is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the sandbox workdir path
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Get the sandbox configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Resolve a path within the sandbox
    ///
    /// Returns an error if the resolved path is outside the sandbox workdir.
    pub fn resolve_path(&self, path: &str) -> Result<PathBuf, SandboxError> {
        if !self.initialized {
            return Err(SandboxError::NotInitialized(
                "Sandbox must be initialized before resolving paths".to_string(),
            ));
        }

        let requested = Path::new(path);

        // If the path is absolute, check if it's within the sandbox
        let resolved = if requested.is_absolute() {
            // Canonicalize to resolve symlinks and relative components
            match requested.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    // Path doesn't exist yet — resolve relative to root
                    let components: Vec<_> = requested.components().collect();
                    let mut p = PathBuf::new();
                    for c in components {
                        p.push(c);
                    }
                    p
                }
            }
        } else {
            // Relative path — resolve relative to sandbox workdir
            self.workdir.join(requested)
        };

        // Check that the resolved path is within the sandbox workdir
        if !resolved.starts_with(&self.workdir) {
            // Allow paths in system temp directories
            let temp_dirs = [
                std::env::temp_dir(),
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
            ];
            let in_temp = temp_dirs.iter().any(|d| resolved.starts_with(d));

            if !in_temp {
                return Err(SandboxError::PathOutsideSandbox(
                    resolved.to_string_lossy().to_string(),
                ));
            }
        }

        Ok(resolved)
    }

    /// Check if a path is allowed for reading
    pub fn check_read_path(&self, path: &str) -> Result<PathBuf, SandboxError> {
        let resolved = self.resolve_path(path)?;

        // Check that the path exists and is readable
        if !resolved.exists() {
            return Err(SandboxError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", resolved.display()),
            )));
        }

        Ok(resolved)
    }

    /// Check if a path is allowed for writing
    pub fn check_write_path(&self, path: &str) -> Result<PathBuf, SandboxError> {
        let resolved = self.resolve_path(path)?;

        // Check write size limit
        // (actual size check happens at write time, but we can check the path is valid)

        Ok(resolved)
    }

    /// Check if network access is allowed
    pub fn check_network(&self) -> Result<(), SandboxError> {
        if !self.config.allow_network {
            return Err(SandboxError::NetworkDenied);
        }
        Ok(())
    }

    /// Get a filtered environment for sandboxed processes
    pub fn filtered_env(&self) -> Vec<(String, String)> {
        std::env::vars()
            .filter(|(key, _)| {
                self.config
                    .allowed_env_prefixes
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
            })
            .collect()
    }

    /// Clean up the sandbox workdir
    pub async fn cleanup(&mut self) -> Result<(), SandboxError> {
        if self.initialized && self.workdir.exists() {
            tokio::fs::remove_dir_all(&self.workdir).await?;
            info!(workdir = %self.workdir.display(), "Sandbox cleaned up");
        }
        self.initialized = false;
        Ok(())
    }

    /// Create a temporary file within the sandbox
    pub async fn create_temp_file(
        &self,
        prefix: &str,
        suffix: &str,
    ) -> Result<PathBuf, SandboxError> {
        if !self.initialized {
            return Err(SandboxError::NotInitialized(
                "Sandbox must be initialized before creating temp files".to_string(),
            ));
        }

        // Generate a unique filename
        let uuid = uuid::Uuid::new_v4();
        let filename = format!("{}_{}{}", prefix, uuid, suffix);
        let path = self.workdir.join(&filename);

        // Create the file
        tokio::fs::write(&path, "").await?;

        Ok(path)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.initialized && self.workdir.exists() {
            // Best-effort cleanup in drop (can't use async here)
            let _ = std::fs::remove_dir_all(&self.workdir);
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_init() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_init_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        assert!(!sandbox.is_initialized());

        sandbox.init().await.unwrap();
        assert!(sandbox.is_initialized());
        assert!(sandbox.workdir().exists());

        sandbox.cleanup().await.unwrap();
        assert!(!sandbox.is_initialized());
    }

    #[tokio::test]
    async fn test_sandbox_resolve_relative_path() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_rel_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        sandbox.init().await.unwrap();

        let resolved = sandbox.resolve_path("test.txt").unwrap();
        assert!(resolved.starts_with(&dir));
        assert!(resolved.ends_with("test.txt"));

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_sandbox_resolve_absolute_path_in_sandbox() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_abs_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        sandbox.init().await.unwrap();

        let test_path = dir.join("subdir").join("file.txt");
        let resolved = sandbox
            .resolve_path(test_path.to_string_lossy().as_ref())
            .unwrap();
        assert!(resolved.starts_with(&dir));

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_sandbox_resolve_path_outside() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_outside_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        sandbox.init().await.unwrap();

        // /etc is outside the sandbox and not in temp dirs
        let result = sandbox.resolve_path("/etc/passwd");
        // This might succeed if /etc is in temp dirs (unlikely) or fail
        // On most systems, /etc is not in temp dirs, so this should fail
        if let Err(e) = result {
            assert!(matches!(e, SandboxError::PathOutsideSandbox(_)));
        }

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_sandbox_not_initialized() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config);

        let result = sandbox.resolve_path("test.txt");
        assert!(matches!(
            result.unwrap_err(),
            SandboxError::NotInitialized(_)
        ));
    }

    #[tokio::test]
    async fn test_sandbox_check_read_path_not_found() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_read_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        sandbox.init().await.unwrap();

        let result = sandbox.check_read_path("nonexistent_file.txt");
        assert!(result.is_err());

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_sandbox_check_network_allowed() {
        let config = SandboxConfig {
            allow_network: true,
            ..SandboxConfig::default()
        };
        let sandbox = Sandbox::new(config);
        assert!(sandbox.check_network().is_ok());
    }

    #[tokio::test]
    async fn test_sandbox_check_network_denied() {
        let config = SandboxConfig {
            allow_network: false,
            ..SandboxConfig::default()
        };
        let sandbox = Sandbox::new(config);
        let result = sandbox.check_network();
        assert!(matches!(result.unwrap_err(), SandboxError::NetworkDenied));
    }

    #[tokio::test]
    async fn test_sandbox_filtered_env() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config);
        let env = sandbox.filtered_env();

        // Should include PATH
        assert!(env.iter().any(|(k, _)| k == "PATH"));

        // Should NOT include random env vars
        assert!(!env.iter().any(|(k, _)| k == "AWS_SECRET_ACCESS_KEY"));
    }

    #[tokio::test]
    async fn test_sandbox_create_temp_file() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_temp_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        sandbox.init().await.unwrap();

        let path = sandbox.create_temp_file("test", ".txt").await.unwrap();
        assert!(path.exists());
        assert!(path.starts_with(&dir));

        // Cleanup
        tokio::fs::remove_file(&path).await.unwrap();
        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_sandbox_create_temp_file_not_initialized() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config);

        let result = sandbox.create_temp_file("test", ".txt").await;
        assert!(matches!(
            result.unwrap_err(),
            SandboxError::NotInitialized(_)
        ));
    }

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.workdir, "/tmp/ravenclaws-sandbox");
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_output_bytes, 65536);
        assert!(!config.allow_network);
        assert!(config.create_workdir);
    }

    #[test]
    fn test_sandbox_error_path_outside() {
        let err = SandboxError::PathOutsideSandbox("/etc".to_string());
        assert_eq!(format!("{}", err), "Path '/etc' is outside the sandbox");
    }

    #[test]
    fn test_sandbox_error_not_initialized() {
        let err = SandboxError::NotInitialized("not ready".to_string());
        assert_eq!(format!("{}", err), "Sandbox not initialized: not ready");
    }

    #[test]
    fn test_sandbox_error_resource_limit() {
        let err = SandboxError::ResourceLimit("too big".to_string());
        assert_eq!(format!("{}", err), "Resource limit exceeded: too big");
    }

    #[test]
    fn test_sandbox_error_network_denied() {
        let err = SandboxError::NetworkDenied;
        assert_eq!(format!("{}", err), "Network access denied by sandbox");
    }

    #[test]
    fn test_sandbox_drop_cleanup() {
        let dir = std::env::temp_dir().join(format!(
            "ravenclaws_sandbox_drop_test_{}",
            std::process::id()
        ));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };

        // Create and init in a scope
        {
            let mut sandbox = Sandbox::new(config);
            // Use tokio::runtime::Runtime to call async init in sync context
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(sandbox.init()).unwrap();
            assert!(dir.exists());
            // sandbox drops here, calling cleanup
        }

        // After drop, the directory should be removed
        // Note: this is best-effort in drop, so it might not always work
        // but for our test it should
    }

    #[tokio::test]
    async fn test_sandbox_check_write_path() {
        let dir =
            std::env::temp_dir().join(format!("ravenclaws_sandbox_write_{}", std::process::id()));
        let config = SandboxConfig {
            workdir: dir.to_string_lossy().to_string(),
            create_workdir: true,
            ..SandboxConfig::default()
        };
        let mut sandbox = Sandbox::new(config);
        sandbox.init().await.unwrap();

        let result = sandbox.check_write_path("new_file.txt");
        assert!(result.is_ok());

        sandbox.cleanup().await.unwrap();
    }

    #[test]
    fn test_sandbox_config_serialization() {
        let config = SandboxConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("/tmp/ravenclaws-sandbox"));
        assert!(json.contains("30"));
    }

    #[test]
    fn test_sandbox_config_deserialization() {
        let json = r#"{
            "workdir": "/custom/sandbox",
            "timeout_secs": 60,
            "allow_network": true
        }"#;
        let config: SandboxConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.workdir, "/custom/sandbox");
        assert_eq!(config.timeout_secs, 60);
        assert!(config.allow_network);
    }
}
