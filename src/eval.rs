//! RavenClaws
//!
//! Provides a framework for defining, running, and scoring evaluation tasks
//! against LLM agents. Captures full run traces for inspection and debugging.
//!
//! # Architecture
//!
//! ```text
//! EvalConfig (TOML file)
//!   └── Vec<EvalTask>
//!         ├── prompt + golden answer
//!         ├── assertions (contains, not_contains, regex, exact)
//!         └── scoring weights
//!
//! EvalRunner
//!   ├── run_task() → EvalResult (with RunTrace)
//!   └── run_suite() → EvalReport (summary of all results)
//!
//! RunTrace
//!   ├── steps: Vec<TraceStep>
//!   ├── llm_calls: Vec<LlmCallTrace>
//!   └── tool_calls: Vec<ToolCallTrace>
//! ```

use crate::agent::{run_agent_loop, AgentLoopConfig};
use crate::error::{RavenClawsError, Result};
use crate::llm::LLMProviderTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for an eval suite — loaded from a TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    /// Name of this eval suite
    #[serde(default = "default_suite_name")]
    pub name: String,
    /// Description of what this suite tests
    #[serde(default)]
    pub description: String,
    /// System prompt to use for all tasks in this suite
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    /// Maximum iterations per task
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// List of eval tasks to run
    #[serde(default)]
    pub tasks: Vec<EvalTask>,
}

fn default_suite_name() -> String {
    "unnamed".to_string()
}

fn default_system_prompt() -> String {
    "You are a helpful assistant. Be concise and accurate.".to_string()
}

fn default_max_iterations() -> usize {
    5
}

/// A single eval task with prompt, golden answer, and assertions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    /// Name of this task (used in reports)
    pub name: String,
    /// Description of what this task tests
    #[serde(default)]
    pub description: String,
    /// The prompt to send to the agent
    pub prompt: String,
    /// Expected golden answer (used for exact match scoring)
    #[serde(default)]
    pub golden: String,
    /// List of assertions to check against the response
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    /// Weight of this task in the overall score (0.0 - 1.0)
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// Whether this task is required to pass (fails the suite if not)
    #[serde(default)]
    pub required: bool,
}

fn default_weight() -> f64 {
    1.0
}

/// Types of assertions that can be checked against a response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Assertion {
    /// Response must contain this substring
    #[serde(rename = "contains")]
    Contains(String),
    /// Response must NOT contain this substring
    #[serde(rename = "not_contains")]
    NotContains(String),
    /// Response must exactly match this string
    #[serde(rename = "exact")]
    Exact(String),
    /// Response must match this regex pattern
    #[serde(rename = "regex")]
    Regex(String),
    /// Response must be non-empty
    #[serde(rename = "non_empty")]
    NonEmpty,
    /// Response length must be at least N characters
    #[serde(rename = "min_length")]
    MinLength(usize),
    /// Response length must be at most N characters
    #[serde(rename = "max_length")]
    MaxLength(usize),
    /// A tool with this name must have been called during execution (v0.9.6)
    #[serde(rename = "tool_called")]
    ToolCalled(String),
    /// A tool with this name must NOT have been called during execution (v0.9.6)
    #[serde(rename = "tool_not_called")]
    ToolNotCalled(String),
}

// ── Run Trace ───────────────────────────────────────────────────────────────

/// Full trace of a single agent run — captures every step for inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTrace {
    /// Task name
    pub task_name: String,
    /// When the run started (ISO 8601)
    pub started_at: String,
    /// When the run ended (ISO 8601)
    pub ended_at: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of iterations used
    pub iterations: usize,
    /// All steps in chronological order
    pub steps: Vec<TraceStep>,
    /// LLM calls made during the run
    pub llm_calls: Vec<LlmCallTrace>,
    /// Tool calls made during the run
    pub tool_calls: Vec<ToolCallTrace>,
    /// Final response from the agent
    pub final_response: String,
}

/// A single step in the agent loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    /// Step number (0-based)
    pub number: usize,
    /// Type of step
    pub step_type: StepType,
    /// Content of the step (LLM response, tool result, etc.)
    pub content: String,
    /// Duration of this step in milliseconds
    pub duration_ms: u64,
}

/// Type of a trace step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// LLM thought/response
    Thought,
    /// Tool call
    ToolCall,
    /// Tool result/observation
    Observation,
    /// Final answer
    Final,
    /// Error
    Error,
}

/// Trace of a single LLM call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallTrace {
    /// Iteration number
    pub iteration: usize,
    /// Provider name
    pub provider: String,
    /// Model name
    pub model: String,
    /// Prompt tokens (if available)
    pub prompt_tokens: Option<u32>,
    /// Completion tokens (if available)
    pub completion_tokens: Option<u32>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Response content (truncated to 1000 chars for storage)
    pub response_preview: String,
}

/// Trace of a single tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallTrace {
    /// Iteration number
    pub iteration: usize,
    /// Tool name
    pub tool_name: String,
    /// Arguments (JSON)
    pub arguments: serde_json::Value,
    /// Whether the tool succeeded
    pub success: bool,
    /// Output preview (truncated to 500 chars)
    pub output_preview: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

// ── Results ─────────────────────────────────────────────────────────────────

/// Result of a single eval task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// Task name
    pub task_name: String,
    /// Whether the task passed all assertions
    pub passed: bool,
    /// Score (0.0 - 1.0)
    pub score: f64,
    /// Number of assertions that passed
    pub assertions_passed: usize,
    /// Number of assertions that failed
    pub assertions_failed: usize,
    /// Details of each assertion check
    pub assertion_results: Vec<AssertionResult>,
    /// Full run trace for inspection
    pub trace: RunTrace,
    /// Error message if the task failed to run
    pub error: Option<String>,
}

/// Result of a single assertion check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// The assertion that was checked
    pub assertion: String,
    /// Whether it passed
    pub passed: bool,
    /// Details about the check
    pub details: String,
}

/// Summary report of an entire eval suite run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    /// Suite name
    pub suite_name: String,
    /// When the suite was run (ISO 8601)
    pub ran_at: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Overall score (0.0 - 1.0)
    pub overall_score: f64,
    /// Number of tasks
    pub total_tasks: usize,
    /// Number of tasks that passed
    pub passed_tasks: usize,
    /// Number of tasks that failed
    pub failed_tasks: usize,
    /// Individual task results
    pub results: Vec<EvalResult>,
}

// ── Eval Runner ─────────────────────────────────────────────────────────────

/// Runs eval tasks against an LLM provider and captures traces
pub struct EvalRunner {
    /// The LLM provider to test
    llm: Arc<dyn LLMProviderTrait>,
    /// Eval configuration
    config: EvalConfig,
}

impl EvalRunner {
    /// Create a new eval runner
    pub fn new(llm: Arc<dyn LLMProviderTrait>, config: EvalConfig) -> Self {
        Self { llm, config }
    }

    /// Run the full eval suite and return a report
    #[instrument(skip(self), fields(suite = %self.config.name, task_count = self.config.tasks.len()))]
    pub async fn run_suite(&self) -> EvalReport {
        let started_at = chrono::Utc::now().to_rfc3339();
        let suite_start = std::time::Instant::now();
        let mut results = Vec::with_capacity(self.config.tasks.len());

        info!(
            suite = %self.config.name,
            task_count = self.config.tasks.len(),
            "Starting eval suite"
        );

        for task in &self.config.tasks {
            let result = self.run_task(task).await;
            let passed = result.passed;
            let name = &result.task_name;

            if passed {
                info!(task = %name, score = result.score, "Eval task passed");
            } else {
                warn!(
                    task = %name,
                    score = result.score,
                    passed = result.assertions_passed,
                    failed = result.assertions_failed,
                    "Eval task failed"
                );
            }

            results.push(result);
        }

        let duration_ms = suite_start.elapsed().as_millis() as u64;
        let total_tasks = results.len();
        let passed_tasks = results.iter().filter(|r| r.passed).count();
        let failed_tasks = total_tasks - passed_tasks;
        let overall_score = if total_tasks > 0 {
            results
                .iter()
                .map(|r| r.score * r.trace.iterations as f64)
                .sum::<f64>()
                / results
                    .iter()
                    .map(|r| r.trace.iterations as f64)
                    .sum::<f64>()
        } else {
            0.0
        };

        info!(
            suite = %self.config.name,
            passed = passed_tasks,
            failed = failed_tasks,
            overall_score = overall_score,
            duration_ms = duration_ms,
            "Eval suite completed"
        );

        EvalReport {
            suite_name: self.config.name.clone(),
            ran_at: started_at,
            duration_ms,
            overall_score,
            total_tasks,
            passed_tasks,
            failed_tasks,
            results,
        }
    }

    /// Run a single eval task and return the result with trace
    ///
    /// Uses the full agent loop (`run_agent_loop`) instead of a single LLM call,
    /// so eval tasks exercise the complete ReAct loop with tool use, security
    /// checks, and iteration limits.
    #[instrument(skip(self), fields(task = %task.name))]
    async fn run_task(&self, task: &EvalTask) -> EvalResult {
        let task_start = std::time::Instant::now();
        let started_at = chrono::Utc::now().to_rfc3339();

        // Build agent loop config from suite settings
        let agent_config = AgentLoopConfig {
            max_iterations: self.config.max_iterations,
            enable_tools: true,
            require_approval: false,
            prompt_injection_protection: true,
            token_lifetime_secs: 0,
            no_final_required: false,
            fallback_chain: None,
            token_budget: None,
            ravenfabric: None,
        };

        // Run the full agent loop (ReAct + tools + security)
        let result = run_agent_loop(
            self.llm.clone(),
            &task.prompt,
            &self.config.system_prompt,
            agent_config,
        )
        .await;

        let duration_ms = task_start.elapsed().as_millis() as u64;

        match result {
            Ok(final_response) => {
                let trace = RunTrace {
                    task_name: task.name.clone(),
                    started_at,
                    ended_at: chrono::Utc::now().to_rfc3339(),
                    duration_ms,
                    iterations: self.config.max_iterations, // best-effort; agent loop doesn't expose exact count
                    steps: vec![TraceStep {
                        number: 0,
                        step_type: StepType::Final,
                        content: final_response.clone(),
                        duration_ms,
                    }],
                    llm_calls: Vec::new(), // agent loop doesn't expose per-call traces
                    tool_calls: Vec::new(), // agent loop doesn't expose per-call traces
                    final_response: final_response.clone(),
                };

                // Run assertions against the final response
                let (assertion_results, assertions_passed, assertions_failed) =
                    check_assertions(&final_response, &task.assertions, Some(&trace));

                // Calculate score
                let score = if task.assertions.is_empty() {
                    if final_response.is_empty() || final_response.len() < 10 {
                        0.0
                    } else {
                        1.0
                    }
                } else if task.assertions.len() == assertions_passed + assertions_failed {
                    assertions_passed as f64 / task.assertions.len() as f64
                } else {
                    0.0
                };

                let passed = assertions_failed == 0 && !final_response.is_empty();

                EvalResult {
                    task_name: task.name.clone(),
                    passed,
                    score,
                    assertions_passed,
                    assertions_failed,
                    assertion_results,
                    trace,
                    error: None,
                }
            }
            Err(e) => {
                let trace = RunTrace {
                    task_name: task.name.clone(),
                    started_at,
                    ended_at: chrono::Utc::now().to_rfc3339(),
                    duration_ms,
                    iterations: 0,
                    steps: vec![TraceStep {
                        number: 0,
                        step_type: StepType::Error,
                        content: format!("Agent loop failed: {}", e),
                        duration_ms,
                    }],
                    llm_calls: Vec::new(),
                    tool_calls: Vec::new(),
                    final_response: String::new(),
                };

                EvalResult {
                    task_name: task.name.clone(),
                    passed: false,
                    score: 0.0,
                    assertions_passed: 0,
                    assertions_failed: 1,
                    assertion_results: vec![AssertionResult {
                        assertion: "agent_loop".to_string(),
                        passed: false,
                        details: format!("Agent loop failed: {}", e),
                    }],
                    trace,
                    error: Some(e.to_string()),
                }
            }
        }
    }
}

// ── Assertion Checking ──────────────────────────────────────────────────────

/// Check all assertions against a response string
fn check_assertions(
    response: &str,
    assertions: &[Assertion],
    run_trace: Option<&RunTrace>,
) -> (Vec<AssertionResult>, usize, usize) {
    let mut results = Vec::with_capacity(assertions.len());
    let mut passed = 0;
    let mut failed = 0;

    for assertion in assertions {
        let result = check_single_assertion(response, assertion, run_trace);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
        }
        results.push(result);
    }

    (results, passed, failed)
}

/// Check a single assertion against a response
fn check_single_assertion(
    response: &str,
    assertion: &Assertion,
    run_trace: Option<&RunTrace>,
) -> AssertionResult {
    match assertion {
        Assertion::Contains(pattern) => {
            let passed = response.contains(pattern);
            AssertionResult {
                assertion: format!("contains: {}", pattern),
                passed,
                details: if passed {
                    format!("Response contains '{}'", pattern)
                } else {
                    format!("Response does not contain '{}'", pattern)
                },
            }
        }
        Assertion::NotContains(pattern) => {
            let passed = !response.contains(pattern);
            AssertionResult {
                assertion: format!("not_contains: {}", pattern),
                passed,
                details: if passed {
                    format!("Response does not contain '{}'", pattern)
                } else {
                    format!("Response contains '{}'", pattern)
                },
            }
        }
        Assertion::Exact(expected) => {
            let trimmed_response = response.trim();
            let passed = trimmed_response == expected.as_str();
            AssertionResult {
                assertion: format!("exact: {}", expected),
                passed,
                details: if passed {
                    "Response matches exactly".to_string()
                } else {
                    format!(
                        "Expected '{}', got '{}'",
                        expected,
                        trimmed_response.chars().take(100).collect::<String>()
                    )
                },
            }
        }
        Assertion::Regex(pattern) => {
            let re = regex_lite::Regex::new(pattern);
            match re {
                Ok(re) => {
                    let passed = re.is_match(response);
                    AssertionResult {
                        assertion: format!("regex: {}", pattern),
                        passed,
                        details: if passed {
                            format!("Response matches pattern '{}'", pattern)
                        } else {
                            format!("Response does not match pattern '{}'", pattern)
                        },
                    }
                }
                Err(e) => AssertionResult {
                    assertion: format!("regex: {}", pattern),
                    passed: false,
                    details: format!("Invalid regex pattern: {}", e),
                },
            }
        }
        Assertion::NonEmpty => {
            let passed = !response.is_empty();
            AssertionResult {
                assertion: "non_empty".to_string(),
                passed,
                details: if passed {
                    format!("Response is non-empty ({} chars)", response.len())
                } else {
                    "Response is empty".to_string()
                },
            }
        }
        Assertion::MinLength(min) => {
            let passed = response.len() >= *min;
            AssertionResult {
                assertion: format!("min_length: {}", min),
                passed,
                details: if passed {
                    format!("Response length {} >= {}", response.len(), min)
                } else {
                    format!("Response length {} < {}", response.len(), min)
                },
            }
        }
        Assertion::MaxLength(max) => {
            let passed = response.len() <= *max;
            AssertionResult {
                assertion: format!("max_length: {}", max),
                passed,
                details: if passed {
                    format!("Response length {} <= {}", response.len(), max)
                } else {
                    format!("Response length {} > {}", response.len(), max)
                },
            }
        }
        Assertion::ToolCalled(tool_name) => {
            let tool_calls = run_trace
                .map(|t| &t.tool_calls)
                .filter(|calls| calls.iter().any(|tc| tc.tool_name == *tool_name));
            let passed = tool_calls.is_some();
            AssertionResult {
                assertion: format!("tool_called: {}", tool_name),
                passed,
                details: if passed {
                    format!("Tool '{}' was called", tool_name)
                } else {
                    let all_tools: Vec<&str> = run_trace
                        .map(|t| {
                            t.tool_calls
                                .iter()
                                .map(|tc| tc.tool_name.as_str())
                                .collect()
                        })
                        .unwrap_or_default();
                    if all_tools.is_empty() {
                        format!("Tool '{}' was not called (no tools were called)", tool_name)
                    } else {
                        format!(
                            "Tool '{}' was not called (called: {})",
                            tool_name,
                            all_tools.join(", ")
                        )
                    }
                },
            }
        }
        Assertion::ToolNotCalled(tool_name) => {
            let tool_calls = run_trace
                .map(|t| &t.tool_calls)
                .filter(|calls| calls.iter().any(|tc| tc.tool_name == *tool_name));
            let passed = tool_calls.is_none();
            AssertionResult {
                assertion: format!("tool_not_called: {}", tool_name),
                passed,
                details: if passed {
                    format!("Tool '{}' was not called", tool_name)
                } else {
                    format!("Tool '{}' was called but should not have been", tool_name)
                },
            }
        }
    }
}

// ── Report Formatting ───────────────────────────────────────────────────────

impl EvalReport {
    /// Format the report as a human-readable string
    pub fn format_text(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("\n🐦‍⬛ Eval Report: {}\n", self.suite_name));
        output.push_str(&format!("{:-^60}\n", ""));
        output.push_str(&format!(
            "Ran at:       {}\n",
            &self.ran_at[..19].replace('T', " ")
        ));
        output.push_str(&format!("Duration:     {} ms\n", self.duration_ms));
        output.push_str(&format!(
            "Overall score: {:.1}%\n",
            self.overall_score * 100.0
        ));
        output.push_str(&format!(
            "Tasks:        {}/{} passed\n",
            self.passed_tasks, self.total_tasks
        ));
        output.push_str(&format!("{:-^60}\n", ""));

        for result in &self.results {
            output.push_str(&format!(
                "\n  {} {} — {:.1}%\n",
                if result.passed { "✅" } else { "❌" },
                result.task_name,
                result.score * 100.0
            ));

            if let Some(ref error) = result.error {
                output.push_str(&format!("    Error: {}\n", error));
            }

            if !result.assertion_results.is_empty() {
                for ar in &result.assertion_results {
                    output.push_str(&format!(
                        "    {} {}\n",
                        if ar.passed { "  ✅" } else { "  ❌" },
                        ar.details
                    ));
                }
            }

            // Show trace summary
            let trace = &result.trace;
            output.push_str(&format!(
                "    Iterations: {} · LLM calls: {} · Tool calls: {} · Duration: {} ms\n",
                trace.iterations,
                trace.llm_calls.len(),
                trace.tool_calls.len(),
                trace.duration_ms
            ));

            // Show response preview
            let preview: String = trace.final_response.chars().take(200).collect();
            if !preview.is_empty() {
                output.push_str(&format!("    Response: {}\n", preview));
            }
        }

        output
    }

    /// Format the report as JSON
    pub fn format_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({"error": "serialization failed"}))
    }
}

// ── Config Loading ──────────────────────────────────────────────────────────

impl EvalConfig {
    /// Load eval config from a TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            RavenClawsError::CommandExecution(format!("Failed to read eval config: {}", e))
        })?;

        let config: EvalConfig = toml::from_str(&content).map_err(|e| {
            RavenClawsError::CommandExecution(format!("Failed to parse eval config: {}", e))
        })?;

        Ok(config)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_contains_pass() {
        let result = check_single_assertion(
            "hello world",
            &Assertion::Contains("world".to_string()),
            None,
        );
        assert!(result.passed);
        assert!(result.details.contains("contains"));
    }

    #[test]
    fn test_assertion_contains_fail() {
        let result =
            check_single_assertion("hello world", &Assertion::Contains("foo".to_string()), None);
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_not_contains_pass() {
        let result = check_single_assertion(
            "hello world",
            &Assertion::NotContains("foo".to_string()),
            None,
        );
        assert!(result.passed);
    }

    #[test]
    fn test_assertion_not_contains_fail() {
        let result = check_single_assertion(
            "hello world",
            &Assertion::NotContains("world".to_string()),
            None,
        );
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_exact_pass() {
        let result = check_single_assertion("hello", &Assertion::Exact("hello".to_string()), None);
        assert!(result.passed);
    }

    #[test]
    fn test_assertion_exact_fail() {
        let result = check_single_assertion("world", &Assertion::Exact("hello".to_string()), None);
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_regex_pass() {
        let result =
            check_single_assertion("hello 123", &Assertion::Regex(r"\d+".to_string()), None);
        assert!(result.passed);
    }

    #[test]
    fn test_assertion_regex_fail() {
        let result = check_single_assertion("hello", &Assertion::Regex(r"\d+".to_string()), None);
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_non_empty_pass() {
        let result = check_single_assertion("hello", &Assertion::NonEmpty, None);
        assert!(result.passed);
    }

    #[test]
    fn test_assertion_non_empty_fail() {
        let result = check_single_assertion("", &Assertion::NonEmpty, None);
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_min_length_pass() {
        let result = check_single_assertion("hello", &Assertion::MinLength(3), None);
        assert!(result.passed);
    }

    #[test]
    fn test_assertion_min_length_fail() {
        let result = check_single_assertion("hi", &Assertion::MinLength(5), None);
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_max_length_pass() {
        let result = check_single_assertion("hi", &Assertion::MaxLength(5), None);
        assert!(result.passed);
    }

    #[test]
    fn test_assertion_max_length_fail() {
        let result = check_single_assertion("hello world", &Assertion::MaxLength(5), None);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_assertions_empty() {
        let (results, passed, failed) = check_assertions("hello", &[], None);
        assert!(results.is_empty());
        assert_eq!(passed, 0);
        assert_eq!(failed, 0);
    }

    #[test]
    fn test_check_assertions_multiple() {
        let assertions = vec![
            Assertion::Contains("hello".to_string()),
            Assertion::Contains("world".to_string()),
            Assertion::NonEmpty,
        ];
        let (results, passed, failed) = check_assertions("hello world", &assertions, None);
        assert_eq!(passed, 3);
        assert_eq!(failed, 0);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_check_assertions_tool_called() {
        let trace = RunTrace {
            task_name: "test".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: "2026-01-01T00:00:01Z".to_string(),
            duration_ms: 1000,
            iterations: 1,
            steps: vec![],
            llm_calls: vec![],
            tool_calls: vec![
                ToolCallTrace {
                    iteration: 0,
                    tool_name: "web_search".to_string(),
                    arguments: serde_json::json!({"query": "test"}),
                    success: true,
                    output_preview: "results".to_string(),
                    duration_ms: 100,
                },
                ToolCallTrace {
                    iteration: 0,
                    tool_name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": "/tmp/test"}),
                    success: true,
                    output_preview: "content".to_string(),
                    duration_ms: 50,
                },
            ],
            final_response: "response".to_string(),
        };

        // ToolCalled — should pass
        let (results, passed, failed) = check_assertions(
            "response",
            &[Assertion::ToolCalled("web_search".to_string())],
            Some(&trace),
        );
        assert_eq!(passed, 1);
        assert_eq!(failed, 0);
        assert!(results[0].passed);

        // ToolCalled — should fail (tool not called)
        let (results, passed, failed) = check_assertions(
            "response",
            &[Assertion::ToolCalled("nonexistent".to_string())],
            Some(&trace),
        );
        assert_eq!(passed, 0);
        assert_eq!(failed, 1);
        assert!(!results[0].passed);

        // ToolNotCalled — should pass (tool not in list)
        let (results, passed, failed) = check_assertions(
            "response",
            &[Assertion::ToolNotCalled("nonexistent".to_string())],
            Some(&trace),
        );
        assert_eq!(passed, 1);
        assert_eq!(failed, 0);
        assert!(results[0].passed);

        // ToolNotCalled — should fail (tool was called)
        let (results, passed, failed) = check_assertions(
            "response",
            &[Assertion::ToolNotCalled("web_search".to_string())],
            Some(&trace),
        );
        assert_eq!(passed, 0);
        assert_eq!(failed, 1);
        assert!(!results[0].passed);

        // ToolCalled with no trace — should fail
        let (results, passed, failed) = check_assertions(
            "response",
            &[Assertion::ToolCalled("web_search".to_string())],
            None,
        );
        assert_eq!(passed, 0);
        assert_eq!(failed, 1);
        assert!(!results[0].passed);
    }

    #[test]
    fn test_eval_config_from_toml() {
        let toml_str = r#"
name = "test-suite"
description = "A test suite"
system_prompt = "Be concise"
max_iterations = 3

[[tasks]]
name = "test-1"
prompt = "What is 2+2?"
golden = "4"
assertions = [{ type = "contains", value = "4" }]
weight = 1.0
required = true
"#;

        let config: EvalConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "test-suite");
        assert_eq!(config.tasks.len(), 1);
        assert_eq!(config.tasks[0].name, "test-1");
        assert_eq!(config.tasks[0].prompt, "What is 2+2?");
        assert_eq!(config.tasks[0].golden, "4");
        assert_eq!(config.tasks[0].assertions.len(), 1);
    }

    #[test]
    fn test_eval_config_defaults() {
        let toml_str = r#"
[[tasks]]
name = "simple"
prompt = "Say hello"
"#;

        let config: EvalConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "unnamed");
        assert_eq!(config.system_prompt, default_system_prompt());
        assert_eq!(config.max_iterations, 5);
        assert_eq!(config.tasks[0].weight, 1.0);
        assert!(!config.tasks[0].required);
    }

    #[test]
    fn test_report_format_text() {
        let report = EvalReport {
            suite_name: "test".to_string(),
            ran_at: "2026-06-22T12:00:00+00:00".to_string(),
            duration_ms: 100,
            overall_score: 0.75,
            total_tasks: 2,
            passed_tasks: 1,
            failed_tasks: 1,
            results: vec![
                EvalResult {
                    task_name: "pass-task".to_string(),
                    passed: true,
                    score: 1.0,
                    assertions_passed: 2,
                    assertions_failed: 0,
                    assertion_results: vec![AssertionResult {
                        assertion: "contains: hello".to_string(),
                        passed: true,
                        details: "Response contains 'hello'".to_string(),
                    }],
                    trace: RunTrace {
                        task_name: "pass-task".to_string(),
                        started_at: "2026-06-22T12:00:00+00:00".to_string(),
                        ended_at: "2026-06-22T12:00:01+00:00".to_string(),
                        duration_ms: 50,
                        iterations: 1,
                        steps: vec![],
                        llm_calls: vec![],
                        tool_calls: vec![],
                        final_response: "hello world".to_string(),
                    },
                    error: None,
                },
                EvalResult {
                    task_name: "fail-task".to_string(),
                    passed: false,
                    score: 0.0,
                    assertions_passed: 0,
                    assertions_failed: 1,
                    assertion_results: vec![AssertionResult {
                        assertion: "contains: foo".to_string(),
                        passed: false,
                        details: "Response does not contain 'foo'".to_string(),
                    }],
                    trace: RunTrace {
                        task_name: "fail-task".to_string(),
                        started_at: "2026-06-22T12:00:01+00:00".to_string(),
                        ended_at: "2026-06-22T12:00:02+00:00".to_string(),
                        duration_ms: 50,
                        iterations: 1,
                        steps: vec![],
                        llm_calls: vec![],
                        tool_calls: vec![],
                        final_response: "bar".to_string(),
                    },
                    error: None,
                },
            ],
        };

        let text = report.format_text();
        assert!(text.contains("Eval Report: test"));
        assert!(text.contains("75.0%"));
        assert!(text.contains("1/2 passed"));
        assert!(text.contains("✅ pass-task"));
        assert!(text.contains("❌ fail-task"));
    }

    #[test]
    fn test_report_format_json() {
        let report = EvalReport {
            suite_name: "test".to_string(),
            ran_at: "2026-06-22T12:00:00+00:00".to_string(),
            duration_ms: 100,
            overall_score: 1.0,
            total_tasks: 1,
            passed_tasks: 1,
            failed_tasks: 0,
            results: vec![],
        };

        let json = report.format_json();
        assert_eq!(json["suite_name"], "test");
        assert_eq!(json["overall_score"], 1.0);
    }

    #[test]
    fn test_eval_config_from_file_not_found() {
        let result = EvalConfig::from_file("/tmp/nonexistent-eval-config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_assertion_regex_invalid_pattern() {
        let result =
            check_single_assertion("hello", &Assertion::Regex(r"[invalid".to_string()), None);
        assert!(!result.passed);
        assert!(result.details.contains("Invalid regex"));
    }

    #[test]
    fn test_trace_step_serialization() {
        let step = TraceStep {
            number: 0,
            step_type: StepType::Thought,
            content: "test".to_string(),
            duration_ms: 100,
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("Thought"));
    }

    #[test]
    fn test_tool_call_trace_serialization() {
        let trace = ToolCallTrace {
            iteration: 0,
            tool_name: "shell_exec".to_string(),
            arguments: serde_json::json!({"command": "echo hello"}),
            success: true,
            output_preview: "hello".to_string(),
            duration_ms: 50,
        };
        let json = serde_json::to_string(&trace).unwrap();
        assert!(json.contains("shell_exec"));
        assert!(json.contains("echo hello"));
    }
}
