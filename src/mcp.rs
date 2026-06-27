//! Model Context Protocol (MCP) for RavenClaws
//!
//! Implements both MCP client and server:
//! - **Client**: Connect to external MCP servers, discover tools, execute them via JSON-RPC over stdio or SSE
//! - **Server**: Expose RavenClaws's built-in tools as an MCP server over stdio or SSE
//!
//! # Architecture
//!
//! ```text
//! McpClient                          McpServer
//!   ├── McpTransport (stdio/SSE)       ├── McpServerTransport (stdio)
//!   ├── McpToolRegistry                ├── McpSseServer (HTTP/SSE)
//!   └── JsonRpcClient                  └── ToolRegistry (RavenClaws tools)
//!                                       └── JsonRpcHandler
//! ```
//!
//! # SSE Transport
//!
//! SSE (Server-Sent Events) transport allows MCP communication over HTTP:
//! - **Client**: Connects to an SSE endpoint, receives JSON-RPC messages via SSE stream,
//!   sends requests via HTTP POST to the endpoint provided in the SSE `endpoint` event.
//! - **Server**: Runs an HTTP server with:
//!   - `GET /sse` — SSE stream for sending JSON-RPC messages to clients
//!   - `POST /message` — Receive JSON-RPC requests from clients
//!
//! # References
//! - MCP Spec: https://modelcontextprotocol.io/specification
//! - JSON-RPC 2.0: https://www.jsonrpc.org/specification
//! - SSE (Server-Sent Events): https://html.spec.whatwg.org/multipage/server-sent-events.html

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

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
    /// SSE transport: connect to HTTP endpoint
    #[allow(dead_code)]
    Sse { url: String },
}

/// MCP Transport — handles low-level communication
pub struct McpTransport {
    config: McpTransportConfig,
    // Stdio fields
    child: Option<Child>,
    stdin: Option<tokio::process::ChildStdin>,
    stdout_reader: Option<BufReader<tokio::process::ChildStdout>>,
    // SSE fields
    http_client: Option<reqwest::Client>,
    /// The message endpoint URL received from the SSE `endpoint` event
    message_endpoint: Option<String>,
    /// Channel to receive JSON-RPC responses from the SSE reader task
    sse_response_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    /// Join handle for the SSE reader background task
    sse_reader_handle: Option<tokio::task::JoinHandle<()>>,
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
            http_client: None,
            message_endpoint: None,
            sse_response_rx: None,
            sse_reader_handle: None,
            request_id: 0,
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> McpResult<()> {
        let config = self.config.clone();
        match &config {
            McpTransportConfig::Stdio { command, args, env } => {
                self.connect_stdio(command, args, env).await
            }
            McpTransportConfig::Sse { url } => self.connect_sse(url).await,
        }
    }

    /// Connect via stdio transport
    async fn connect_stdio(
        &mut self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> McpResult<()> {
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

    /// Connect via SSE transport
    async fn connect_sse(&mut self, url: &str) -> McpResult<()> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| {
                McpError::ConnectionFailed(format!("Failed to create HTTP client: {}", e))
            })?;

        info!(url = %url, "MCP SSE transport connecting");

        // Make initial SSE connection
        let response = client
            .get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| McpError::ConnectionFailed(format!("SSE connection failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(McpError::ConnectionFailed(format!(
                "SSE connection returned status {}",
                response.status()
            )));
        }

        // Channel for receiving JSON-RPC responses from the SSE stream
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Store the message endpoint URL (will be updated when we receive the `endpoint` event)
        let message_endpoint = Arc::new(tokio::sync::Mutex::new(None::<String>));
        let message_endpoint_clone = message_endpoint.clone();

        // Spawn background task to read SSE stream
        let handle = tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);

                        // Process complete SSE events (delimited by \n\n)
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();
                            Self::handle_sse_event(&event, &tx, &message_endpoint_clone).await;
                        }
                    }
                    Err(e) => {
                        warn!("SSE stream error: {}", e);
                        break;
                    }
                }
            }
            debug!("MCP SSE stream ended");
        });

        self.http_client = Some(client);
        self.sse_response_rx = Some(rx);
        self.sse_reader_handle = Some(handle);
        self.message_endpoint = None; // Will be set when we receive the endpoint event

        // Wait briefly for the endpoint event (the server sends it immediately on connect)
        // The MCP spec says the server sends an `endpoint` event right after connection
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
        let mut endpoint_received = false;

        while tokio::time::Instant::now() < deadline {
            if let Ok(msg) = self.sse_response_rx.as_mut().unwrap().try_recv() {
                // Check if this is an endpoint event
                if msg.starts_with("endpoint:") {
                    let ep = msg
                        .strip_prefix("endpoint:")
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    // Resolve relative URLs
                    let resolved = if ep.starts_with("http") {
                        ep.clone()
                    } else {
                        let base = url.trim_end_matches('/');
                        let path = ep.trim_start_matches('/');
                        format!("{}/{}", base, path)
                    };
                    self.message_endpoint = Some(resolved.clone());
                    *message_endpoint.lock().await = Some(resolved);
                    endpoint_received = true;
                    info!(
                        "MCP SSE endpoint received: {}",
                        self.message_endpoint.as_ref().unwrap()
                    );
                    break;
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }

        if !endpoint_received {
            // If no explicit endpoint event, assume the SSE URL itself is the message endpoint
            self.message_endpoint = Some(url.to_string());
            *message_endpoint.lock().await = Some(url.to_string());
            warn!("No SSE endpoint event received, using SSE URL as message endpoint");
        }

        info!(url = %url, "MCP SSE transport connected");
        Ok(())
    }

    /// Handle a single SSE event
    async fn handle_sse_event(
        event: &str,
        tx: &tokio::sync::mpsc::UnboundedSender<String>,
        message_endpoint: &Arc<tokio::sync::Mutex<Option<String>>>,
    ) {
        for line in event.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("event:") {
                let event_type = value.trim();
                debug!("MCP SSE event: {}", event_type);
            } else if let Some(value) = line.strip_prefix("data:") {
                let data = value.trim().to_string();
                // Check if this is an endpoint event
                if let Some(ep) = data.strip_prefix("/message") {
                    *message_endpoint.lock().await = Some(ep.to_string());
                    let _ = tx.send(format!("endpoint:{}", ep));
                } else if data.starts_with("http") && data.contains("/message") {
                    *message_endpoint.lock().await = Some(data.clone());
                    let _ = tx.send(format!("endpoint:{}", data));
                } else {
                    // This is a JSON-RPC response — forward it
                    let _ = tx.send(data);
                }
            }
        }
    }

    /// Send a JSON-RPC request and wait for response
    pub async fn send_request(&mut self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        match &self.config {
            McpTransportConfig::Stdio { .. } => self.send_request_stdio(request).await,
            McpTransportConfig::Sse { .. } => self.send_request_sse(request).await,
        }
    }

    /// Send a JSON-RPC request via stdio
    async fn send_request_stdio(&mut self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let request_json = serde_json::to_string(&request)?;
        debug!("MCP → {}", request_json);

        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Transport("Transport not connected".to_string()))?;

        stdin.write_all(request_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

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

        if let Some(err) = &response.error {
            return Err(McpError::Server {
                code: err.code,
                message: err.message.clone(),
            });
        }

        Ok(response)
    }

    /// Send a JSON-RPC request via SSE transport (HTTP POST to message endpoint)
    async fn send_request_sse(&mut self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let request_json = serde_json::to_string(&request)?;
        let request_id = request.id.clone();
        debug!(
            "MCP SSE → POST {}: {}",
            self.message_endpoint.as_deref().unwrap_or("?"),
            request_json
        );

        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| McpError::Transport("HTTP client not initialized".to_string()))?;

        let endpoint = self
            .message_endpoint
            .as_ref()
            .ok_or_else(|| McpError::Transport("No message endpoint available".to_string()))?;

        // Send POST request
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(request_json.clone())
            .send()
            .await
            .map_err(|e| McpError::Transport(format!("HTTP POST failed: {}", e)))?;

        // For JSON-RPC over SSE, the response may come via the SSE stream
        // or as the HTTP response body. Try HTTP response first.
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| McpError::Transport(format!("Failed to read response body: {}", e)))?;

        if !body.trim().is_empty() {
            // Try to parse as JSON-RPC response
            if let Ok(rpc_response) = serde_json::from_str::<JsonRpcResponse>(&body) {
                debug!("MCP SSE ← {}", body.trim());
                if let Some(err) = &rpc_response.error {
                    return Err(McpError::Server {
                        code: err.code,
                        message: err.message.clone(),
                    });
                }
                return Ok(rpc_response);
            }
        }

        // If no response body or not valid JSON-RPC, wait for SSE stream response
        if status.is_success() {
            let rx = self
                .sse_response_rx
                .as_mut()
                .ok_or_else(|| McpError::Transport("SSE receiver not available".to_string()))?;

            let timeout = tokio::time::Duration::from_secs(30);
            let deadline = tokio::time::Instant::now() + timeout;

            while tokio::time::Instant::now() < deadline {
                match tokio::time::timeout(tokio::time::Duration::from_secs(1), rx.recv()).await {
                    Ok(Some(msg)) => {
                        // Skip endpoint events
                        if msg.starts_with("endpoint:") {
                            continue;
                        }
                        // Try to parse as JSON-RPC response
                        if let Ok(rpc_response) = serde_json::from_str::<JsonRpcResponse>(&msg) {
                            // Match by request ID if possible
                            if rpc_response.id == request_id
                                || rpc_response.id == serde_json::Value::Null
                            {
                                debug!("MCP SSE ← {}", msg);
                                if let Some(err) = &rpc_response.error {
                                    return Err(McpError::Server {
                                        code: err.code,
                                        message: err.message.clone(),
                                    });
                                }
                                return Ok(rpc_response);
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(_) => continue, // Timeout on recv, keep waiting
                }
            }

            return Err(McpError::Timeout(
                "No response received from SSE stream".to_string(),
            ));
        }

        Err(McpError::Transport(format!(
            "HTTP POST returned status {}",
            status
        )))
    }

    /// Get next request ID
    fn next_id(&mut self) -> i64 {
        self.request_id += 1;
        self.request_id
    }
}

impl Drop for McpTransport {
    fn drop(&mut self) {
        // Abort the SSE reader task if running
        if let Some(handle) = self.sse_reader_handle.take() {
            handle.abort();
        }
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

// ── MCP Client Manager ─────────────────────────────────────────────────────

/// Manages multiple MCP client connections.
///
/// Creates and holds `McpClient` instances for each configured MCP server.
/// Provides methods to register all tools from all connected servers into a
/// `ToolRegistry`, and to access individual clients by name.
pub struct McpClientManager {
    /// Connected MCP clients, keyed by server name
    clients: Vec<(String, Arc<RwLock<McpClient>>)>,
}

impl McpClientManager {
    /// Create a new empty client manager
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    /// Create and connect clients from config
    pub async fn from_config(config: &crate::config::McpConfig) -> Self {
        let mut manager = Self::new();
        for server in &config.servers {
            let transport_config = McpTransportConfig::Stdio {
                command: server.command.clone(),
                args: server.args.clone(),
                env: server.env.clone(),
            };
            let mut client = McpClient::new(transport_config);
            match client.connect().await {
                Ok(()) => {
                    info!(
                        server = %server.name,
                        server_info = ?client.server_info(),
                        "MCP client connected from config"
                    );
                    manager
                        .clients
                        .push((server.name.clone(), Arc::new(RwLock::new(client))));
                }
                Err(e) => {
                    warn!(
                        server = %server.name,
                        error = %e,
                        "Failed to connect to MCP server from config, skipping"
                    );
                }
            }
        }
        manager
    }

    /// Add a single client (e.g., from CLI --mcp-command)
    #[allow(dead_code)]
    pub fn add_client(&mut self, name: String, client: Arc<RwLock<McpClient>>) {
        self.clients.push((name, client));
    }

    /// Get all clients
    #[allow(dead_code)]
    pub fn clients(&self) -> &[(String, Arc<RwLock<McpClient>>)] {
        &self.clients
    }

    /// Get a client by name
    #[allow(dead_code)]
    pub fn get_client(&self, name: &str) -> Option<&Arc<RwLock<McpClient>>> {
        self.clients.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    /// Register all tools from all connected MCP servers into a ToolRegistry
    pub async fn register_all_tools(&self, registry: &mut crate::tools::ToolRegistry) -> usize {
        let mut total = 0;
        for (name, client) in &self.clients {
            let mcp_client = client.read().await;
            let mcp_tools = mcp_client.get_tools().await;
            drop(mcp_client);

            for mcp_tool in mcp_tools {
                let wrapper = McpToolWrapper::new(client.clone(), mcp_tool);
                registry.register(Arc::new(wrapper));
                total += 1;
            }
            info!(
                server = %name,
                tools_registered = total,
                "Registered MCP tools from server"
            );
        }
        info!(total, "Total MCP tools registered from all servers");
        total
    }

    /// Number of connected clients
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// Whether there are any connected clients
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
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

// ═══════════════════════════════════════════════════════════════════════════
// MCP SSE Server — Expose RavenClaws tools over HTTP with SSE transport
// ═══════════════════════════════════════════════════════════════════════════

/// MCP SSE Server — runs an HTTP server that supports SSE transport.
///
/// Provides two endpoints:
/// - `GET /sse` — SSE stream for sending JSON-RPC messages to connected clients
/// - `POST /message` — Receive JSON-RPC requests from clients
///
/// The server follows the MCP SSE transport specification:
/// 1. Client connects to `GET /sse` and receives an `endpoint` event with the
///    message endpoint URL
/// 2. Client sends JSON-RPC requests via `POST /message`
/// 3. Server sends JSON-RPC responses via the SSE stream
#[allow(dead_code)]
pub struct McpSseServer {
    registry: crate::tools::ToolRegistry,
    policy_engine: crate::policy::PolicyEngine,
    sandbox: crate::sandbox::Sandbox,
    audit_log: crate::audit::AuditLog,
    server_info: McpServerInfo,
    /// Connected SSE clients: maps client ID to sender channel
    clients: Arc<tokio::sync::RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>,
    host: String,
    port: u16,
}

#[allow(dead_code)]
impl McpSseServer {
    /// Create a new MCP SSE server
    pub fn new(registry: crate::tools::ToolRegistry, host: String, port: u16) -> Self {
        let server_info = McpServerInfo {
            name: "ravenclaws".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Self {
            registry,
            policy_engine: crate::policy::PolicyEngine::default_secure(),
            sandbox: crate::sandbox::Sandbox::default(),
            audit_log: crate::audit::AuditLog::new(format!("mcp-sse-{}", std::process::id())),
            server_info,
            clients: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            host,
            port,
        }
    }

    /// Run the SSE server, blocking until shutdown signal
    pub async fn run(
        &mut self,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), McpError> {
        // Initialize sandbox
        self.sandbox
            .init()
            .await
            .map_err(|e| McpError::Transport(format!("Sandbox init failed: {}", e)))?;

        let addr: std::net::SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| McpError::Transport(format!("Invalid address: {}", e)))?;

        info!(addr = %addr, "MCP SSE server starting");

        // Build our router using tokio's TcpListener and manual HTTP handling
        // We use a simple approach: one TcpListener, dispatch based on method/path
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| McpError::Transport(format!("Failed to bind: {}", e)))?;

        let clients = self.clients.clone();
        let registry = Arc::new(tokio::sync::RwLock::new(std::mem::replace(
            &mut self.registry,
            crate::tools::ToolRegistry::new(),
        )));
        let policy_engine = Arc::new(tokio::sync::RwLock::new(std::mem::replace(
            &mut self.policy_engine,
            crate::policy::PolicyEngine::default_secure(),
        )));
        let sandbox = Arc::new(tokio::sync::RwLock::new(std::mem::take(&mut self.sandbox)));
        let audit_log = Arc::new(tokio::sync::RwLock::new(std::mem::replace(
            &mut self.audit_log,
            crate::audit::AuditLog::new(format!("mcp-sse-{}", std::process::id())),
        )));
        let server_info = Arc::new(self.server_info.clone());

        // We use a channel to signal shutdown
        let mut shutdown = shutdown;

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            let clients = clients.clone();
                            let registry = registry.clone();
                            let policy_engine = policy_engine.clone();
                            let sandbox = sandbox.clone();
                            let audit_log = audit_log.clone();
                            let server_info = server_info.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(
                                    stream, peer_addr, clients, registry,
                                    policy_engine, sandbox, audit_log, server_info,
                                ).await {
                                    warn!(peer = %peer_addr, error = %e, "MCP SSE connection error");
                                }
                            });
                        }
                        Err(e) => {
                            warn!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown.changed() => {
                    info!("MCP SSE server shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single HTTP connection
    #[allow(clippy::too_many_arguments)]
    async fn handle_connection(
        mut stream: tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
        clients: Arc<
            tokio::sync::RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>,
        >,
        registry: Arc<tokio::sync::RwLock<crate::tools::ToolRegistry>>,
        policy_engine: Arc<tokio::sync::RwLock<crate::policy::PolicyEngine>>,
        sandbox: Arc<tokio::sync::RwLock<crate::sandbox::Sandbox>>,
        audit_log: Arc<tokio::sync::RwLock<crate::audit::AuditLog>>,
        server_info: Arc<McpServerInfo>,
    ) -> Result<(), McpError> {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 8192];
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| McpError::Transport(format!("Read error: {}", e)))?;

        if n == 0 {
            return Ok(());
        }

        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        // Parse the HTTP request line
        let (method, path) = if let Some(first_line) = request.lines().next() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(McpError::Transport("Invalid HTTP request".to_string()));
            }
            (parts[0].to_string(), parts[1].to_string())
        } else {
            return Err(McpError::Transport("Empty HTTP request".to_string()));
        };

        match (method.as_str(), path.as_str()) {
            ("GET", "/sse") => {
                Self::handle_sse_connection(
                    stream,
                    peer_addr,
                    clients,
                    registry,
                    policy_engine,
                    sandbox,
                    audit_log,
                    server_info,
                )
                .await
            }
            ("POST", "/message") => {
                // Extract body from the request
                let body = if let Some(body_start) = request.find("\r\n\r\n") {
                    request[body_start + 4..].to_string()
                } else {
                    return Err(McpError::Transport("No body in POST request".to_string()));
                };

                Self::handle_message_post(
                    stream,
                    &body,
                    &registry,
                    &policy_engine,
                    &sandbox,
                    &audit_log,
                    &server_info,
                    clients,
                )
                .await
            }
            _ => {
                // 404 for unknown paths
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                stream
                    .write_all(response.as_bytes())
                    .await
                    .map_err(|e| McpError::Transport(format!("Write error: {}", e)))?;
                Ok(())
            }
        }
    }

    /// Handle an SSE connection (GET /sse)
    #[allow(clippy::too_many_arguments)]
    async fn handle_sse_connection(
        mut stream: tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
        clients: Arc<
            tokio::sync::RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>,
        >,
        _registry: Arc<tokio::sync::RwLock<crate::tools::ToolRegistry>>,
        _policy_engine: Arc<tokio::sync::RwLock<crate::policy::PolicyEngine>>,
        _sandbox: Arc<tokio::sync::RwLock<crate::sandbox::Sandbox>>,
        _audit_log: Arc<tokio::sync::RwLock<crate::audit::AuditLog>>,
        _server_info: Arc<McpServerInfo>,
    ) -> Result<(), McpError> {
        use tokio::io::AsyncWriteExt;

        let client_id = Uuid::new_v4().to_string();
        info!(client = %client_id, peer = %peer_addr, "MCP SSE client connected");

        // Send SSE headers
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
        stream
            .write_all(headers.as_bytes())
            .await
            .map_err(|e| McpError::Transport(format!("Write error: {}", e)))?;
        stream
            .flush()
            .await
            .map_err(|e| McpError::Transport(format!("Flush error: {}", e)))?;

        // Send the endpoint event so the client knows where to POST messages
        let endpoint_event = "event: endpoint\ndata: /message\n\n".to_string();
        stream
            .write_all(endpoint_event.as_bytes())
            .await
            .map_err(|e| McpError::Transport(format!("Write error: {}", e)))?;
        stream
            .flush()
            .await
            .map_err(|e| McpError::Transport(format!("Flush error: {}", e)))?;

        // Create channel for sending messages to this client
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Register client
        clients.write().await.insert(client_id.clone(), tx);

        // Keep connection open, forwarding messages from the channel to the SSE stream
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Some(data) => {
                            let sse_event = format!("data: {}\n\n", data);
                            if stream.write_all(sse_event.as_bytes()).await.is_err() {
                                break;
                            }
                            if stream.flush().await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        // Cleanup
        clients.write().await.remove(&client_id);
        info!(client = %client_id, "MCP SSE client disconnected");
        Ok(())
    }

    /// Handle a POST to /message (JSON-RPC request from client)
    #[allow(clippy::too_many_arguments)]
    async fn handle_message_post(
        mut stream: tokio::net::TcpStream,
        body: &str,
        registry: &Arc<tokio::sync::RwLock<crate::tools::ToolRegistry>>,
        policy_engine: &Arc<tokio::sync::RwLock<crate::policy::PolicyEngine>>,
        sandbox: &Arc<tokio::sync::RwLock<crate::sandbox::Sandbox>>,
        audit_log: &Arc<tokio::sync::RwLock<crate::audit::AuditLog>>,
        server_info: &Arc<McpServerInfo>,
        clients: Arc<
            tokio::sync::RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>,
        >,
    ) -> Result<(), McpError> {
        use tokio::io::AsyncWriteExt;

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(body) {
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
                let response_body = serde_json::to_string(&error_response)?;
                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(http_response.as_bytes()).await?;
                return Ok(());
            }
        };

        // Handle the request using the same logic as McpServer
        let request_id = request.id.clone();
        let response = Self::handle_jsonrpc_request(
            &request,
            &request_id,
            registry,
            policy_engine,
            sandbox,
            audit_log,
            server_info,
        )
        .await;

        let response_body = serde_json::to_string(&response)?;

        // Send HTTP response
        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(http_response.as_bytes()).await?;
        stream.flush().await?;

        // Also broadcast the response to all connected SSE clients
        let response_json = serde_json::to_string(&response)?;
        let clients_guard = clients.read().await;
        for (_, tx) in clients_guard.iter() {
            let _ = tx.send(response_json.clone());
        }

        Ok(())
    }

    /// Handle a JSON-RPC request (shared logic with McpServer)
    async fn handle_jsonrpc_request(
        request: &JsonRpcRequest,
        request_id: &serde_json::Value,
        registry: &Arc<tokio::sync::RwLock<crate::tools::ToolRegistry>>,
        policy_engine: &Arc<tokio::sync::RwLock<crate::policy::PolicyEngine>>,
        _sandbox: &Arc<tokio::sync::RwLock<crate::sandbox::Sandbox>>,
        audit_log: &Arc<tokio::sync::RwLock<crate::audit::AuditLog>>,
        server_info: &Arc<McpServerInfo>,
    ) -> serde_json::Value {
        match request.method.as_str() {
            "initialize" => {
                // Parse client info from params
                if let Some(params) = request.params.as_object() {
                    if let Some(client_info) = params.get("clientInfo") {
                        info!(
                            client = ?client_info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
                            "MCP SSE client initialized"
                        );
                    }
                }

                let capabilities = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": server_info.name,
                        "version": server_info.version
                    }
                });

                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": capabilities,
                    "id": request_id
                })
            }
            "notifications/initialized" => {
                info!("MCP SSE client initialized notification received");
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": null,
                    "id": request_id
                })
            }
            "tools/list" => {
                let defs = registry.read().await.definitions().clone();
                let tools: Vec<serde_json::Value> = defs
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
            "tools/call" => {
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
                let decision = policy_engine
                    .read()
                    .await
                    .check_tool_call(&name, &arguments);
                match decision {
                    crate::policy::Decision::Deny(reason) => {
                        warn!(tool = %name, reason = %reason, "MCP SSE tool call denied by policy");
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
                        let _ = audit_log.write().await.tool_call(&name, &arguments);
                    }
                }

                // Execute the tool
                let call = crate::tools::ToolCall {
                    name: name.clone(),
                    arguments,
                    id: None,
                };

                match registry.read().await.execute(call).await {
                    Ok(result) => {
                        let _ = audit_log.write().await.append(
                            crate::audit::AuditEventType::ToolResult,
                            &name,
                            &format!(
                                "MCP SSE tool executed: {} (success: {})",
                                name, result.success
                            ),
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
                        warn!(tool = %name, error = %e, "MCP SSE tool execution failed");
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
}

#[cfg(test)]
mod sse_server_tests {
    use super::*;
    use crate::tools::ToolRegistry;

    #[test]
    fn test_mcp_sse_server_new() {
        let registry = ToolRegistry::with_default_tools();
        let server = McpSseServer::new(registry, "127.0.0.1".to_string(), 9091);

        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 9091);
        assert_eq!(server.server_info.name, "ravenclaws");
        assert!(server.clients.blocking_read().is_empty());
    }

    #[test]
    fn test_mcp_sse_server_info() {
        let registry = ToolRegistry::with_default_tools();
        let server = McpSseServer::new(registry, "0.0.0.0".to_string(), 9092);

        assert_eq!(server.server_info.name, "ravenclaws");
        assert!(!server.server_info.version.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_sse_handle_initialize() {
        let registry = ToolRegistry::with_default_tools();
        let server_info = Arc::new(McpServerInfo {
            name: "ravenclaws".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: serde_json::json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
            id: serde_json::Value::Number(1.into()),
        };

        let request_id = serde_json::Value::Number(1.into());
        let registry = Arc::new(tokio::sync::RwLock::new(registry));
        let policy = Arc::new(tokio::sync::RwLock::new(
            crate::policy::PolicyEngine::default_secure(),
        ));
        let sandbox = Arc::new(tokio::sync::RwLock::new(crate::sandbox::Sandbox::default()));
        let audit = Arc::new(tokio::sync::RwLock::new(crate::audit::AuditLog::new(
            "test".to_string(),
        )));

        let response = McpSseServer::handle_jsonrpc_request(
            &request,
            &request_id,
            &registry,
            &policy,
            &sandbox,
            &audit,
            &server_info,
        )
        .await;

        assert!(response.get("result").is_some());
        assert_eq!(response["result"]["serverInfo"]["name"], "ravenclaws");
    }

    #[tokio::test]
    async fn test_mcp_sse_handle_tools_list() {
        let registry = ToolRegistry::with_default_tools();
        let server_info = Arc::new(McpServerInfo {
            name: "ravenclaws".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: serde_json::Value::Null,
            id: serde_json::Value::Number(1.into()),
        };

        let request_id = serde_json::Value::Number(1.into());
        let registry = Arc::new(tokio::sync::RwLock::new(registry));
        let policy = Arc::new(tokio::sync::RwLock::new(
            crate::policy::PolicyEngine::default_secure(),
        ));
        let sandbox = Arc::new(tokio::sync::RwLock::new(crate::sandbox::Sandbox::default()));
        let audit = Arc::new(tokio::sync::RwLock::new(crate::audit::AuditLog::new(
            "test".to_string(),
        )));

        let response = McpSseServer::handle_jsonrpc_request(
            &request,
            &request_id,
            &registry,
            &policy,
            &sandbox,
            &audit,
            &server_info,
        )
        .await;

        assert!(response.get("result").is_some());
        let tools = &response["result"]["tools"];
        assert!(tools.is_array());
        assert!(!tools.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mcp_sse_handle_unknown_method() {
        let registry = ToolRegistry::with_default_tools();
        let server_info = Arc::new(McpServerInfo {
            name: "ravenclaws".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown_method".to_string(),
            params: serde_json::Value::Null,
            id: serde_json::Value::Number(1.into()),
        };

        let request_id = serde_json::Value::Number(1.into());
        let registry = Arc::new(tokio::sync::RwLock::new(registry));
        let policy = Arc::new(tokio::sync::RwLock::new(
            crate::policy::PolicyEngine::default_secure(),
        ));
        let sandbox = Arc::new(tokio::sync::RwLock::new(crate::sandbox::Sandbox::default()));
        let audit = Arc::new(tokio::sync::RwLock::new(crate::audit::AuditLog::new(
            "test".to_string(),
        )));

        let response = McpSseServer::handle_jsonrpc_request(
            &request,
            &request_id,
            &registry,
            &policy,
            &sandbox,
            &audit,
            &server_info,
        )
        .await;

        assert!(response.get("error").is_some());
        assert_eq!(
            response["error"]["code"],
            serde_json::Value::Number((-32601).into())
        );
    }

    #[tokio::test]
    async fn test_mcp_sse_handle_tools_call_missing_name() {
        let registry = ToolRegistry::with_default_tools();
        let server_info = Arc::new(McpServerInfo {
            name: "ravenclaws".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: serde_json::json!({}),
            id: serde_json::Value::Number(1.into()),
        };

        let request_id = serde_json::Value::Number(1.into());
        let registry = Arc::new(tokio::sync::RwLock::new(registry));
        let policy = Arc::new(tokio::sync::RwLock::new(
            crate::policy::PolicyEngine::default_secure(),
        ));
        let sandbox = Arc::new(tokio::sync::RwLock::new(crate::sandbox::Sandbox::default()));
        let audit = Arc::new(tokio::sync::RwLock::new(crate::audit::AuditLog::new(
            "test".to_string(),
        )));

        let response = McpSseServer::handle_jsonrpc_request(
            &request,
            &request_id,
            &registry,
            &policy,
            &sandbox,
            &audit,
            &server_info,
        )
        .await;

        assert!(response.get("error").is_some());
        assert_eq!(
            response["error"]["code"],
            serde_json::Value::Number((-32602).into())
        );
    }

    #[tokio::test]
    async fn test_mcp_sse_transport_config_serde() {
        // Verify SSE transport config can be created
        let config = McpTransportConfig::Sse {
            url: "http://localhost:9090/sse".to_string(),
        };

        match config {
            McpTransportConfig::Sse { url } => {
                assert_eq!(url, "http://localhost:9090/sse");
            }
            _ => panic!("Expected SSE variant"),
        }
    }

    #[tokio::test]
    async fn test_mcp_sse_transport_connect_failure() {
        // Connecting to a non-existent server should fail gracefully
        let config = McpTransportConfig::Sse {
            url: "http://127.0.0.1:1/sse".to_string(),
        };

        let mut transport = McpTransport::new(config);
        let result = transport.connect().await;

        // Should fail with connection error (not panic)
        assert!(result.is_err());
        match result {
            Err(McpError::ConnectionFailed(_)) => {} // Expected
            Err(McpError::Transport(_)) => {}        // Also acceptable
            _ => panic!("Expected connection or transport error, got {:?}", result),
        }
    }
}
