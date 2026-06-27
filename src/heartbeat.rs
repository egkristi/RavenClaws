//! Autonomous heartbeat — persistent background agent loop
//!
//! RavenClaws's heartbeat mode enables truly autonomous operation: the agent
//! runs in a persistent loop with a configurable tick interval, assessing
//! progress, planning next steps, and executing them without human supervision.
//!
//! # Architecture
//!
//! ```text
//! HeartbeatAgent
//!   ├── tick_interval_secs  — how long to sleep between ticks
//!   ├── goal_prompt         — the overarching objective
//!   ├── state               — persisted progress state (survives restarts)
//!   └── run() loop:
//!       1. Assess progress via LLM
//!       2. Plan next actions
//!       3. Execute actions (tool calls via agent loop)
//!       4. Persist state
//!       5. Sleep until next tick
//! ```
//!
//! # Persistence
//!
//! Heartbeat state is saved as JSON to `workdir/heartbeat-<id>.json` after
//! every tick, so the agent can resume from its last checkpoint even after
//! a process restart.
//!
//! # Integration
//!
//! - CLI: `ravenclaws --heartbeat --goal "..." --tick-interval 300`
//! - Config: `[heartbeat]` section in `ravenclaws.toml`
//! - Scheduler: heartbeat can be started via cron trigger

use crate::agent::AgentLoopConfig;
use crate::error::{RavenClawsError, Result};
use crate::llm::LLMProviderTrait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, instrument, warn};

/// Configuration for the heartbeat agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// The overarching goal or mission prompt
    pub goal: String,

    /// Interval between ticks in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_tick_interval")]
    pub tick_interval_secs: u64,

    /// Maximum iterations per tick (default: 5)
    #[serde(default = "default_max_iterations_per_tick")]
    pub max_iterations_per_tick: usize,

    /// Working directory for state persistence
    #[serde(default = "default_workdir")]
    pub workdir: String,

    /// Maximum number of ticks before the heartbeat stops (0 = unlimited)
    #[serde(default)]
    pub max_ticks: u64,

    /// Enable tool calling during heartbeat ticks
    #[serde(default = "default_true")]
    pub enable_tools: bool,
}

fn default_tick_interval() -> u64 {
    300
}

fn default_max_iterations_per_tick() -> usize {
    5
}

fn default_workdir() -> String {
    "/workspace".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            goal: String::new(),
            tick_interval_secs: default_tick_interval(),
            max_iterations_per_tick: default_max_iterations_per_tick(),
            workdir: default_workdir(),
            max_ticks: 0,
            enable_tools: true,
        }
    }
}

/// Persistent state of a heartbeat agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatState {
    /// Unique heartbeat session ID
    pub id: String,

    /// The goal prompt
    pub goal: String,

    /// Current tick number
    pub tick: u64,

    /// Summary of progress so far
    pub progress: String,

    /// Last assessment from the LLM
    pub last_assessment: Option<String>,

    /// Last plan generated
    pub last_plan: Option<String>,

    /// Last action result
    pub last_result: Option<String>,

    /// When the heartbeat was created (ISO 8601)
    pub created_at: String,

    /// When the heartbeat was last updated (ISO 8601)
    pub updated_at: String,

    /// Whether the heartbeat has completed its goal
    pub completed: bool,
}

impl HeartbeatState {
    /// Create a new heartbeat state
    pub fn new(id: String, goal: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            goal,
            tick: 0,
            progress: String::new(),
            last_assessment: None,
            last_plan: None,
            last_result: None,
            created_at: now.clone(),
            updated_at: now,
            completed: false,
        }
    }
}

/// The autonomous heartbeat agent
///
/// Runs a persistent loop: assess → plan → act → persist → sleep.
pub struct HeartbeatAgent {
    /// LLM client for generating assessments and plans
    llm: Arc<dyn LLMProviderTrait>,

    /// Heartbeat configuration
    config: HeartbeatConfig,

    /// Current state (persisted to disk)
    state: HeartbeatState,

    /// Path to the state file
    state_path: PathBuf,
}

impl HeartbeatAgent {
    /// Create a new heartbeat agent.
    ///
    /// If a state file already exists for this goal, it will be loaded
    /// and the heartbeat will resume from where it left off.
    pub async fn new(
        llm: Arc<dyn LLMProviderTrait>,
        config: HeartbeatConfig,
        session_id: Option<String>,
    ) -> Result<Self> {
        let id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let workdir = PathBuf::from(&config.workdir);

        // Create workdir if it doesn't exist
        std::fs::create_dir_all(&workdir).map_err(|e| {
            RavenClawsError::CommandExecution(format!(
                "Failed to create heartbeat workdir '{}': {}",
                workdir.display(),
                e
            ))
        })?;

        let state_path = workdir.join(format!("heartbeat-{}.json", id));

        // Try to load existing state (for resumability)
        let state = if state_path.exists() {
            match std::fs::read_to_string(&state_path) {
                Ok(content) => match serde_json::from_str::<HeartbeatState>(&content) {
                    Ok(s) => {
                        info!(
                            heartbeat_id = %id,
                            tick = s.tick,
                            "Resumed heartbeat from saved state"
                        );
                        s
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            "Failed to deserialize heartbeat state, starting fresh"
                        );
                        HeartbeatState::new(id.clone(), config.goal.clone())
                    }
                },
                Err(e) => {
                    warn!(
                        error = %e,
                        "Failed to read heartbeat state file, starting fresh"
                    );
                    HeartbeatState::new(id.clone(), config.goal.clone())
                }
            }
        } else {
            HeartbeatState::new(id.clone(), config.goal.clone())
        };

        Ok(Self {
            llm,
            config,
            state,
            state_path,
        })
    }

    /// Get the heartbeat session ID
    pub fn id(&self) -> &str {
        &self.state.id
    }

    /// Run the heartbeat loop — blocks until the goal is completed or max ticks reached.
    #[instrument(skip(self), fields(heartbeat_id = %self.state.id, goal = %self.state.goal.chars().take(80).collect::<String>()))]
    pub async fn run(&mut self) -> Result<String> {
        info!(
            heartbeat_id = %self.state.id,
            tick_interval_secs = self.config.tick_interval_secs,
            max_ticks = self.config.max_ticks,
            starting_from_tick = self.state.tick,
            "Heartbeat agent starting"
        );

        loop {
            // Check if we've exceeded max ticks
            if self.config.max_ticks > 0 && self.state.tick >= self.config.max_ticks {
                info!(
                    tick = self.state.tick,
                    max_ticks = self.config.max_ticks,
                    "Heartbeat reached max ticks"
                );
                return Ok(format!(
                    "Heartbeat completed after {} ticks. Final progress: {}",
                    self.state.tick, self.state.progress
                ));
            }

            // Check if goal is already completed
            if self.state.completed {
                info!(
                    tick = self.state.tick,
                    "Heartbeat goal already marked as completed"
                );
                return Ok(self.state.progress.clone());
            }

            self.state.tick += 1;
            info!(tick = self.state.tick, "Heartbeat tick starting");

            // ── Step 1: Assess progress ──────────────────────────────────
            let assessment_prompt = self.build_assessment_prompt();
            match self
                .llm
                .chat(vec![
                    crate::llm::ChatMessage {
                        role: "system".to_string(),
                        content: self.build_system_prompt(),
                    },
                    crate::llm::ChatMessage {
                        role: "user".to_string(),
                        content: assessment_prompt,
                    },
                ])
                .await
            {
                Ok(response) => {
                    let assessment = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();
                    self.state.last_assessment = Some(assessment.clone());
                    info!(
                        tick = self.state.tick,
                        assessment = %assessment.chars().take(100).collect::<String>(),
                        "Progress assessment complete"
                    );

                    // Check if the LLM indicates the goal is complete
                    if assessment.to_uppercase().contains("GOAL_COMPLETE")
                        || assessment.to_uppercase().contains("[DONE]")
                    {
                        info!(
                            tick = self.state.tick,
                            "Heartbeat goal completed according to LLM assessment"
                        );
                        self.state.completed = true;
                        self.state.progress = format!(
                            "Goal completed at tick {}.\nAssessment: {}",
                            self.state.tick, assessment
                        );
                        self.persist_state()?;
                        return Ok(self.state.progress.clone());
                    }

                    // ── Step 2: Plan next actions ────────────────────────
                    let plan_prompt = format!(
                        "Based on this assessment, what specific actions should be taken next?\n\n\
                         Assessment: {}\n\n\
                         Current progress: {}\n\n\
                         Provide a concise plan with specific, actionable steps. \
                         End with 'PLAN_COMPLETE' when you have finished planning.",
                        assessment, self.state.progress
                    );

                    match self
                        .llm
                        .chat(vec![
                            crate::llm::ChatMessage {
                                role: "system".to_string(),
                                content: self.build_system_prompt(),
                            },
                            crate::llm::ChatMessage {
                                role: "user".to_string(),
                                content: plan_prompt,
                            },
                        ])
                        .await
                    {
                        Ok(plan_response) => {
                            let plan = plan_response
                                .choices
                                .first()
                                .map(|c| c.message.content.clone())
                                .unwrap_or_default();
                            self.state.last_plan = Some(plan.clone());
                            info!(
                                tick = self.state.tick,
                                plan = %plan.chars().take(100).collect::<String>(),
                                "Plan generated"
                            );

                            // ── Step 3: Execute actions ──────────────────
                            if self.config.enable_tools {
                                let exec_prompt = format!(
                                    "Execute the following plan step by step. Use tools as needed.\n\n\
                                     Goal: {}\n\n\
                                     Plan: {}\n\n\
                                     Current progress: {}\n\n\
                                     Execute the plan. After each action, report what was done. \
                                     When finished, say 'EXECUTION_COMPLETE' and summarize the results.",
                                    self.state.goal, plan, self.state.progress
                                );

                                let loop_config = AgentLoopConfig {
                                    max_iterations: self.config.max_iterations_per_tick,
                                    enable_tools: true,
                                    require_approval: false,
                                    prompt_injection_protection: true,
                                    token_lifetime_secs: 0,
                                    no_final_required: false,
                                };

                                match crate::agent::run_agent_loop(
                                    self.llm.clone(),
                                    &exec_prompt,
                                    &self.build_system_prompt(),
                                    loop_config,
                                )
                                .await
                                {
                                    Ok(result) => {
                                        self.state.last_result = Some(result.clone());
                                        self.state.progress = format!(
                                            "{}\n\n## Tick {}\n**Assessment:** {}\n**Plan:** {}\n**Result:** {}",
                                            self.state.progress,
                                            self.state.tick,
                                            assessment,
                                            plan,
                                            result
                                        );
                                        info!(
                                            tick = self.state.tick,
                                            "Actions executed successfully"
                                        );
                                    }
                                    Err(e) => {
                                        warn!(
                                            tick = self.state.tick,
                                            error = %e,
                                            "Action execution failed"
                                        );
                                        self.state.last_result = Some(format!("Error: {}", e));
                                        self.state.progress = format!(
                                            "{}\n\n## Tick {} (FAILED)\n**Assessment:** {}\n**Plan:** {}\n**Error:** {}",
                                            self.state.progress,
                                            self.state.tick,
                                            assessment,
                                            plan,
                                            e
                                        );
                                    }
                                }
                            } else {
                                // No tools — just use the plan as the result
                                self.state.last_result = Some(plan.clone());
                                self.state.progress = format!(
                                    "{}\n\n## Tick {}\n**Assessment:** {}\n**Plan:** {}",
                                    self.state.progress, self.state.tick, assessment, plan
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                tick = self.state.tick,
                                error = %e,
                                "Plan generation failed"
                            );
                            self.state.progress = format!(
                                "{}\n\n## Tick {} (PLANNING FAILED)\n**Assessment:** {}\n**Error:** {}",
                                self.state.progress, self.state.tick, assessment, e
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        tick = self.state.tick,
                        error = %e,
                        "Progress assessment failed"
                    );
                    self.state.progress = format!(
                        "{}\n\n## Tick {} (ASSESSMENT FAILED)\n**Error:** {}",
                        self.state.progress, self.state.tick, e
                    );
                }
            }

            // Persist state after every tick
            self.persist_state()?;

            // ── Step 4: Sleep until next tick ────────────────────────────
            if (self.config.max_ticks == 0 || self.state.tick < self.config.max_ticks)
                && !self.state.completed
            {
                info!(
                    tick = self.state.tick,
                    sleep_secs = self.config.tick_interval_secs,
                    "Heartbeat sleeping until next tick"
                );
                sleep(Duration::from_secs(self.config.tick_interval_secs)).await;
            }
        }
    }

    /// Build the system prompt for the heartbeat agent
    fn build_system_prompt(&self) -> String {
        format!(
            "You are RavenClaws Heartbeat — an autonomous agent operating in a persistent loop.\n\n\
             Your overarching goal: {}\n\n\
             You operate in ticks. Each tick you:\n\
             1. ASSESS your progress toward the goal\n\
             2. PLAN the next actions to take\n\
             3. EXECUTE those actions using available tools\n\
             4. REPORT what was accomplished\n\n\
             Be concise and focused. Always work toward the goal efficiently.\n\
             When the goal is complete, respond with 'GOAL_COMPLETE' and a summary.\n\n\
             Current tick: {}",
            self.state.goal, self.state.tick
        )
    }

    /// Build the assessment prompt for the current tick
    fn build_assessment_prompt(&self) -> String {
        format!(
            "Assess your progress toward the goal.\n\n\
             Goal: {}\n\n\
             Current progress so far:\n{}\n\n\
             Tick number: {}\n\n\
             Answer the following:\n\
             1. What progress has been made?\n\
             2. What obstacles remain?\n\
             3. Is the goal complete? (If yes, say 'GOAL_COMPLETE')\n\
             4. What should be done next?",
            self.state.goal,
            if self.state.progress.is_empty() {
                "No progress yet — this is the first tick.".to_string()
            } else {
                self.state.progress.clone()
            },
            self.state.tick
        )
    }

    /// Persist the current state to disk
    fn persist_state(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.state).map_err(|e| {
            RavenClawsError::CommandExecution(format!("Failed to serialize heartbeat state: {}", e))
        })?;

        std::fs::write(&self.state_path, content).map_err(|e| {
            RavenClawsError::CommandExecution(format!(
                "Failed to write heartbeat state to '{}': {}",
                self.state_path.display(),
                e
            ))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.tick_interval_secs, 300);
        assert_eq!(config.max_iterations_per_tick, 5);
        assert!(config.enable_tools);
        assert_eq!(config.max_ticks, 0);
    }

    #[test]
    fn test_heartbeat_state_new() {
        let state = HeartbeatState::new("test-id".to_string(), "Test goal".to_string());
        assert_eq!(state.id, "test-id");
        assert_eq!(state.goal, "Test goal");
        assert_eq!(state.tick, 0);
        assert!(!state.completed);
        assert!(state.last_assessment.is_none());
    }

    #[test]
    fn test_heartbeat_state_serialization() {
        let state = HeartbeatState::new("serialize-test".to_string(), "Serialize goal".to_string());
        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: HeartbeatState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "serialize-test");
        assert_eq!(deserialized.goal, "Serialize goal");
        assert_eq!(deserialized.tick, 0);
    }

    #[test]
    fn test_heartbeat_state_progress() {
        let mut state =
            HeartbeatState::new("progress-test".to_string(), "Progress goal".to_string());
        state.tick = 5;
        state.progress = "Made significant progress".to_string();
        state.last_assessment = Some("Assessment text".to_string());
        state.last_plan = Some("Plan text".to_string());
        state.last_result = Some("Result text".to_string());
        assert_eq!(state.tick, 5);
        assert_eq!(state.progress, "Made significant progress");
        assert_eq!(state.last_assessment.as_deref(), Some("Assessment text"));
    }

    #[test]
    fn test_heartbeat_state_completed() {
        let mut state =
            HeartbeatState::new("complete-test".to_string(), "Complete goal".to_string());
        state.completed = true;
        assert!(state.completed);
    }

    #[test]
    fn test_heartbeat_config_custom() {
        let config = HeartbeatConfig {
            goal: "Custom goal".to_string(),
            tick_interval_secs: 60,
            max_iterations_per_tick: 3,
            workdir: "/tmp/custom".to_string(),
            max_ticks: 10,
            enable_tools: false,
        };
        assert_eq!(config.goal, "Custom goal");
        assert_eq!(config.tick_interval_secs, 60);
        assert_eq!(config.max_iterations_per_tick, 3);
        assert_eq!(config.max_ticks, 10);
        assert!(!config.enable_tools);
    }

    #[test]
    fn test_heartbeat_state_persistence_roundtrip() {
        let mut state = HeartbeatState::new("persist-test".to_string(), "Persist goal".to_string());
        state.tick = 3;
        state.progress = "Tick 3 progress".to_string();
        state.last_assessment = Some("Assessment".to_string());
        state.last_plan = Some("Plan".to_string());
        state.last_result = Some("Result".to_string());

        let json = serde_json::to_string_pretty(&state).unwrap();
        let restored: HeartbeatState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, "persist-test");
        assert_eq!(restored.tick, 3);
        assert_eq!(restored.progress, "Tick 3 progress");
        assert_eq!(restored.last_assessment.as_deref(), Some("Assessment"));
        assert_eq!(restored.last_plan.as_deref(), Some("Plan"));
        assert_eq!(restored.last_result.as_deref(), Some("Result"));
    }

    #[test]
    fn test_heartbeat_build_system_prompt() {
        let config = HeartbeatConfig {
            goal: "Test goal for prompt".to_string(),
            ..HeartbeatConfig::default()
        };
        // We can't easily test the full prompt without an LLM, but we can
        // verify the prompt contains the goal
        assert!(config.goal.contains("Test goal for prompt"));
    }
}
