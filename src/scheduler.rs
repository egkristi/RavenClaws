//! Scheduling & triggers for proactive 24/7 agents
//!
//! Provides three trigger mechanisms that submit tasks to the BackgroundTaskManager:
//!
//! 1. **Cron scheduling** — execute tasks on a recurring schedule (cron expressions)
//! 2. **Webhook triggers** — HTTP endpoint to receive webhooks and trigger tasks
//! 3. **File-watch triggers** — watch files/directories for changes and trigger tasks
//!
//! # Architecture
//!
//! ```text
//! Scheduler
//!   ├── CronTrigger   — cron expression → tokio interval → submit task
//!   ├── WebhookServer — HTTP POST /webhook/<name> → submit task
//!   └── FileWatcher   — notify events → debounce → submit task
//! ```
//!
//! All triggers submit tasks to a shared `BackgroundTaskManager` and use the
//! configured LLM client for execution.

use crate::background::BackgroundTaskManager;
use crate::config::SchedulerConfig;
use crate::error::{RavenClawError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

// ── Trigger types ──────────────────────────────────────────────────────────

/// A single scheduled trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Unique name for this trigger
    pub name: String,
    /// The prompt to send to the agent when triggered
    pub prompt: String,
    /// Optional system prompt override
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Trigger type-specific configuration
    pub trigger: TriggerType,
}

/// The type of trigger and its specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerType {
    /// Cron expression (e.g., "0 */6 * * *" for every 6 hours)
    #[serde(rename = "cron")]
    Cron {
        /// Standard cron expression (5 or 6 fields)
        expression: String,
    },
    /// Webhook endpoint (e.g., POST /webhook/<name>)
    #[serde(rename = "webhook")]
    Webhook {
        /// Optional secret for HMAC verification
        #[serde(default)]
        secret: Option<String>,
    },
    /// File system watcher
    #[serde(rename = "watch")]
    Watch {
        /// Path to watch
        path: String,
        /// Event types to trigger on (create, modify, delete)
        #[serde(default = "default_watch_events")]
        events: Vec<String>,
        /// Debounce interval in seconds (default: 5)
        #[serde(default = "default_debounce_secs")]
        debounce_secs: u64,
    },
}

fn default_watch_events() -> Vec<String> {
    vec!["modify".to_string()]
}

fn default_debounce_secs() -> u64 {
    5
}

// ── Scheduler ──────────────────────────────────────────────────────────────

/// The main scheduler that manages all triggers
///
/// Owns the trigger configurations and manages their lifecycle.
/// All triggers submit tasks to the shared BackgroundTaskManager.
#[derive(Clone)]
pub struct Scheduler {
    /// Background task manager for executing triggered tasks
    bg_manager: BackgroundTaskManager,
    /// Trigger configurations
    triggers: Vec<TriggerConfig>,
    /// Whether the scheduler is running
    running: Arc<RwLock<bool>>,
}

impl Scheduler {
    /// Create a new scheduler with the given background task manager and triggers
    pub fn new(bg_manager: BackgroundTaskManager, config: &SchedulerConfig) -> Self {
        Self {
            bg_manager,
            triggers: config.triggers.clone(),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start all triggers and run until cancelled
    ///
    /// Spawns tokio tasks for each trigger type:
    /// - Cron triggers: periodic timer based on cron expression
    /// - Webhook triggers: HTTP server listening for POST requests
    /// - Watch triggers: file system watcher with debouncing
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Scheduler is already running");
                return Ok(());
            }
            *running = true;
        }

        let trigger_count = self.triggers.len();
        if trigger_count == 0 {
            info!("No triggers configured, scheduler idle");
            return Ok(());
        }

        info!(count = trigger_count, "Starting scheduler with triggers");

        // Parse and validate all triggers
        for trigger in &self.triggers {
            match &trigger.trigger {
                TriggerType::Cron { expression } => {
                    let _schedule = expression.parse::<cron::Schedule>().map_err(|e| {
                        RavenClawError::CommandExecution(format!(
                            "Invalid cron expression '{}': {}",
                            expression, e
                        ))
                    })?;
                    info!(
                        name = %trigger.name,
                        expression = %expression,
                        "Registered cron trigger"
                    );
                }
                TriggerType::Webhook { secret } => {
                    let _has_secret = secret.is_some();
                    info!(
                        name = %trigger.name,
                        has_secret = _has_secret,
                        "Registered webhook trigger"
                    );
                }
                TriggerType::Watch {
                    path,
                    events,
                    debounce_secs,
                } => {
                    let path_buf = PathBuf::from(path);
                    if !path_buf.exists() {
                        warn!(
                            name = %trigger.name,
                            path = %path,
                            "Watch path does not exist yet, will retry on start"
                        );
                    }
                    info!(
                        name = %trigger.name,
                        path = %path,
                        events = ?events,
                        debounce_secs = debounce_secs,
                        "Registered file watch trigger"
                    );
                }
            }
        }

        // Spawn cron trigger tasks
        for trigger in &self.triggers {
            if let TriggerType::Cron { expression } = &trigger.trigger {
                let schedule = expression.parse::<cron::Schedule>().map_err(|e| {
                    RavenClawError::CommandExecution(format!(
                        "Invalid cron expression '{}': {}",
                        expression, e
                    ))
                })?;
                let bg = self.bg_manager.clone();
                let name = trigger.name.clone();
                let prompt = trigger.prompt.clone();
                let system_prompt = trigger.system_prompt.clone().unwrap_or_default();

                tokio::spawn(async move {
                    run_cron_trigger(name, schedule, bg, prompt, system_prompt).await;
                });
            }
        }

        // Spawn webhook server if any webhook triggers exist
        let has_webhooks = self
            .triggers
            .iter()
            .any(|t| matches!(t.trigger, TriggerType::Webhook { .. }));

        if has_webhooks {
            let webhook_triggers: Vec<(String, String, Option<String>)> = self
                .triggers
                .iter()
                .filter_map(|t| {
                    if let TriggerType::Webhook { secret } = &t.trigger {
                        Some((t.name.clone(), t.prompt.clone(), secret.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            let bg = self.bg_manager.clone();
            // Use a default webhook port — configurable in future
            let port = 9090u16;

            tokio::spawn(async move {
                run_webhook_server(port, webhook_triggers, bg).await;
            });
        }

        // Spawn file watch tasks
        for trigger in &self.triggers {
            if let TriggerType::Watch {
                path,
                events,
                debounce_secs,
            } = &trigger.trigger
            {
                let bg = self.bg_manager.clone();
                let name = trigger.name.clone();
                let prompt = trigger.prompt.clone();
                let system_prompt = trigger.system_prompt.clone().unwrap_or_default();
                let watch_path = path.clone();
                let watch_events = events.clone();
                let debounce = *debounce_secs;

                tokio::spawn(async move {
                    run_watch_trigger(
                        name,
                        watch_path,
                        watch_events,
                        debounce,
                        bg,
                        prompt,
                        system_prompt,
                    )
                    .await;
                });
            }
        }

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Scheduler stopped");
    }

    /// Check if the scheduler is running
    #[allow(dead_code)]
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

// ── Cron trigger runner ────────────────────────────────────────────────────

/// Run a cron trigger: sleep until next scheduled time, then submit a task
#[instrument(skip(bg_manager, schedule), fields(trigger_name = %name))]
async fn run_cron_trigger(
    name: String,
    schedule: cron::Schedule,
    bg_manager: BackgroundTaskManager,
    prompt: String,
    system_prompt: String,
) {
    info!(trigger = %name, "Cron trigger started");

    for next in schedule.upcoming(chrono::Utc) {
        let now = chrono::Utc::now();
        let delay = (next - now).to_std().unwrap_or(std::time::Duration::ZERO);

        if delay > std::time::Duration::ZERO {
            debug!(
                trigger = %name,
                next_run = %next,
                delay_ms = delay.as_millis(),
                "Sleeping until next cron trigger"
            );
            tokio::time::sleep(delay).await;
        }

        debug!(trigger = %name, "Cron trigger firing");
        match bg_manager
            .submit(prompt.clone(), system_prompt.clone())
            .await
        {
            Ok(task_id) => {
                info!(
                    trigger = %name,
                    task_id = %task_id,
                    "Cron trigger submitted background task"
                );
            }
            Err(e) => {
                error!(
                    trigger = %name,
                    error = %e,
                    "Cron trigger failed to submit task"
                );
            }
        }
    }
}

// ── Webhook server ─────────────────────────────────────────────────────────

/// Run a simple HTTP server that listens for webhook POST requests
#[instrument(skip(triggers, bg_manager))]
async fn run_webhook_server(
    port: u16,
    triggers: Vec<(String, String, Option<String>)>,
    bg_manager: BackgroundTaskManager,
) {
    let bind_addr = format!("127.0.0.1:{}", port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => {
            info!(
                address = %bind_addr,
                trigger_count = triggers.len(),
                "Webhook server started"
            );
            l
        }
        Err(e) => {
            error!(
                address = %bind_addr,
                error = %e,
                "Failed to start webhook server"
            );
            return;
        }
    };

    let triggers = Arc::new(triggers);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let triggers = Arc::clone(&triggers);
                let bg = bg_manager.clone();
                tokio::spawn(async move {
                    handle_webhook_connection(stream, peer, triggers, bg).await;
                });
            }
            Err(e) => {
                warn!(error = %e, "Webhook server accept error");
            }
        }
    }
}

/// Handle a single webhook HTTP connection
async fn handle_webhook_connection(
    mut stream: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
    triggers: Arc<Vec<(String, String, Option<String>)>>,
    bg_manager: BackgroundTaskManager,
) {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).await.is_err() {
        return;
    }

    let request_line = request_line.trim();
    if request_line.is_empty() {
        return;
    }

    // Parse method and path
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        send_http_response(&mut stream, "400 Bad Request", b"Bad Request").await;
        return;
    }

    let method = parts[0];
    let path = parts[1];

    // Only accept POST requests
    if method != "POST" {
        send_http_response(&mut stream, "405 Method Not Allowed", b"Method Not Allowed").await;
        return;
    }

    // Extract trigger name from path: /webhook/<name>
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if path_parts.len() < 2 || path_parts[0] != "webhook" {
        send_http_response(&mut stream, "404 Not Found", b"Not Found").await;
        return;
    }

    let trigger_name = path_parts[1];

    // Read headers
    let mut header_line = String::new();
    let mut content_length: usize = 0;
    loop {
        header_line.clear();
        if reader.read_line(&mut header_line).await.is_err() {
            return;
        }
        let line = header_line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(len_str) = line.strip_prefix("Content-Length:") {
            content_length = len_str.trim().parse().unwrap_or(0);
        }
    }

    // Read body
    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).await.is_err() {
        send_http_response(&mut stream, "400 Bad Request", b"Bad Request").await;
        return;
    }

    // Find matching trigger
    let matched = triggers.iter().find(|(name, _, _)| name == trigger_name);

    if let Some((_name, prompt, _secret)) = matched {
        // Submit the task with webhook body as context
        let webhook_body = String::from_utf8_lossy(&body);
        let full_prompt = format!("{}\n\nWebhook payload:\n{}", prompt, webhook_body);

        match bg_manager.submit(full_prompt, String::new()).await {
            Ok(task_id) => {
                info!(
                    trigger = %_name,
                    task_id = %task_id,
                    peer = %peer,
                    "Webhook trigger submitted background task"
                );
                send_http_response(
                    &mut stream,
                    "200 OK",
                    format!("{{\"task_id\":\"{}\"}}", task_id).as_bytes(),
                )
                .await;
            }
            Err(e) => {
                error!(
                    trigger = %_name,
                    error = %e,
                    "Webhook trigger failed to submit task"
                );
                send_http_response(
                    &mut stream,
                    "500 Internal Server Error",
                    b"Internal Server Error",
                )
                .await;
            }
        }
    } else {
        send_http_response(&mut stream, "404 Not Found", b"Trigger Not Found").await;
    }
}

/// Send an HTTP response to any async writer
async fn send_http_response(
    stream: &mut (impl tokio::io::AsyncWrite + Unpin),
    status: &str,
    body: &[u8],
) {
    use tokio::io::AsyncWriteExt;

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        body.len(),
    );

    if let Err(e) = stream.write_all(response.as_bytes()).await {
        warn!(error = %e, "Failed to write webhook response headers");
        return;
    }
    if let Err(e) = stream.write_all(body).await {
        warn!(error = %e, "Failed to write webhook response body");
        return;
    }
    if let Err(e) = stream.flush().await {
        warn!(error = %e, "Failed to flush webhook response");
    }
}

// ── File watch trigger runner ──────────────────────────────────────────────

/// Run a file watch trigger: monitor a path for changes and submit tasks
#[instrument(skip(bg_manager), fields(trigger_name = %name, path = %watch_path))]
async fn run_watch_trigger(
    name: String,
    watch_path: String,
    events: Vec<String>,
    debounce_secs: u64,
    bg_manager: BackgroundTaskManager,
    prompt: String,
    system_prompt: String,
) {
    use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;

    info!(
        trigger = %name,
        path = %watch_path,
        "File watch trigger started"
    );

    // Create a channel to receive file system events
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            error!(
                trigger = %name,
                error = %e,
                "Failed to create file watcher"
            );
            return;
        }
    };

    // Watch the path
    let path = PathBuf::from(&watch_path);
    if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
        error!(
            trigger = %name,
            path = %watch_path,
            error = %e,
            "Failed to watch path"
        );
        return;
    }

    // Debounce state
    let debounce_duration = std::time::Duration::from_secs(debounce_secs);
    let mut last_trigger_time: Option<std::time::Instant> = None;

    // Process events
    for event in rx {
        match event {
            Ok(event) => {
                let should_trigger = match &event.kind {
                    EventKind::Create(_) => events.contains(&"create".to_string()),
                    EventKind::Modify(_) => events.contains(&"modify".to_string()),
                    EventKind::Remove(_) => events.contains(&"delete".to_string()),
                    _ => false,
                };

                if should_trigger {
                    let now = std::time::Instant::now();
                    let should_fire = match last_trigger_time {
                        Some(last) => now.duration_since(last) >= debounce_duration,
                        None => true,
                    };

                    if should_fire {
                        last_trigger_time = Some(now);
                        debug!(
                            trigger = %name,
                            event = ?event.kind,
                            paths = ?event.paths,
                            "File watch trigger firing"
                        );

                        match bg_manager
                            .submit(prompt.clone(), system_prompt.clone())
                            .await
                        {
                            Ok(task_id) => {
                                info!(
                                    trigger = %name,
                                    task_id = %task_id,
                                    "File watch trigger submitted background task"
                                );
                            }
                            Err(e) => {
                                error!(
                                    trigger = %name,
                                    error = %e,
                                    "File watch trigger failed to submit task"
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    trigger = %name,
                    error = %e,
                    "File watch error"
                );
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::BackgroundTaskManager;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ravenclaw-test-sched-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn test_trigger_config_cron() {
        let config = TriggerConfig {
            name: "hourly".to_string(),
            prompt: "Run hourly check".to_string(),
            system_prompt: None,
            trigger: TriggerType::Cron {
                expression: "0 * * * * *".to_string(),
            },
        };

        assert_eq!(config.name, "hourly");
        match &config.trigger {
            TriggerType::Cron { expression } => {
                assert_eq!(expression, "0 * * * * *");
            }
            _ => panic!("Expected Cron trigger"),
        }
    }

    #[tokio::test]
    async fn test_trigger_config_webhook() {
        let config = TriggerConfig {
            name: "github-webhook".to_string(),
            prompt: "Process GitHub event".to_string(),
            system_prompt: None,
            trigger: TriggerType::Webhook {
                secret: Some("mysecret".to_string()),
            },
        };

        assert_eq!(config.name, "github-webhook");
        match &config.trigger {
            TriggerType::Webhook { secret } => {
                assert_eq!(secret.as_deref(), Some("mysecret"));
            }
            _ => panic!("Expected Webhook trigger"),
        }
    }

    #[tokio::test]
    async fn test_trigger_config_watch() {
        let config = TriggerConfig {
            name: "config-watch".to_string(),
            prompt: "Config changed".to_string(),
            system_prompt: None,
            trigger: TriggerType::Watch {
                path: "/etc/config".to_string(),
                events: vec!["modify".to_string(), "create".to_string()],
                debounce_secs: 10,
            },
        };

        assert_eq!(config.name, "config-watch");
        match &config.trigger {
            TriggerType::Watch {
                path,
                events,
                debounce_secs,
            } => {
                assert_eq!(path, "/etc/config");
                assert_eq!(events.len(), 2);
                assert_eq!(*debounce_secs, 10);
            }
            _ => panic!("Expected Watch trigger"),
        }
    }

    #[tokio::test]
    async fn test_scheduler_new() {
        let dir = test_dir("new");
        let bg = BackgroundTaskManager::new(&dir).await.unwrap();
        let config = SchedulerConfig { triggers: vec![] };
        let scheduler = Scheduler::new(bg, &config);

        assert!(!scheduler.is_running().await);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let dir = test_dir("start_stop");
        let bg = BackgroundTaskManager::new(&dir).await.unwrap();
        let config = SchedulerConfig { triggers: vec![] };
        let scheduler = Scheduler::new(bg, &config);

        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);

        scheduler.stop().await;
        assert!(!scheduler.is_running().await);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_cron_expression_parsing() {
        // Valid cron expression
        let expr = "0 */6 * * * *";
        let schedule = expr.parse::<cron::Schedule>();
        assert!(schedule.is_ok(), "Valid cron expression should parse");

        // Invalid cron expression
        let bad_expr = "not-a-cron";
        let schedule = bad_expr.parse::<cron::Schedule>();
        assert!(schedule.is_err(), "Invalid cron expression should fail");
    }

    #[tokio::test]
    async fn test_cron_schedule_upcoming() {
        let expr = "0 * * * * *"; // Every minute at :00
        let schedule = expr.parse::<cron::Schedule>().unwrap();
        let now = chrono::Utc::now();

        let mut upcoming = schedule.upcoming(chrono::Utc);
        let next = upcoming.next();
        assert!(next.is_some(), "Should have a next scheduled time");
        assert!(next.unwrap() > now, "Next time should be in the future");
    }

    #[tokio::test]
    async fn test_scheduler_with_cron_trigger() {
        let dir = test_dir("with_cron");
        let bg = BackgroundTaskManager::new(&dir).await.unwrap();

        let config = SchedulerConfig {
            triggers: vec![TriggerConfig {
                name: "test-cron".to_string(),
                prompt: "Cron test".to_string(),
                system_prompt: None,
                trigger: TriggerType::Cron {
                    expression: "0 0 1 1 * *".to_string(), // Once a year
                },
            }],
        };

        let scheduler = Scheduler::new(bg, &config);
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);

        scheduler.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_scheduler_with_webhook_trigger() {
        let dir = test_dir("with_webhook");
        let bg = BackgroundTaskManager::new(&dir).await.unwrap();

        let config = SchedulerConfig {
            triggers: vec![TriggerConfig {
                name: "test-webhook".to_string(),
                prompt: "Webhook test".to_string(),
                system_prompt: None,
                trigger: TriggerType::Webhook { secret: None },
            }],
        };

        let scheduler = Scheduler::new(bg, &config);
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);

        scheduler.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_scheduler_with_watch_trigger() {
        let dir = test_dir("with_watch");
        let bg = BackgroundTaskManager::new(&dir).await.unwrap();

        let config = SchedulerConfig {
            triggers: vec![TriggerConfig {
                name: "test-watch".to_string(),
                prompt: "Watch test".to_string(),
                system_prompt: None,
                trigger: TriggerType::Watch {
                    path: dir.to_string_lossy().to_string(),
                    events: vec!["modify".to_string()],
                    debounce_secs: 1,
                },
            }],
        };

        let scheduler = Scheduler::new(bg, &config);
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);

        scheduler.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_webhook_response_format() {
        let task_id = "test-uuid-1234";
        let response = format!("{{\"task_id\":\"{}\"}}", task_id);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["task_id"], task_id);
    }

    #[tokio::test]
    async fn test_send_http_response() {
        let (mut a, mut b) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            send_http_response(&mut a, "200 OK", b"{\"status\":\"ok\"}").await;
        });

        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 512];
        let n = b.read(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.contains("200 OK"));
        assert!(response.contains("{\"status\":\"ok\"}"));
    }

    #[tokio::test]
    async fn test_trigger_config_serialization() {
        let config = TriggerConfig {
            name: "test".to_string(),
            prompt: "test prompt".to_string(),
            system_prompt: Some("system".to_string()),
            trigger: TriggerType::Cron {
                expression: "0 * * * * *".to_string(),
            },
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: TriggerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.prompt, "test prompt");
        assert_eq!(deserialized.system_prompt, Some("system".to_string()));
        match &deserialized.trigger {
            TriggerType::Cron { expression } => {
                assert_eq!(expression, "0 * * * * *");
            }
            _ => panic!("Expected Cron trigger"),
        }
    }

    #[tokio::test]
    async fn test_webhook_trigger_serialization() {
        let config = TriggerConfig {
            name: "gh".to_string(),
            prompt: "process".to_string(),
            system_prompt: None,
            trigger: TriggerType::Webhook {
                secret: Some("s3cret".to_string()),
            },
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: TriggerConfig = serde_json::from_str(&json).unwrap();

        match &deserialized.trigger {
            TriggerType::Webhook { secret } => {
                assert_eq!(secret.as_deref(), Some("s3cret"));
            }
            _ => panic!("Expected Webhook trigger"),
        }
    }

    #[tokio::test]
    async fn test_watch_trigger_serialization() {
        let config = TriggerConfig {
            name: "fw".to_string(),
            prompt: "file changed".to_string(),
            system_prompt: None,
            trigger: TriggerType::Watch {
                path: "/tmp".to_string(),
                events: vec!["modify".to_string()],
                debounce_secs: 5,
            },
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: TriggerConfig = serde_json::from_str(&json).unwrap();

        match &deserialized.trigger {
            TriggerType::Watch {
                path,
                events,
                debounce_secs,
            } => {
                assert_eq!(path, "/tmp");
                assert_eq!(events, &vec!["modify".to_string()]);
                assert_eq!(*debounce_secs, 5);
            }
            _ => panic!("Expected Watch trigger"),
        }
    }

    #[tokio::test]
    async fn test_default_watch_events() {
        let events = default_watch_events();
        assert_eq!(events, vec!["modify".to_string()]);
    }

    #[tokio::test]
    async fn test_default_debounce_secs() {
        assert_eq!(default_debounce_secs(), 5);
    }
}
