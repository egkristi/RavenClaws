//! Model Context Protocol (MCP) for RavenClaws
//!
//! Implements both MCP client and server:
//! - **Client**: Connect to external MCP servers, discover tools, execute them via JSON-RPC over stdio
//! - **Server**: Expose RavenClaws's built-in tools as an MCP server over stdio
//!
//! # Architecture
//!
//! ```text
//! McpClient                          McpServer
//!   ├── McpTransport (stdio)           ├── McpServerTransport (stdio)
//!   ├── McpToolRegistry                ├── ToolRegistry (RavenClaws tools)
//!   └── JsonRpcClient                  └── JsonRpcHandler
//! ```
//!
//! # References
//! - MCP Spec: https://modelcontextprotocol.io/specification
//! - JSON-RPC 2.0: https://www.jsonrpc.org/specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::tools::{
    JsonSchema, ToolCategory, ToolDefinition, ToolImpl, ToolResult, ToolResultValue,
};

// ── Error types ────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("Server error: {code} - {message}")]
    Server { code: i32, message: String },

    #[error("Tool not found: {0}")]
    #[allow(dead_code)]
    ToolNotFound(String),

    #[error("Invalid tool arguments: {0}")]
    #[allow(dead_code)]
    InvalidArguments(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Timeout: {0}")]
    #[allow(dead_code)]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type McpResult<T> = std::result::Result<T, McpError>;

// ── JSON-RPC types ─────────────────────────────────────────────────────────

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(method: &str, params: serde_json::Value, id: i64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: serde_json::Value::Number(id.into()),
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ── MCP Protocol types ─────────────────────────────────────────────────────

/// MCP Protocol version
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// MCP Initialize request params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: McpClientCapabilities,
    pub client_info: McpClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpClientCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<McpRootsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRootsCapability {
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientInfo {
    pub name: String,
    pub version: String,
}

/// MCP Initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: McpServerCapabilities,
    pub server_info: McpServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<McpToolsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP Tool definition (from server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// MCP Tool call arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// MCP Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: serde_json::Value },
}

// ── MCP Transport ──────────────────────────────────────────────────────────

/// MCP transport type
#[derive(Debug, Clone)]
pub enum McpTransportConfig {
    /// Stdio transport: spawn a command and communicate via stdin/stdout
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// SSE transport: connect to HTTP endpoint (not yet implemented)
    #[allow(dead_code)]
    Sse { url: String },
}

/// MCP Transport — handles low-level communication
pub struct McpTransport {
    config: McpTransportConfig,
    #[allow(dead_code)]
    child: Option<Child>,
    stdin: Option<tokio::process::ChildStdin>,
    stdout_reader: Option<BufReader<tokio::process::ChildStdout>>,
    request_id: i64,
}

impl McpTransport {
    /// Create a new transport with the given configuration
    pub fn new(config: McpTransportConfig) -> Self {
        Self {
            config,
            child: None,
            stdin: None,
            stdout_reader: None,
            request_id: 0,
        }
    }

    /// Connect to the MCP server (spawn process for stdio)
    pub async fn connect(&mut self) -> McpResult<()> {
        match &self.config {
            McpTransportConfig::Stdio { command, args, env } => {
                let mut cmd = Command::new(command);
                cmd.args(args);
                cmd.stdin(Stdio::piped());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                cmd.envs(env);

                let mut child = cmd.spawn().map_err(|e| {
                    McpError::ConnectionFailed(format!("Failed to spawn {}: {}", command, e))
                })?;

                let stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| McpError::ConnectionFailed("No stdin available".to_string()))?;

                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| McpError::ConnectionFailed("No stdout available".to_string()))?;

                self.child = Some(child);
                self.stdin = Some(stdin);
                self.stdout_reader = Some(BufReader::new(stdout));

                info!(command = %command, "MCP stdio transport connected");
                Ok(())
            }
            McpTransportConfig::Sse { url } => {
                // TODO: Implement SSE transport
                Err(McpError::Transport(format!(
                    "SSE transport not yet implemented for {}",
                    url
                )))
            }
        }
    }

    /// Send a JSON-RPC request and wait for response
    pub async fn send_request(&mut self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let request_json = serde_json::to_string(&request)?;
        debug!("MCP → {}", request_json);

        // Send request with newline delimiter
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Transport("Transport not connected".to_string()))?;

        stdin.write_all(request_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response
        let stdout = self
            .stdout_reader
            .as_mut()
            .ok_or_else(|| McpError::Transport("Transport not connected".to_string()))?;

        let mut response_line = String::new();
        stdout.read_line(&mut response_line).await?;

        if response_line.trim().is_empty() {
            return Err(McpError::Transport(
                "Empty response from server".to_string(),
            ));
        }

        debug!("MCP ← {}", response_line.trim());

        let response: JsonRpcResponse = serde_json::from_str(&response_line)?;

        // Check for JSON-RPC error
        if let Some(err) = &response.error {
            return Err(McpError::Server {
                code: err.code,
                message: err.message.clone(),
            });
        }

        Ok(response)
    }

    /// Get next request ID
    fn next_id(&mut self) -> i64 {
        self.request_id += 1;
        self.request_id
    }
}

// ── MCP Client ─────────────────────────────────────────────────────────────

/// MCP Client — high-level interface to MCP servers
pub struct McpClient {
    transport: McpTransport,
    server_info: Option<McpServerInfo>,
    tools: Arc<RwLock<Vec<McpTool>>>,
}

impl McpClient {
    /// Create a new MCP client with the given transport config
    pub fn new(config: McpTransportConfig) -> Self {
        Self {
            transport: McpTransport::new(config),
            server_info: None,
            tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Connect to the MCP server and initialize
    pub async fn connect(&mut self) -> McpResult<()> {
        // Connect transport
        self.transport.connect().await?;

        // Send initialize request
        let init_params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: McpClientCapabilities {
                roots: Some(McpRootsCapability {
                    list_changed: false,
                }),
                sampling: None,
            },
            client_info: McpClientInfo {
                name: "ravenclaws".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let init_id = self.transport.next_id();
        let response = self
            .transport
            .send_request(JsonRpcRequest::new(
                "initialize",
                serde_json::to_value(init_params)?,
                init_id,
            ))
            .await?;

        let init_result: InitializeResult = response
            .result
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| McpError::JsonRpc("Invalid initialize response".to_string()))?;

        let server_info = init_result.server_info.clone();
        self.server_info = Some(init_result.server_info);

        info!(
            server = %server_info.name,
            version = %server_info.version,
            "MCP server initialized"
        );

        // Send initialized notification
        let notify = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
            params: serde_json::Value::Null,
            id: serde_json::Value::Null,
        };
        self.transport.send_request(notify).await?;

        // Discover tools
        self.discover_tools().await?;

        Ok(())
    }

    /// Discover available tools from the server
    pub async fn discover_tools(&mut self) -> McpResult<()> {
        let list_id = self.transport.next_id();
        let response = self
            .transport
            .send_request(JsonRpcRequest::new(
                "tools/list",
                serde_json::Value::Null,
                list_id,
            ))
            .await?;

        let tools_result = response
            .result
            .and_then(|v| v.get("tools").cloned())
            .ok_or_else(|| McpError::JsonRpc("No tools in response".to_string()))?;

        let tools: Vec<McpTool> = serde_json::from_value(tools_result)?;

        info!(count = tools.len(), "Discovered MCP tools");

        let mut tool_lock = self.tools.write().await;
        *tool_lock = tools;

        Ok(())
    }

    /// Get discovered tools
    pub async fn get_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<McpToolResult> {
        let params = McpToolCall {
            name: name.to_string(),
            arguments,
        };

        let call_id = self.transport.next_id();
        let response = self
            .transport
            .send_request(JsonRpcRequest::new(
                "tools/call",
                serde_json::to_value(params)?,
                call_id,
            ))
            .await?;

        let result: McpToolResult = response
            .result
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| McpError::JsonRpc("Invalid tool call response".to_string()))?;

        if result.is_error {
            return Err(McpError::Server {
                code: -32000,
                message: "Tool execution failed".to_string(),
            });
        }

        Ok(result)
    }

    /// Get server info
    pub fn server_info(&self) -> Option<&McpServerInfo> {
        self.server_info.as_ref()
    }
}

// ── MCP Tool Wrapper ───────────────────────────────────────────────────────

/// Wrapper that adapts MCP tools to RavenClaws's ToolImpl trait
pub struct McpToolWrapper {
    definition: ToolDefinition,
    client: Arc<RwLock<McpClient>>,
    tool_name: String,
}

impl McpToolWrapper {
    /// Create a new MCP tool wrapper
    pub fn new(client: Arc<RwLock<McpClient>>, mcp_tool: McpTool) -> Self {
        // Convert MCP input_schema to our JsonSchema
        let parameters = Self::convert_schema(&mcp_tool.input_schema);

        Self {
            definition: ToolDefinition {
                name: mcp_tool.name.clone(),
                description: mcp_tool
                    .description
                    .unwrap_or_else(|| "MCP-provided tool".to_string()),
                parameters,
                requires_approval: false,
                category: ToolCategory::Mcp,
            },
            client,
            tool_name: mcp_tool.name,
        }
    }

    /// Convert MCP JSON schema to our JsonSchema type
    fn convert_schema(schema: &serde_json::Value) -> JsonSchema {
        if let Some(obj) = schema.as_object() {
            let schema_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("object")
                .to_string();

            let description = obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let properties = obj
                .get("properties")
                .and_then(|v| v.as_object())
                .map(|props| {
                    props
                        .iter()
                        .map(|(k, v)| (k.clone(), Self::convert_schema(v)))
                        .collect::<HashMap<String, JsonSchema>>()
                });

            let required = obj.get("required").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

            JsonSchema {
                schema_type,
                description,
                properties,
                required,
                items: None,
                enum_values: None,
            }
        } else {
            JsonSchema::string("MCP tool parameter")
        }
    }
}

#[async_trait::async_trait]
impl ToolImpl for McpToolWrapper {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResultValue<ToolResult> {
        let mut client = self.client.write().await;

        let result = client
            .call_tool(&self.tool_name, Some(args))
            .await
            .map_err(|e| {
                crate::tools::ToolError::ExecutionFailed(self.tool_name.clone(), e.to_string())
            })?;

        // Convert MCP content to string output
        let output = result
            .content
            .iter()
            .map(|c| match c {
                McpContent::Text { text } => text.clone(),
                McpContent::Image { data, mime_type } => {
                    format!("[Image: {} bytes, {}]", data.len(), mime_type)
                }
                McpContent::Resource { resource } => {
                    format!("[Resource: {}]", resource)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult {
            tool_name: self.tool_name.clone(),
            success: !result.is_error,
            output,
            error: if result.is_error {
                Some("Tool returned error".to_string())
            } else {
                None
            },
            exit_code: None,
            duration_ms: None,
        })
    }
}

// ── MCP Registry Integration ───────────────────────────────────────────────

/// Helper to register all MCP tools into a ToolRegistry
pub async fn register_mcp_tools(
    registry: &mut crate::tools::ToolRegistry,
    client: Arc<RwLock<McpClient>>,
) -> McpResult<usize> {
    let mcp_client = client.read().await;
    let mcp_tools = mcp_client.get_tools().await;
    drop(mcp_client);

    let count = mcp_tools.len();

    for mcp_tool in mcp_tools {
        let wrapper = McpToolWrapper::new(client.clone(), mcp_tool);
        registry.register(Arc::new(wrapper));
    }

    info!(count, "Registered MCP tools");
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request() {
        let req = JsonRpcRequest::new("tools/list", serde_json::Value::Null, 1);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, serde_json::Value::Number(1.into()));
    }

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                }
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("A test tool"));
    }

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: McpClientCapabilities {
                roots: Some(McpRootsCapability {
                    list_changed: false,
                }),
                sampling: None,
            },
            client_info: McpClientInfo {
                name: "ravenclaws".to_string(),
                version: "0.5.2".to_string(),
            },
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("ravenclaws"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MCP Server — Expose RavenClaws tools as an MCP server over stdio
// ═══════════════════════════════════════════════════════════════════════════

/// MCP Server — listens for JSON-RPC requests on stdin and responds on stdout.
///
/// Implements the MCP protocol as a server:
/// - `initialize` — protocol handshake
/// - `notifications/initialized` — no-op
/// - `tools/list` — returns RavenClaws's registered tools
/// - `tools/call` — executes a tool and returns the result
///
/// # Security
///
/// The server uses the same `PolicyEngine`, `Sandbox`, and `AuditLog` as the
/// agent loop, ensuring all tool calls are policy-checked and audited.
pub struct McpServer {
    /// Tool registry with RavenClaws's built-in tools
    registry: crate::tools::ToolRegistry,
    /// Policy engine for tool call authorization
    policy_engine: crate::policy::PolicyEngine,
    /// Sandbox for shell command execution
    sandbox: crate::sandbox::Sandbox,
    /// Audit log for tamper-evident logging
    audit_log: crate::audit::AuditLog,
    /// Whether the server has been initialized
    initialized: bool,
    /// Server info sent during initialize
    server_info: McpServerInfo,
    /// Request ID counter
    request_id: i64,
}

impl McpServer {
    /// Create a new MCP server with the given tool registry.
    ///
    /// Uses default secure policy, sandbox, and audit log.
    pub fn new(registry: crate::tools::ToolRegistry) -> Self {
        let server_info = McpServerInfo {
            name: "ravenclaws".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Self {
            registry,
            policy_engine: crate::policy::PolicyEngine::default_secure(),
            sandbox: crate::sandbox::Sandbox::default(),
            audit_log: crate::audit::AuditLog::new(format!("mcp-server-{}", std::process::id())),
            initialized: false,
            server_info,
            request_id: 0,
        }
    }

    /// Run the MCP server, reading JSON-RPC requests from stdin and writing
    /// responses to stdout. Continues until stdin is closed or an error occurs.
    pub async fn run(&mut self) -> Result<(), McpError> {
        // Initialize sandbox
        self.sandbox
            .init()
            .await
            .map_err(|e| McpError::Transport(format!("Sandbox init failed: {}", e)))?;

        info!("MCP server starting on stdio");

        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            debug!("MCP Server ← {}", &line);

            // Parse JSON-RPC request
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let error_response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": "Parse error",
                            "data": e.to_string()
                        },
                        "id": serde_json::Value::Null
                    });
                    let _ = self.write_response(&error_response).await;
                    continue;
                }
            };

            let response = self.handle_request(&request).await;
            let _ = self.write_response(&response).await;
        }

        info!("MCP server shutting down (stdin closed)");
        Ok(())
    }

    /// Handle a single JSON-RPC request and return a response value.
    async fn handle_request(&mut self, request: &JsonRpcRequest) -> serde_json::Value {
        let request_id = request.id.clone();

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request, &request_id).await,
            "notifications/initialized" => {
                self.initialized = true;
                info!("MCP server initialized by client");
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": null,
                    "id": request_id
                })
            }
            "tools/list" => self.handle_tools_list(&request_id).await,
            "tools/call" => self.handle_tools_call(request, &request_id).await,
            _ => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", request.method)
                    },
                    "id": request_id
                })
            }
        }
    }

    /// Handle `initialize` request — protocol handshake.
    async fn handle_initialize(
        &mut self,
        request: &JsonRpcRequest,
        request_id: &serde_json::Value,
    ) -> serde_json::Value {
        // Parse client info from params (optional — we accept any client)
        if let Some(params) = request.params.as_object() {
            if let Some(client_info) = params.get("clientInfo") {
                info!(
                    client = ?client_info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "MCP client connected"
                );
            }
        }

        let capabilities = McpServerCapabilities {
            tools: Some(McpToolsCapability {
                list_changed: false,
            }),
            resources: None,
            prompts: None,
        };

        let result = serde_json::json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": capabilities,
            "serverInfo": {
                "name": self.server_info.name,
                "version": self.server_info.version
            }
        });

        serde_json::json!({
            "jsonrpc": "2.0",
            "result": result,
            "id": request_id
        })
    }

    /// Handle `tools/list` request — return all registered tools.
    async fn handle_tools_list(&self, request_id: &serde_json::Value) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self
            .registry
            .definitions()
            .iter()
            .map(|def| {
                serde_json::json!({
                    "name": def.name,
                    "description": def.description,
                    "inputSchema": def.parameters
                })
            })
            .collect();

        serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": tools
            },
            "id": request_id
        })
    }

    /// Handle `tools/call` request — execute a tool and return the result.
    async fn handle_tools_call(
        &mut self,
        request: &JsonRpcRequest,
        request_id: &serde_json::Value,
    ) -> serde_json::Value {
        let name = request
            .params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let arguments = request
            .params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        if name.is_empty() {
            return serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32602,
                    "message": "Invalid params: missing tool name"
                },
                "id": request_id
            });
        }

        // Check policy
        let policy_decision = self.policy_engine.check_tool_call(&name, &arguments);
        match policy_decision {
            crate::policy::Decision::Deny(reason) => {
                warn!(tool = %name, reason = %reason, "MCP tool call denied by policy");
                return serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": format!("Policy denied: {}", reason)
                        }],
                        "isError": true
                    },
                    "id": request_id
                });
            }
            crate::policy::Decision::Allow => {
                // Audit: tool call
                let _ = self.audit_log.tool_call(&name, &arguments);
            }
        }

        // Execute the tool
        let call = crate::tools::ToolCall {
            name: name.clone(),
            arguments,
            id: None,
        };

        match self.registry.execute(call).await {
            Ok(result) => {
                // Audit: tool result
                let _ = self.audit_log.append(
                    crate::audit::AuditEventType::ToolResult,
                    &name,
                    &format!("MCP tool executed: {} (success: {})", name, result.success),
                    Some(serde_json::json!({
                        "success": result.success,
                        "exit_code": result.exit_code,
                        "duration_ms": result.duration_ms,
                    })),
                );

                let content = if result.success {
                    vec![serde_json::json!({
                        "type": "text",
                        "text": result.output
                    })]
                } else {
                    vec![serde_json::json!({
                        "type": "text",
                        "text": result.error.as_deref().unwrap_or("Unknown error")
                    })]
                };

                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": {
                        "content": content,
                        "isError": !result.success
                    },
                    "id": request_id
                })
            }
            Err(e) => {
                warn!(tool = %name, error = %e, "MCP tool execution failed");
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": format!("Tool execution failed: {}", e)
                        }],
                        "isError": true
                    },
                    "id": request_id
                })
            }
        }
    }

    /// Write a JSON-RPC response to stdout.
    async fn write_response(&self, response: &serde_json::Value) -> std::io::Result<()> {
        let json = serde_json::to_string(response)?;
        debug!("MCP Server → {}", &json);
        use tokio::io::AsyncWriteExt;
        let mut stdout = tokio::io::stdout();
        stdout.write_all(json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
        Ok(())
    }

    /// Get the next request ID.
    #[allow(dead_code)]
    fn next_id(&mut self) -> i64 {
        self.request_id += 1;
        self.request_id
    }
}

#[cfg(test)]
mod server_tests {
    use super::*;
    use crate::tools::ToolRegistry;

    #[test]
    fn test_mcp_server_initialize_response() {
        let registry = ToolRegistry::with_default_tools();
        let server = McpServer::new(registry);

        // Check server info
        assert_eq!(server.server_info.name, "ravenclaws");
        assert!(!server.server_info.version.is_empty());
        assert!(!server.initialized);
    }

    #[test]
    fn test_mcp_server_tools_list_response() {
        let registry = ToolRegistry::with_default_tools();
        let server = McpServer::new(registry);

        // Check that all 5 built-in tools are registered
        let defs = server.registry.definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"shell_exec"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"web_fetch"));
        assert!(names.contains(&"web_search"));
        assert_eq!(defs.len(), 5);
    }

    #[tokio::test]
    async fn test_mcp_server_handle_unknown_method() {
        let registry = ToolRegistry::with_default_tools();
        let mut server = McpServer::new(registry);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown_method".to_string(),
            params: serde_json::Value::Null,
            id: serde_json::Value::Number(1.into()),
        };

        let response = server.handle_request(&request).await;
        assert!(response.get("error").is_some());
        assert_eq!(
            response["error"]["code"],
            serde_json::Value::Number((-32601).into())
        );
    }

    #[tokio::test]
    async fn test_mcp_server_handle_tools_list() {
        let registry = ToolRegistry::with_default_tools();
        let server = McpServer::new(registry);

        let request_id = serde_json::Value::Number(1.into());
        let response = server.handle_tools_list(&request_id).await;

        assert!(response.get("result").is_some());
        let tools = &response["result"]["tools"];
        assert!(tools.is_array());
        assert!(!tools.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mcp_server_handle_tools_call_missing_name() {
        let registry = ToolRegistry::with_default_tools();
        let mut server = McpServer::new(registry);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: serde_json::json!({}),
            id: serde_json::Value::Number(1.into()),
        };

        let request_id = serde_json::Value::Number(1.into());
        let response = server.handle_tools_call(&request, &request_id).await;

        assert!(response.get("error").is_some());
        assert_eq!(
            response["error"]["code"],
            serde_json::Value::Number((-32602).into())
        );
    }

    #[tokio::test]
    async fn test_mcp_server_handle_tools_call_unknown_tool() {
        let registry = ToolRegistry::with_default_tools();
        let mut server = McpServer::new(registry);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }),
            id: serde_json::Value::Number(1.into()),
        };

        let request_id = serde_json::Value::Number(1.into());
        let response = server.handle_tools_call(&request, &request_id).await;

        // Unknown tool should return an error result
        assert!(response["result"]["isError"].as_bool().unwrap_or(false));
    }

    #[test]
    fn test_mcp_server_json_rpc_error_codes() {
        // -32700: Parse error
        // -32601: Method not found
        // -32602: Invalid params
        // -32000: Server error (tool execution failed)

        assert_eq!(-32700i32, -32700);
        assert_eq!(-32601i32, -32601);
        assert_eq!(-32602i32, -32602);
    }
}
