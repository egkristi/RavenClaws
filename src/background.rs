//! Background task manager for long-horizon async runs
//!
//! Supports assign-and-walk-away execution with persistence across restarts.
//! Tasks are serialized to JSON in a configurable directory and can be resumed
//! on process restart.
//!
//! # Architecture
//!
//! - `BackgroundTaskManager` — owns the task store and manages lifecycle
//! - `BackgroundTask` — a single task with status, prompt, and result
//! - `TaskStatus` — pending → running → completed / failed
//! - Tasks are persisted as individual JSON files in `tasks/` directory
//!
//! # Usage
//!
//! ```ignore
//! let mut manager = BackgroundTaskManager::new("/tmp/ravenclaws-tasks")?;
//! let task_id = manager.submit("Analyze this data", llm_client).await?;
//! let status = manager.status(&task_id)?;
//! ```

use crate::config::RuntimeConfig;
use crate::error::{RavenClawsError, Result};
use crate::llm::LLMProviderTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

/// Status of a background task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Task has been submitted but not yet started
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled by the user
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A single background task with full lifecycle tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTask {
    /// Unique task identifier (UUID v4)
    pub id: String,
    /// User-provided prompt for the agent
    pub prompt: String,
    /// System prompt used for this task
    pub system_prompt: String,
    /// Current status
    pub status: TaskStatus,
    /// Final result (set when completed)
    pub result: Option<String>,
    /// Error message (set when failed)
    pub error: Option<String>,
    /// When the task was created (ISO 8601)
    pub created_at: String,
    /// When the task was last updated (ISO 8601)
    pub updated_at: String,
    /// Number of agent loop iterations used
    pub iterations: usize,
    /// Provider name that executed this task
    pub provider: Option<String>,
    /// Model name that executed this task
    pub model: Option<String>,
}

impl BackgroundTask {
    /// Create a new pending task
    pub fn new(id: String, prompt: String, system_prompt: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            prompt,
            system_prompt,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
            iterations: 0,
            provider: None,
            model: None,
        }
    }
}

/// Manager for background tasks with disk persistence
#[derive(Debug, Clone)]
pub struct BackgroundTaskManager {
    /// Directory where task files are stored
    tasks_dir: PathBuf,
    /// In-memory task index (id → task)
    tasks: Arc<RwLock<HashMap<String, BackgroundTask>>>,
}

impl BackgroundTaskManager {
    /// Create a new background task manager.
    ///
    /// The `tasks_dir` is where task JSON files are persisted.
    /// If the directory doesn't exist, it will be created.
    /// On creation, it loads any existing tasks from disk.
    pub async fn new(tasks_dir: &Path) -> Result<Self> {
        let tasks_dir = tasks_dir.to_path_buf();

        // Create the tasks directory if it doesn't exist
        std::fs::create_dir_all(&tasks_dir).map_err(|e| {
            RavenClawsError::CommandExecution(format!(
                "Failed to create tasks directory '{}': {}",
                tasks_dir.display(),
                e
            ))
        })?;

        let tasks = Arc::new(RwLock::new(HashMap::new()));

        let mut manager = Self { tasks_dir, tasks };

        // Load existing tasks from disk
        let count = manager.load_tasks().await?;
        if count > 0 {
            info!(count, "Loaded existing background tasks from disk");
        }

        Ok(manager)
    }

    /// Create from runtime config — uses `workdir/tasks/` as the tasks directory
    pub async fn from_config(config: &RuntimeConfig) -> Result<Self> {
        let tasks_dir = PathBuf::from(&config.workdir).join("tasks");
        Self::new(&tasks_dir).await
    }

    /// Load all tasks from disk into memory
    async fn load_tasks(&mut self) -> Result<usize> {
        let mut count = 0;
        let read_dir = match std::fs::read_dir(&self.tasks_dir) {
            Ok(d) => d,
            Err(_) => return Ok(0),
        };

        let mut tasks_to_insert = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<BackgroundTask>(&content) {
                        Ok(task) => {
                            tasks_to_insert.push(task);
                            count += 1;
                        }
                        Err(e) => {
                            warn!(
                                path = %path.display(),
                                error = %e,
                                "Failed to deserialize background task"
                            );
                        }
                    },
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to read background task file"
                        );
                    }
                }
            }
        }

        let mut tasks = self.tasks.write().await;
        for task in tasks_to_insert {
            tasks.insert(task.id.clone(), task);
        }

        Ok(count)
    }

    /// Persist a single task to disk
    fn save_task(&self, task: &BackgroundTask) -> Result<()> {
        let path = self.tasks_dir.join(format!("{}.json", task.id));
        let content = serde_json::to_string_pretty(task).map_err(|e| {
            RavenClawsError::CommandExecution(format!("Failed to serialize task: {}", e))
        })?;

        std::fs::write(&path, content).map_err(|e| {
            RavenClawsError::CommandExecution(format!(
                "Failed to write task file '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Submit a new background task and return its ID.
    /// The task is persisted immediately and will be executed in the background.
    pub async fn submit(&self, prompt: String, system_prompt: String) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let task = BackgroundTask::new(id.clone(), prompt, system_prompt);

        // Persist to disk
        self.save_task(&task)?;

        // Add to in-memory index
        let mut tasks = self.tasks.write().await;
        tasks.insert(id.clone(), task);

        info!(task_id = %id, "Background task submitted");
        Ok(id)
    }

    /// Execute a background task with the given LLM client.
    /// Updates the task status to Running, runs the agent loop, and saves the result.
    #[instrument(skip(self, llm), fields(task_id = %task_id))]
    pub async fn execute(&self, task_id: &str, llm: Arc<dyn LLMProviderTrait>) -> Result<String> {
        // Get the task and mark as running
        {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(task_id).ok_or_else(|| {
                RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
            })?;

            task.status = TaskStatus::Running;
            task.provider = Some(llm.provider_name().to_string());
            task.model = Some(llm.model().to_string());
            task.updated_at = chrono::Utc::now().to_rfc3339();
            self.save_task(task)?;
        }

        info!(
            task_id = %task_id,
            provider = llm.provider_name(),
            model = llm.model(),
            "Executing background task"
        );

        // Run the agent loop
        let loop_config = crate::agent::AgentLoopConfig {
            max_iterations: 10,
            enable_tools: true,
            require_approval: false,
            prompt_injection_protection: true,
            token_lifetime_secs: 0,
            no_final_required: false,
        };

        let result = crate::agent::run_agent_loop(
            llm.clone(),
            &self.get_prompt(task_id).await?,
            &self.get_system_prompt(task_id).await?,
            loop_config,
        )
        .await;

        // Update task with result
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id).ok_or_else(|| {
            RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
        })?;

        match result {
            Ok(response) => {
                task.status = TaskStatus::Completed;
                task.result = Some(response.clone());
                task.updated_at = chrono::Utc::now().to_rfc3339();
                self.save_task(task)?;

                info!(
                    task_id = %task_id,
                    iterations = task.iterations,
                    "Background task completed"
                );

                Ok(response)
            }
            Err(e) => {
                task.status = TaskStatus::Failed;
                task.error = Some(e.to_string());
                task.updated_at = chrono::Utc::now().to_rfc3339();
                self.save_task(task)?;

                warn!(
                    task_id = %task_id,
                    error = %e,
                    "Background task failed"
                );

                Err(e)
            }
        }
    }

    /// Get the current status of a task
    #[allow(dead_code)]
    pub async fn status(&self, task_id: &str) -> Result<TaskStatus> {
        let tasks = self.tasks.read().await;
        let task = tasks.get(task_id).ok_or_else(|| {
            RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
        })?;
        Ok(task.status.clone())
    }

    /// Get the full task details
    pub async fn get_task(&self, task_id: &str) -> Result<BackgroundTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned().ok_or_else(|| {
            RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
        })
    }

    /// List all tasks with their status
    pub async fn list_tasks(&self) -> Vec<BackgroundTask> {
        let tasks = self.tasks.read().await;
        let mut task_list: Vec<BackgroundTask> = tasks.values().cloned().collect();
        // Sort by creation time (newest first)
        task_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        task_list
    }

    /// Cancel a pending or running task
    pub async fn cancel(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id).ok_or_else(|| {
            RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
        })?;

        match task.status {
            TaskStatus::Pending | TaskStatus::Running => {
                task.status = TaskStatus::Cancelled;
                task.updated_at = chrono::Utc::now().to_rfc3339();
                self.save_task(task)?;
                info!(task_id = %task_id, "Background task cancelled");
                Ok(())
            }
            _ => Err(RavenClawsError::CommandExecution(format!(
                "Cannot cancel task '{}' in status '{}'",
                task_id, task.status
            ))),
        }
    }

    /// Resume all incomplete tasks (Pending or Running) from disk.
    /// Returns the list of task IDs that need execution.
    pub async fn resume_incomplete(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        let mut incomplete = Vec::new();

        for task in tasks.values() {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::Running {
                incomplete.push(task.id.clone());
            }
        }

        if !incomplete.is_empty() {
            info!(
                count = incomplete.len(),
                "Found incomplete background tasks to resume"
            );
        }

        incomplete
    }

    /// Get the prompt for a task (internal helper)
    async fn get_prompt(&self, task_id: &str) -> Result<String> {
        let tasks = self.tasks.read().await;
        let task = tasks.get(task_id).ok_or_else(|| {
            RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
        })?;
        Ok(task.prompt.clone())
    }

    /// Get the system prompt for a task (internal helper)
    async fn get_system_prompt(&self, task_id: &str) -> Result<String> {
        let tasks = self.tasks.read().await;
        let task = tasks.get(task_id).ok_or_else(|| {
            RavenClawsError::CommandExecution(format!("Task '{}' not found", task_id))
        })?;
        Ok(task.system_prompt.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ravenclaws-test-bg-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn test_manager_new_creates_directory() {
        let dir = test_dir("create_dir");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();
        assert!(dir.exists(), "Tasks directory should be created");
        assert!(manager.tasks.read().await.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_submit_task() {
        let dir = test_dir("submit");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();

        let task_id = manager
            .submit("Test prompt".to_string(), "Test system".to_string())
            .await
            .unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.prompt, "Test prompt");
        assert_eq!(task.system_prompt, "Test system");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.result.is_none());

        // Verify persistence
        let task_path = dir.join(format!("{}.json", task_id));
        assert!(task_path.exists(), "Task file should exist on disk");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_status_transitions() {
        let dir = test_dir("transitions");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();

        let task_id = manager
            .submit("Test".to_string(), "System".to_string())
            .await
            .unwrap();

        assert_eq!(manager.status(&task_id).await.unwrap(), TaskStatus::Pending);

        // Cancel the task
        manager.cancel(&task_id).await.unwrap();
        assert_eq!(
            manager.status(&task_id).await.unwrap(),
            TaskStatus::Cancelled
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let dir = test_dir("list");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();

        manager
            .submit("Task 1".to_string(), "System".to_string())
            .await
            .unwrap();
        manager
            .submit("Task 2".to_string(), "System".to_string())
            .await
            .unwrap();

        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_cancel_completed_task_fails() {
        let dir = test_dir("cancel_fail");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();

        let task_id = manager
            .submit("Test".to_string(), "System".to_string())
            .await
            .unwrap();

        // Manually set to completed
        {
            let mut tasks = manager.tasks.write().await;
            let task = tasks.get_mut(&task_id).unwrap();
            task.status = TaskStatus::Completed;
        }

        let result = manager.cancel(&task_id).await;
        assert!(result.is_err(), "Cancelling a completed task should fail");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_resume_incomplete_tasks() {
        let dir = test_dir("resume");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();

        manager
            .submit("Task 1".to_string(), "System".to_string())
            .await
            .unwrap();
        manager
            .submit("Task 2".to_string(), "System".to_string())
            .await
            .unwrap();

        // Mark one as completed
        {
            let tasks = manager.tasks.read().await;
            let tasks_vec: Vec<&BackgroundTask> = tasks.values().collect();
            if let Some(task) = tasks_vec.first() {
                let id = task.id.clone();
                drop(tasks);
                let mut tasks = manager.tasks.write().await;
                if let Some(t) = tasks.get_mut(&id) {
                    t.status = TaskStatus::Completed;
                }
            }
        }

        let incomplete = manager.resume_incomplete().await;
        assert_eq!(incomplete.len(), 1, "One task should be incomplete");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_task_not_found() {
        let dir = test_dir("not_found");
        let manager = BackgroundTaskManager::new(&dir).await.unwrap();

        let result = manager.status("nonexistent").await;
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_persistence_across_restart() {
        let dir = test_dir("persist");

        // First session: submit a task
        {
            let manager = BackgroundTaskManager::new(&dir).await.unwrap();
            manager
                .submit("Persist test".to_string(), "System".to_string())
                .await
                .unwrap();
        } // manager drops

        // Second session: load from disk
        {
            let manager = BackgroundTaskManager::new(&dir).await.unwrap();
            let tasks = manager.list_tasks().await;
            assert_eq!(tasks.len(), 1, "Task should persist across restarts");
            assert_eq!(tasks[0].prompt, "Persist test");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
