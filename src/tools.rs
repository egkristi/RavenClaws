//! RavenClaws
//!
//! Provides a provider-agnostic tool schema, a registry for built-in tools,
//! and the execution engine that routes tool calls to their implementations.
//!
//! # Architecture
//!
//! ```text
//! ToolRegistry (holds all registered tools)
//!   ├── ToolDefinition (name, description, JSON schema)
//!   └── ToolImpl (the actual implementation)
//!         ├── ShellTool — execute shell commands (sandboxed)
//!         ├── ReadFileTool — read files (policy-checked)
//!         ├── WriteFileTool — write files (policy-checked)
//!         ├── WebFetchTool — fetch URLs (policy-checked)
//!         └── ... more tools
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, instrument, warn};

// Re-export sandbox for tool implementations
use crate::sandbox::Sandbox;

// ── Error types ────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{0}' not found")]
    NotFound(String),

    #[error("Tool '{0}' execution failed: {1}")]
    ExecutionFailed(String, String),

    #[error("Invalid arguments for tool '{0}': {1}")]
    InvalidArguments(String, String),

    #[allow(dead_code)]
    #[error("Policy denied: {0}")]
    PolicyDenied(String),

    #[allow(dead_code)]
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ToolResultValue<T> = std::result::Result<T, ToolError>;

// ── Tool schema types ──────────────────────────────────────────────────────

/// JSON Schema representation for tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, JsonSchema>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl JsonSchema {
    /// Create a string schema property
    pub fn string(description: &str) -> Self {
        Self {
            schema_type: "string".to_string(),
            description: Some(description.to_string()),
            properties: None,
            required: None,
            items: None,
            enum_values: None,
        }
    }

    /// Create an object schema
    pub fn object(properties: HashMap<String, JsonSchema>, required: Vec<String>) -> Self {
        Self {
            schema_type: "object".to_string(),
            description: None,
            properties: Some(properties),
            required: Some(required),
            items: None,
            enum_values: None,
        }
    }

    /// Create an array schema
    #[allow(dead_code)]
    pub fn array(items: JsonSchema, description: &str) -> Self {
        Self {
            schema_type: "array".to_string(),
            description: Some(description.to_string()),
            properties: None,
            required: None,
            items: Some(Box::new(items)),
            enum_values: None,
        }
    }
}

/// A tool definition — the schema exposed to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The name of the tool (e.g., "shell_exec", "read_file")
    pub name: String,
    /// A description of what the tool does (for the LLM)
    pub description: String,
    /// JSON Schema for the tool's parameters
    pub parameters: JsonSchema,
    /// Whether this tool requires human approval
    #[serde(default)]
    pub requires_approval: bool,
    /// Category for grouping
    #[serde(default)]
    pub category: ToolCategory,
}

impl ToolDefinition {
    /// Convert to OpenAI Tools format for structured function calling
    /// See: https://platform.openai.com/docs/guides/function-calling
    #[allow(dead_code)]
    pub fn to_openai_tool(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters
            }
        })
    }
}

/// Tool categories for grouping and policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ToolCategory {
    #[default]
    General,
    Shell,
    FileSystem,
    Network,
    CodeAnalysis,
    WebSearch,
    Mcp,
}

/// A tool call request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The name of the tool to call
    pub name: String,
    /// The arguments as a JSON object
    pub arguments: serde_json::Value,
    /// An optional ID for tracking (used by some providers)
    #[serde(default)]
    pub id: Option<String>,
}

/// The result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The name of the tool that was called
    pub tool_name: String,
    /// Whether the execution was successful
    pub success: bool,
    /// The output (stdout or result data)
    pub output: String,
    /// Error message if failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Exit code (for shell commands)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Duration in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

// ── Tool implementation trait ──────────────────────────────────────────────

/// The actual implementation of a tool
#[async_trait::async_trait]
pub trait ToolImpl: Send + Sync {
    /// Execute the tool with the given arguments
    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult>;

    /// Get the tool's definition (schema)
    fn definition(&self) -> &ToolDefinition;

    /// Get a display name for logging
    fn name(&self) -> &str {
        &self.definition().name
    }
}

// ── Tool registry ──────────────────────────────────────────────────────────

/// Registry of all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolImpl>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn ToolImpl>) {
        let name = tool.name().to_string();
        info!(tool = %name, category = ?tool.definition().category, "Tool registered");
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn ToolImpl>> {
        self.tools.get(name)
    }

    /// Check if a tool exists
    #[allow(dead_code)]
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool definitions (for sending to LLM)
    #[allow(dead_code)]
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| t.definition().clone())
            .collect()
    }

    /// Get all tool definitions in OpenAI Tools format for structured function calling
    #[allow(dead_code)]
    pub fn to_openai_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| t.definition().to_openai_tool())
            .collect()
    }

    /// Get the number of registered tools
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Execute a tool call
    #[instrument(skip(self), fields(tool = %call.name))]
    pub async fn execute(&self, call: ToolCall) -> ToolResultValue<ToolResult> {
        let start = std::time::Instant::now();

        let tool = self
            .get(&call.name)
            .ok_or_else(|| ToolError::NotFound(call.name.clone()))?;

        info!(tool = %call.name, "Executing tool call");

        let mut result = tool.execute(call.arguments).await?;
        result.duration_ms = Some(start.elapsed().as_millis() as u64);

        if result.success {
            info!(
                tool = %call.name,
                duration_ms = result.duration_ms.unwrap_or(0),
                "Tool executed successfully"
            );
        } else {
            warn!(
                tool = %call.name,
                error = %result.error.as_deref().unwrap_or("unknown"),
                "Tool execution failed"
            );
        }

        Ok(result)
    }

    /// Create a default registry with all built-in tools
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(ShellTool::new()));
        registry.register(Arc::new(ReadFileTool::new()));
        registry.register(Arc::new(WriteFileTool::new()));
        registry.register(Arc::new(WebFetchTool::new()));
        registry.register(Arc::new(WebSearchTool::new()));
        registry
    }

    /// Create a default registry with web search configured
    #[allow(dead_code)]
    pub fn with_web_search_config(
        endpoint: &str,
        engine: &str,
        max_results: usize,
        fetch_content: bool,
    ) -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(ShellTool::new()));
        registry.register(Arc::new(ReadFileTool::new()));
        registry.register(Arc::new(WriteFileTool::new()));
        registry.register(Arc::new(WebFetchTool::new()));
        registry.register(Arc::new(WebSearchTool::with_config(
            endpoint.to_string(),
            engine.to_string(),
            max_results,
            fetch_content,
        )));
        registry
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_default_tools()
    }
}

// ── Built-in tools ─────────────────────────────────────────────────────────

/// Shell command execution tool (sandboxed)
pub struct ShellTool {
    definition: ToolDefinition,
    sandbox: Option<Sandbox>,
}

impl ShellTool {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn new_with_sandbox(sandbox: Sandbox) -> Self {
        Self {
            sandbox: Some(sandbox),
            ..Self::default()
        }
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "command".to_string(),
            JsonSchema::string("The shell command to execute"),
        );
        properties.insert(
            "timeout_secs".to_string(),
            JsonSchema {
                schema_type: "integer".to_string(),
                description: Some("Timeout in seconds (default: 30)".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "workdir".to_string(),
            JsonSchema::string("Working directory (default: current)"),
        );

        Self {
            definition: ToolDefinition {
                name: "shell_exec".to_string(),
                description: "Execute a shell command and return its output. Use for running scripts, compiling code, or any command-line operation. Runs in a sandboxed environment.".to_string(),
                parameters: JsonSchema::object(
                    properties,
                    vec!["command".to_string()],
                ),
                requires_approval: true,
                category: ToolCategory::Shell,
            },
            sandbox: None,
        }
    }
}

#[async_trait::async_trait]
impl ToolImpl for ShellTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments(
                    "shell_exec".to_string(),
                    "missing 'command' argument".to_string(),
                )
            })?;

        let timeout_secs = args
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        // Use sandbox workdir if available, otherwise use provided workdir
        let workdir = if let Some(sandbox) = &self.sandbox {
            sandbox.workdir().to_string_lossy().to_string()
        } else {
            args.get("workdir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                })
        };

        // Execute the command (sandboxed if sandbox is configured)
        let result = run_shell_command(command, timeout_secs, Some(workdir)).await?;

        Ok(result)
    }
}

/// Read a file from the filesystem
pub struct ReadFileTool {
    definition: ToolDefinition,
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            JsonSchema::string("Absolute path to the file to read"),
        );
        properties.insert(
            "max_bytes".to_string(),
            JsonSchema {
                schema_type: "integer".to_string(),
                description: Some("Maximum bytes to read (default: 65536)".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
            },
        );

        Self {
            definition: ToolDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file from the filesystem. Returns the file content as text.".to_string(),
                parameters: JsonSchema::object(
                    properties,
                    vec!["path".to_string()],
                ),
                requires_approval: false,
                category: ToolCategory::FileSystem,
            },
        }
    }
}

#[async_trait::async_trait]
impl ToolImpl for ReadFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments(
                "read_file".to_string(),
                "missing 'path' argument".to_string(),
            )
        })?;

        let max_bytes = args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(65536) as usize;

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ToolError::ExecutionFailed("read_file".to_string(), format!("Cannot read file: {}", e))
        })?;

        let truncated = if content.len() > max_bytes {
            format!(
                "{}...\n[truncated at {} bytes]",
                &content[..max_bytes],
                max_bytes
            )
        } else {
            content
        };

        Ok(ToolResult {
            tool_name: "read_file".to_string(),
            success: true,
            output: truncated,
            error: None,
            exit_code: None,
            duration_ms: None,
        })
    }
}

/// Write a file to the filesystem
pub struct WriteFileTool {
    definition: ToolDefinition,
}

impl WriteFileTool {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for WriteFileTool {
    fn default() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            JsonSchema::string("Absolute path to the file to write"),
        );
        properties.insert(
            "content".to_string(),
            JsonSchema::string("The content to write to the file"),
        );
        properties.insert(
            "append".to_string(),
            JsonSchema {
                schema_type: "boolean".to_string(),
                description: Some(
                    "If true, append instead of overwrite (default: false)".to_string(),
                ),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
            },
        );

        Self {
            definition: ToolDefinition {
                name: "write_file".to_string(),
                description: "Write content to a file. Creates parent directories if they don't exist. Can append to existing files.".to_string(),
                parameters: JsonSchema::object(
                    properties,
                    vec!["path".to_string(), "content".to_string()],
                ),
                requires_approval: true,
                category: ToolCategory::FileSystem,
            },
        }
    }
}

#[async_trait::async_trait]
impl ToolImpl for WriteFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments(
                "write_file".to_string(),
                "missing 'path' argument".to_string(),
            )
        })?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments(
                    "write_file".to_string(),
                    "missing 'content' argument".to_string(),
                )
            })?;

        let append = args
            .get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Create parent directories
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionFailed(
                    "write_file".to_string(),
                    format!("Cannot create directories: {}", e),
                )
            })?;
        }

        if append {
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(
                        "write_file".to_string(),
                        format!("Cannot open file for append: {}", e),
                    )
                })?;
            tokio::io::AsyncWriteExt::write_all(&mut file, content.as_bytes())
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(
                        "write_file".to_string(),
                        format!("Cannot write to file: {}", e),
                    )
                })?;
        } else {
            tokio::fs::write(path, content).await.map_err(|e| {
                ToolError::ExecutionFailed(
                    "write_file".to_string(),
                    format!("Cannot write file: {}", e),
                )
            })?;
        }

        Ok(ToolResult {
            tool_name: "write_file".to_string(),
            success: true,
            output: format!("Successfully wrote {} bytes to {}", content.len(), path),
            error: None,
            exit_code: None,
            duration_ms: None,
        })
    }
}

/// Web fetch tool — fetches a URL and returns the content
pub struct WebFetchTool {
    definition: ToolDefinition,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        let mut properties = HashMap::new();
        properties.insert("url".to_string(), JsonSchema::string("The URL to fetch"));
        properties.insert(
            "max_bytes".to_string(),
            JsonSchema {
                schema_type: "integer".to_string(),
                description: Some("Maximum bytes to read (default: 131072)".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
            },
        );

        Self {
            definition: ToolDefinition {
                name: "web_fetch".to_string(),
                description: "Fetch a URL and return its content as text. Use for reading web pages, APIs, or documentation.".to_string(),
                parameters: JsonSchema::object(
                    properties,
                    vec!["url".to_string()],
                ),
                requires_approval: false,
                category: ToolCategory::Network,
            },
        }
    }
}

#[async_trait::async_trait]
impl ToolImpl for WebFetchTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult> {
        let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments(
                "web_fetch".to_string(),
                "missing 'url' argument".to_string(),
            )
        })?;

        let max_bytes = args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(131072) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("RavenClaws/0.9.2")
            .build()
            .map_err(|e| {
                ToolError::ExecutionFailed("web_fetch".to_string(), format!("HTTP client: {}", e))
            })?;

        let response = client.get(url).send().await.map_err(|e| {
            ToolError::ExecutionFailed("web_fetch".to_string(), format!("Request failed: {}", e))
        })?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let body = response.text().await.map_err(|e| {
            ToolError::ExecutionFailed(
                "web_fetch".to_string(),
                format!("Failed to read response body: {}", e),
            )
        })?;

        let truncated = if body.len() > max_bytes {
            format!(
                "{}...\n[truncated at {} bytes]",
                &body[..max_bytes],
                max_bytes
            )
        } else {
            body
        };

        Ok(ToolResult {
            tool_name: "web_fetch".to_string(),
            success: status.is_success(),
            output: format!(
                "Status: {}\nContent-Type: {}\n\n{}",
                status.as_u16(),
                content_type,
                truncated
            ),
            error: if status.is_success() {
                None
            } else {
                Some(format!("HTTP {}", status.as_u16()))
            },
            exit_code: Some(status.as_u16() as i32),
            duration_ms: None,
        })
    }
}

/// Web search tool — searches the web using a configurable search API
pub struct WebSearchTool {
    definition: ToolDefinition,
    search_endpoint: String,
    search_engine: String,
    max_results: usize,
    fetch_content: bool,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(
        endpoint: String,
        engine: String,
        max_results: usize,
        fetch_content: bool,
    ) -> Self {
        let mut properties = HashMap::new();
        properties.insert("query".to_string(), JsonSchema::string("The search query"));
        properties.insert(
            "max_results".to_string(),
            JsonSchema {
                schema_type: "integer".to_string(),
                description: Some(
                    "Maximum number of search results to return (default: 5)".to_string(),
                ),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "fetch_content".to_string(),
            JsonSchema {
                schema_type: "boolean".to_string(),
                description: Some(
                    "Whether to fetch and extract content from each result (default: true)"
                        .to_string(),
                ),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
            },
        );

        Self {
            definition: ToolDefinition {
                name: "web_search".to_string(),
                description: "Search the web for information. Returns a list of results with titles, URLs, and snippets. Can optionally fetch and extract readable content from each result.".to_string(),
                parameters: JsonSchema::object(
                    properties,
                    vec!["query".to_string()],
                ),
                requires_approval: false,
                category: ToolCategory::WebSearch,
            },
            search_endpoint: endpoint,
            search_engine: engine,
            max_results,
            fetch_content,
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::with_config(
            "https://searx.be".to_string(),
            "duckduckgo".to_string(),
            5,
            true,
        )
    }
}

impl WebSearchTool {
    /// Search via SearXNG API (self-hosted, privacy-respecting)
    async fn search_searxng(
        &self,
        query: &str,
        max_results: usize,
    ) -> ToolResultValue<Vec<SearchResult>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("RavenClaws/0.9.2")
            .build()
            .map_err(|e| {
                ToolError::ExecutionFailed("web_search".to_string(), format!("HTTP client: {}", e))
            })?;

        let url = format!(
            "{}/search?q={}&format=json&language=en&pageno=1",
            self.search_endpoint.trim_end_matches('/'),
            urlencoding(query)
        );

        let response = client.get(&url).send().await.map_err(|e| {
            ToolError::ExecutionFailed(
                "web_search".to_string(),
                format!("Search request failed: {}", e),
            )
        })?;

        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(
                "web_search".to_string(),
                format!("Search API returned HTTP {}", response.status().as_u16()),
            ));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            ToolError::ExecutionFailed(
                "web_search".to_string(),
                format!("Failed to parse search results: {}", e),
            )
        })?;

        let results = body["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .take(max_results)
                    .filter_map(|r| {
                        let title = r["title"].as_str().unwrap_or("").to_string();
                        let url = r["url"].as_str().unwrap_or("").to_string();
                        let snippet = r["content"].as_str().unwrap_or("").to_string();
                        if title.is_empty() && url.is_empty() {
                            None
                        } else {
                            Some(SearchResult {
                                title,
                                url,
                                snippet,
                            })
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// Search via DuckDuckGo HTML (no API key needed)
    async fn search_duckduckgo(
        &self,
        query: &str,
        max_results: usize,
    ) -> ToolResultValue<Vec<SearchResult>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (compatible; RavenClaws/0.9.2)")
            .build()
            .map_err(|e| {
                ToolError::ExecutionFailed("web_search".to_string(), format!("HTTP client: {}", e))
            })?;

        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding(query));

        let response = client.get(&url).send().await.map_err(|e| {
            ToolError::ExecutionFailed(
                "web_search".to_string(),
                format!("Search request failed: {}", e),
            )
        })?;

        let body = response.text().await.map_err(|e| {
            ToolError::ExecutionFailed(
                "web_search".to_string(),
                format!("Failed to read search results: {}", e),
            )
        })?;

        // Parse DuckDuckGo HTML results — extract from result links
        let mut results = Vec::new();
        let mut pos = 0;
        let result_class = "result__a";

        while results.len() < max_results {
            // Find the next result link
            let link_start = match body[pos..].find(result_class) {
                Some(i) => pos + i,
                None => break,
            };

            // Find the <a> tag within this result
            let a_start = match body[link_start..].find("<a ") {
                Some(i) => link_start + i,
                None => break,
            };
            let a_end = match body[a_start..].find("</a>") {
                Some(i) => a_start + i,
                None => break,
            };

            let a_tag = &body[a_start..a_end];

            // Extract URL from href
            let url = extract_href(a_tag).unwrap_or_default();
            // Extract title from tag content (after last >)
            let title = a_tag.rsplit('>').next().unwrap_or("").trim().to_string();

            // Find snippet (next .result__snippet)
            let snippet_start = match body[a_end..].find("result__snippet") {
                Some(i) => a_end + i,
                None => {
                    results.push(SearchResult {
                        title,
                        url,
                        snippet: String::new(),
                    });
                    pos = a_end + 1;
                    continue;
                }
            };
            let snippet_close = match body[snippet_start..].find("</a>") {
                Some(i) => snippet_start + i,
                None => {
                    results.push(SearchResult {
                        title,
                        url,
                        snippet: String::new(),
                    });
                    pos = a_end + 1;
                    continue;
                }
            };
            let snippet_html = &body[snippet_start..snippet_close];
            let snippet = strip_html_tags(snippet_html).trim().to_string();

            if !url.is_empty() || !title.is_empty() {
                results.push(SearchResult {
                    title,
                    url,
                    snippet,
                });
            }

            pos = a_end + 1;
        }

        Ok(results)
    }
}

/// A single search result
#[allow(dead_code)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

#[async_trait::async_trait]
impl ToolImpl for WebSearchTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments(
                "web_search".to_string(),
                "missing 'query' argument".to_string(),
            )
        })?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.max_results as u64) as usize;

        let fetch_content = args
            .get("fetch_content")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.fetch_content);

        // Perform the search
        let results = match self.search_engine.as_str() {
            "searxng" => self.search_searxng(query, max_results).await?,
            _ => self.search_duckduckgo(query, max_results).await?,
        };

        if results.is_empty() {
            return Ok(ToolResult {
                tool_name: "web_search".to_string(),
                success: true,
                output: "No search results found.".to_string(),
                error: None,
                exit_code: None,
                duration_ms: None,
            });
        }

        // Optionally fetch content from each result
        let mut output = String::new();
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!(
                "[{}] **{}**\n    URL: {}\n    Snippet: {}\n",
                i + 1,
                result.title,
                result.url,
                result.snippet
            ));

            if fetch_content && !result.url.is_empty() {
                match fetch_and_extract_content(&result.url, 8192).await {
                    Ok(content) => {
                        output.push_str(&format!("    Content: {}\n", content));
                    }
                    Err(e) => {
                        output.push_str(&format!("    Content: (unavailable: {})\n", e));
                    }
                }
            }
        }

        Ok(ToolResult {
            tool_name: "web_search".to_string(),
            success: true,
            output,
            error: None,
            exit_code: None,
            duration_ms: None,
        })
    }
}

// ── HTML extraction helpers ────────────────────────────────────────────────

/// Extract readable content from a URL (HTML-to-text)
async fn fetch_and_extract_content(url: &str, max_bytes: usize) -> ToolResultValue<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; RavenClaws/0.9.2)")
        .build()
        .map_err(|e| {
            ToolError::ExecutionFailed("web_fetch".to_string(), format!("HTTP client: {}", e))
        })?;

    let response = client.get(url).send().await.map_err(|e| {
        ToolError::ExecutionFailed("web_fetch".to_string(), format!("Request failed: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(ToolError::ExecutionFailed(
            "web_fetch".to_string(),
            format!("HTTP {}", response.status().as_u16()),
        ));
    }

    let body = response.text().await.map_err(|e| {
        ToolError::ExecutionFailed(
            "web_fetch".to_string(),
            format!("Failed to read response: {}", e),
        )
    })?;

    Ok(html_to_text(&body, max_bytes))
}

/// Convert HTML to readable text by stripping tags and extracting meaningful content
fn html_to_text(html: &str, max_chars: usize) -> String {
    let mut text = String::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut in_title = false;
    let mut title_text = String::new();
    let mut last_char_was_space = true;

    while i < len {
        if in_script {
            // Look for </script>
            if i + 8 < len && bytes[i..i + 9].eq_ignore_ascii_case(b"</script>") {
                in_script = false;
                i += 9;
                continue;
            }
            i += 1;
            continue;
        }

        if in_style {
            // Look for </style>
            if i + 7 < len && bytes[i..i + 8].eq_ignore_ascii_case(b"</style>") {
                in_style = false;
                i += 8;
                continue;
            }
            i += 1;
            continue;
        }

        if in_title {
            // Look for </title>
            if i + 7 < len && bytes[i..i + 8].eq_ignore_ascii_case(b"</title>") {
                in_title = false;
                i += 8;
                continue;
            }
            title_text.push(bytes[i] as char);
            i += 1;
            continue;
        }

        if in_tag {
            if bytes[i] == b'>' {
                in_tag = false;
                // Check for <br> and <p> tags — add newline
                if i >= 2 {
                    let tag_start = (0..i).rev().find(|&j| bytes[j] == b'<').unwrap_or(0);
                    let tag_content = &html[tag_start..i].to_lowercase();
                    if (tag_content.starts_with("<br")
                        || tag_content.starts_with("<p")
                        || tag_content.starts_with("<tr")
                        || tag_content.starts_with("<div")
                        || tag_content.starts_with("<li")
                        || tag_content.starts_with("<h1")
                        || tag_content.starts_with("<h2")
                        || tag_content.starts_with("<h3")
                        || tag_content.starts_with("<h4")
                        || tag_content.starts_with("<h5")
                        || tag_content.starts_with("<h6"))
                        && !last_char_was_space
                    {
                        text.push('\n');
                        last_char_was_space = true;
                    }
                }
            } else {
                // Check for <script, <style, <title
                if bytes[i] == b's' || bytes[i] == b'S' {
                    if i + 5 < len && bytes[i..i + 6].eq_ignore_ascii_case(b"script") {
                        in_script = true;
                    } else if i + 4 < len && bytes[i..i + 5].eq_ignore_ascii_case(b"style") {
                        in_style = true;
                    } else if i + 4 < len && bytes[i..i + 5].eq_ignore_ascii_case(b"title") {
                        in_title = true;
                    }
                }
            }
            i += 1;
            continue;
        }

        if bytes[i] == b'<' {
            in_tag = true;
            i += 1;
            continue;
        }

        // Decode common HTML entities
        if bytes[i] == b'&' {
            let remaining = len - i;
            let entity = if remaining > 5 && &html[i..i + 6] == "&nbsp;" {
                i += 6;
                " "
            } else if remaining > 3 && &html[i..i + 4] == "&lt;" {
                i += 4;
                "<"
            } else if remaining > 3 && &html[i..i + 4] == "&gt;" {
                i += 4;
                ">"
            } else if remaining > 4 && &html[i..i + 5] == "&amp;" {
                i += 5;
                "&"
            } else if remaining > 5 && &html[i..i + 6] == "&quot;" {
                i += 6;
                "\""
            } else if remaining > 3 && &html[i..i + 4] == "&#39;" {
                i += 4;
                "'"
            } else {
                i += 1;
                continue;
            };

            if text.len() >= max_chars {
                break;
            }
            text.push_str(entity);
            last_char_was_space = entity == " ";
            continue;
        }

        // Normalize whitespace
        if bytes[i].is_ascii_whitespace() {
            if !last_char_was_space {
                text.push(' ');
                last_char_was_space = true;
            }
            i += 1;
            continue;
        }

        if text.len() >= max_chars {
            break;
        }
        text.push(bytes[i] as char);
        last_char_was_space = false;
        i += 1;
    }

    // Prepend title if found
    let title_text = title_text.trim();
    let text = text.trim();

    if !title_text.is_empty() {
        format!("Title: {}\n\n{}", title_text, text)
    } else {
        text.to_string()
    }
}

/// Strip HTML tags from a string (for snippet extraction)
fn strip_html_tags(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => {
                if !in_tag {
                    output.push(c);
                }
            }
        }
    }
    // Decode common entities
    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// Extract href value from an <a> tag
fn extract_href(a_tag: &str) -> Option<String> {
    let href_start = a_tag.find("href=\"")?;
    let value_start = href_start + 6;
    let value_end = a_tag[value_start..].find('"')?;
    let href = &a_tag[value_start..value_start + value_end];

    // DuckDuckGo redirect URLs
    if href.starts_with("//") {
        return Some(format!("https:{}", href));
    }
    if href.starts_with("/") {
        return None; // Relative URLs, skip
    }

    Some(href.to_string())
}

/// URL-encode a string for use in query parameters
fn urlencoding(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

// ── Helper functions ───────────────────────────────────────────────────────

/// Run a shell command with timeout
async fn run_shell_command(
    command: &str,
    timeout_secs: u64,
    workdir: Option<String>,
) -> ToolResultValue<ToolResult> {
    use tokio::process::Command;

    let shell = if cfg!(target_os = "windows") {
        "cmd.exe"
    } else {
        "sh"
    };
    let flag = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let mut cmd = Command::new(shell);
    cmd.arg(flag).arg(command);

    if let Some(dir) = &workdir {
        cmd.current_dir(dir);
    }

    let output = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| {
            ToolError::ExecutionFailed(
                "shell_exec".to_string(),
                format!("Command timed out after {} seconds", timeout_secs),
            )
        })?
        .map_err(|e| {
            ToolError::ExecutionFailed(
                "shell_exec".to_string(),
                format!("Failed to execute: {}", e),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let mut output_text = String::new();
    if !stdout.is_empty() {
        output_text.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !output_text.is_empty() {
            output_text.push_str("\n--- stderr ---\n");
        }
        output_text.push_str(&stderr);
    }

    // Truncate very long output
    const MAX_OUTPUT: usize = 65536;
    if output_text.len() > MAX_OUTPUT {
        output_text = format!(
            "{}...\n[truncated at {} bytes]",
            &output_text[..MAX_OUTPUT],
            MAX_OUTPUT
        );
    }

    Ok(ToolResult {
        tool_name: "shell_exec".to_string(),
        success: exit_code == 0,
        output: output_text,
        error: if exit_code != 0 {
            Some(format!("Exit code: {}", exit_code))
        } else {
            None
        },
        exit_code: Some(exit_code),
        duration_ms: None,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_tool_registry_register() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(ShellTool::new()));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.has("shell_exec"));
    }

    #[test]
    fn test_tool_registry_default_tools() {
        let registry = ToolRegistry::with_default_tools();
        assert_eq!(registry.len(), 5);
        assert!(registry.has("shell_exec"));
        assert!(registry.has("read_file"));
        assert!(registry.has("write_file"));
        assert!(registry.has("web_fetch"));
        assert!(registry.has("web_search"));
    }

    #[test]
    fn test_tool_definitions() {
        let registry = ToolRegistry::with_default_tools();
        let defs = registry.definitions();
        assert_eq!(defs.len(), 5);

        let shell_def = defs.iter().find(|d| d.name == "shell_exec").unwrap();
        assert!(shell_def.description.contains("shell command"));
        assert!(shell_def.requires_approval);
        assert_eq!(shell_def.category, ToolCategory::Shell);
    }

    #[test]
    fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        let result = registry.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_shell_tool_definition() {
        let tool = ShellTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "shell_exec");
        assert!(def.requires_approval);
    }

    #[test]
    fn test_read_file_tool_definition() {
        let tool = ReadFileTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "read_file");
        assert!(!def.requires_approval);
    }

    #[test]
    fn test_write_file_tool_definition() {
        let tool = WriteFileTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "write_file");
        assert!(def.requires_approval);
    }

    #[test]
    fn test_web_fetch_tool_definition() {
        let tool = WebFetchTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "web_fetch");
        assert!(!def.requires_approval);
    }

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            name: "shell_exec".to_string(),
            arguments: serde_json::json!({"command": "echo hello"}),
            id: Some("call_123".to_string()),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("shell_exec"));
        assert!(json.contains("echo hello"));
        assert!(json.contains("call_123"));
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            tool_name: "shell_exec".to_string(),
            success: true,
            output: "hello\n".to_string(),
            error: None,
            exit_code: Some(0),
            duration_ms: Some(42),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("shell_exec"));
        assert!(json.contains("hello"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult {
            tool_name: "shell_exec".to_string(),
            success: false,
            output: String::new(),
            error: Some("Exit code: 1".to_string()),
            exit_code: Some(1),
            duration_ms: Some(10),
        };

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn test_json_schema_string() {
        let schema = JsonSchema::string("A test string");
        assert_eq!(schema.schema_type, "string");
        assert_eq!(schema.description.unwrap(), "A test string");
    }

    #[test]
    fn test_json_schema_object() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), JsonSchema::string("The name"));
        let schema = JsonSchema::object(props, vec!["name".to_string()]);
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.unwrap().contains_key("name"));
    }

    #[test]
    fn test_tool_error_not_found() {
        let err = ToolError::NotFound("test_tool".to_string());
        assert_eq!(format!("{}", err), "Tool 'test_tool' not found");
    }

    #[test]
    fn test_tool_error_execution_failed() {
        let err = ToolError::ExecutionFailed("test".to_string(), "oops".to_string());
        assert_eq!(format!("{}", err), "Tool 'test' execution failed: oops");
    }

    #[test]
    fn test_tool_error_invalid_arguments() {
        let err = ToolError::InvalidArguments("test".to_string(), "bad arg".to_string());
        assert_eq!(
            format!("{}", err),
            "Invalid arguments for tool 'test': bad arg"
        );
    }

    #[test]
    fn test_tool_error_policy_denied() {
        let err = ToolError::PolicyDenied("not allowed".to_string());
        assert_eq!(format!("{}", err), "Policy denied: not allowed");
    }

    #[test]
    fn test_tool_error_sandbox_violation() {
        let err = ToolError::SandboxViolation("escape attempt".to_string());
        assert_eq!(format!("{}", err), "Sandbox violation: escape attempt");
    }

    #[test]
    fn test_tool_category_default() {
        let cat = ToolCategory::default();
        assert_eq!(cat, ToolCategory::General);
    }

    #[test]
    fn test_tool_category_serialization() {
        let cat = ToolCategory::Shell;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"Shell\"");
    }

    #[test]
    fn test_tool_definition_requires_approval_default() {
        let def = ToolDefinition {
            name: "test".to_string(),
            description: "test".to_string(),
            parameters: JsonSchema::string("test"),
            requires_approval: false,
            category: ToolCategory::General,
        };
        assert!(!def.requires_approval);
    }

    #[tokio::test]
    async fn test_shell_exec_success() {
        let tool = ShellTool::new();
        let args = serde_json::json!({"command": "echo hello"});
        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_shell_exec_failure() {
        let tool = ShellTool::new();
        let args = serde_json::json!({"command": "exit 42"});
        let result = tool.execute(args).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_shell_exec_missing_command() {
        let tool = ShellTool::new();
        let args = serde_json::json!({});
        let err = tool.execute(args).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_, _)));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let tool = ReadFileTool::new();
        let args = serde_json::json!({"path": "/tmp/nonexistent_file_ravenclaws_test"});
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ToolError::ExecutionFailed(_, _)
        ));
    }

    #[tokio::test]
    async fn test_read_file_missing_path() {
        let tool = ReadFileTool::new();
        let args = serde_json::json!({});
        let err = tool.execute(args).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_, _)));
    }

    #[tokio::test]
    async fn test_write_file_missing_args() {
        let tool = WriteFileTool::new();
        let args = serde_json::json!({});
        let err = tool.execute(args).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_, _)));
    }

    #[tokio::test]
    async fn test_web_fetch_missing_url() {
        let tool = WebFetchTool::new();
        let args = serde_json::json!({});
        let err = tool.execute(args).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_, _)));
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let dir = std::env::temp_dir().join(format!("ravenclaws_test_{}", std::process::id()));
        let path = dir.join("test_write.txt");
        let path_str = path.to_string_lossy().to_string();

        // Write
        let write_tool = WriteFileTool::new();
        let args = serde_json::json!({"path": path_str, "content": "Hello, RavenClaws!"});
        let result = write_tool.execute(args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("18 bytes"));

        // Read back
        let read_tool = ReadFileTool::new();
        let args = serde_json::json!({"path": path_str});
        let result = read_tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output.trim(), "Hello, RavenClaws!");

        // Cleanup
        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_dir(dir).await;
    }

    #[tokio::test]
    async fn test_write_file_append() {
        let dir = std::env::temp_dir().join(format!("ravenclaws_test_{}", std::process::id()));
        let path = dir.join("test_append.txt");
        let path_str = path.to_string_lossy().to_string();

        // Write initial
        let write_tool = WriteFileTool::new();
        let args = serde_json::json!({"path": path_str, "content": "line1\n"});
        write_tool.execute(args).await.unwrap();

        // Append
        let args = serde_json::json!({"path": path_str, "content": "line2\n", "append": true});
        let result = write_tool.execute(args).await.unwrap();
        assert!(result.success);

        // Read back
        let read_tool = ReadFileTool::new();
        let args = serde_json::json!({"path": path_str});
        let result = read_tool.execute(args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("line1"));
        assert!(result.output.contains("line2"));

        // Cleanup
        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_dir(dir).await;
    }

    #[tokio::test]
    async fn test_tool_registry_execute() {
        let registry = ToolRegistry::with_default_tools();
        let call = ToolCall {
            name: "shell_exec".to_string(),
            arguments: serde_json::json!({"command": "echo hello"}),
            id: None,
        };
        let result = registry.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_tool_registry_execute_not_found() {
        let registry = ToolRegistry::new();
        let call = ToolCall {
            name: "nonexistent".to_string(),
            arguments: serde_json::json!({}),
            id: None,
        };
        let err = registry.execute(call).await.unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    // ── Web search tool tests ──────────────────────────────────────────────

    #[test]
    fn test_web_search_tool_definition() {
        let tool = WebSearchTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "web_search");
        assert!(!def.requires_approval);
        assert_eq!(def.category, ToolCategory::WebSearch);
        assert!(def.description.contains("Search the web"));
    }

    #[test]
    fn test_web_search_tool_with_config() {
        let tool = WebSearchTool::with_config(
            "http://localhost:8888".to_string(),
            "searxng".to_string(),
            10,
            false,
        );
        let def = tool.definition();
        assert_eq!(def.name, "web_search");
        assert_eq!(tool.search_endpoint, "http://localhost:8888");
        assert_eq!(tool.search_engine, "searxng");
        assert_eq!(tool.max_results, 10);
        assert!(!tool.fetch_content);
    }

    #[tokio::test]
    async fn test_web_search_missing_query() {
        let tool = WebSearchTool::new();
        let args = serde_json::json!({});
        let err = tool.execute(args).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_, _)));
    }

    #[test]
    fn test_web_search_tool_registry() {
        let registry = ToolRegistry::with_default_tools();
        assert!(registry.has("web_search"));
        let defs = registry.definitions();
        let search_def = defs.iter().find(|d| d.name == "web_search").unwrap();
        assert_eq!(search_def.category, ToolCategory::WebSearch);
    }

    #[test]
    fn test_web_search_tool_with_config_registry() {
        let registry =
            ToolRegistry::with_web_search_config("http://localhost:8888", "searxng", 10, false);
        assert!(registry.has("web_search"));
        assert!(registry.has("shell_exec"));
        assert!(registry.has("read_file"));
        assert!(registry.has("write_file"));
        assert!(registry.has("web_fetch"));
        assert_eq!(registry.len(), 5);
    }

    // ── HTML extraction tests ──────────────────────────────────────────────

    #[test]
    fn test_html_to_text_strips_tags() {
        let html = "<html><body><p>Hello, world!</p></body></html>";
        let text = html_to_text(html, 1000);
        assert!(text.contains("Hello, world!"));
        assert!(!text.contains("<p>"));
        assert!(!text.contains("</p>"));
    }

    #[test]
    fn test_html_to_text_extracts_title() {
        let html = "<html><head><title>Test Page</title></head><body><p>Content</p></body></html>";
        let text = html_to_text(html, 1000);
        assert!(text.contains("Test Page"));
        assert!(text.contains("Content"));
    }

    #[test]
    fn test_html_to_text_strips_script_and_style() {
        let html = "<html><head><script>alert('xss');</script><style>.cls{}</style></head><body><p>Visible</p></body></html>";
        let text = html_to_text(html, 1000);
        assert!(text.contains("Visible"));
        assert!(!text.contains("alert"));
        assert!(!text.contains(".cls"));
    }

    #[test]
    fn test_html_to_text_handles_entities() {
        let html = "<p>foo &amp; bar &lt; baz &gt; qux</p>";
        let text = html_to_text(html, 1000);
        assert!(text.contains("foo & bar < baz > qux") || text.contains("foo & bar"));
    }

    #[test]
    fn test_html_to_text_respects_max_chars() {
        let html = "<p>Hello World This Is A Test</p>";
        let text = html_to_text(html, 5);
        assert!(text.len() <= 5);
    }

    #[test]
    fn test_html_to_text_empty_input() {
        assert_eq!(html_to_text("", 1000), "");
    }

    #[test]
    fn test_html_to_text_no_html() {
        let text = html_to_text("Just plain text", 1000);
        assert_eq!(text, "Just plain text");
    }

    #[test]
    fn test_strip_html_tags_basic() {
        let result = strip_html_tags("<b>bold</b> and <i>italic</i>");
        assert_eq!(result, "bold and italic");
    }

    #[test]
    fn test_strip_html_tags_with_entities() {
        let result = strip_html_tags("foo &amp; bar &lt; baz");
        assert_eq!(result, "foo & bar < baz");
    }

    #[test]
    fn test_extract_href_basic() {
        let result = extract_href(r#"<a href="https://example.com">link</a>"#);
        assert_eq!(result, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_extract_href_protocol_relative() {
        let result = extract_href(r#"<a href="//example.com/path">link</a>"#);
        assert_eq!(result, Some("https://example.com/path".to_string()));
    }

    #[test]
    fn test_extract_href_relative() {
        let result = extract_href(r#"<a href="/relative/path">link</a>"#);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_href_no_match() {
        let result = extract_href("<span>no link here</span>");
        assert_eq!(result, None);
    }

    #[test]
    fn test_urlencoding_basic() {
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert_eq!(urlencoding("foo/bar"), "foo%2Fbar");
        assert_eq!(urlencoding("simple"), "simple");
    }

    #[test]
    fn test_fetch_and_extract_content_invalid_url() {
        let result = tokio_test::block_on(fetch_and_extract_content("http://0.0.0.0:1", 1000));
        assert!(result.is_err());
    }
}
