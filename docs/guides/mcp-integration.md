# MCP Integration

RavenClaws supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) for connecting to external tools and services. This guide covers both MCP client and server modes.

## Overview

MCP provides a standardized way for AI agents to discover and use external tools. RavenClaws can:

- **Act as an MCP client** — Connect to MCP servers and use their tools
- **Act as an MCP server** — Expose RavenClaws' built-in tools to other MCP clients

## MCP Client Mode

In client mode, RavenClaws connects to one or more MCP servers and discovers their available tools. The agent can then use those tools during task execution.

### Transport Types

RavenClaws supports two MCP transport types:

| Transport | Description | Use Case |
|---|---|---|
| **Stdio** | Spawns a child process and communicates via stdin/stdout | Local MCP servers (filesystem, git, etc.) |
| **SSE** | Connects to an HTTP endpoint with Server-Sent Events | Remote MCP servers (network services, cloud-hosted) |

### Stdio Transport

#### Starting an MCP Server

First, you need an MCP server running. Here's an example using the official MCP filesystem server:

```bash
npx @modelcontextprotocol/server-filesystem /path/to/allowed/directory
```

#### Connecting RavenClaws as a Client (Stdio)

```bash
# Connect to an MCP server via stdio
ravenclaws --mcp-client "npx @modelcontextprotocol/server-filesystem /tmp" --exec "List files in the current directory"
```

### SSE Transport

SSE (Server-Sent Events) transport allows RavenClaws to connect to MCP servers over HTTP,
which is ideal for remote or containerized MCP servers.

#### Connecting RavenClaws as a Client (SSE)

Configure SSE-based MCP servers in your config file:

```toml
[mcp]
servers = [
  { name = "playwright", url = "http://playwright-mcp:8080/sse" },
  { name = "postgres", url = "http://postgres-mcp:8081/sse" },
  { name = "chromadb", url = "http://chromadb-mcp:8082/sse" },
]
```

Then run:

```bash
ravenclaws --exec "Query the database and summarize results"
```

You can also use the `--mcp-sse-url` CLI flag for a single SSE server:

```bash
ravenclaws --mcp-sse-url "http://localhost:8080/sse" --exec "Browse the web and find documentation"
```

#### How SSE Transport Works

1. RavenClaws connects to the SSE endpoint via HTTP GET
2. The server sends an `endpoint` event with the message endpoint URL
3. RavenClaws sends JSON-RPC requests via HTTP POST to the message endpoint
4. The server sends JSON-RPC responses via the SSE stream
5. Automatic reconnection with exponential backoff on disconnection

### Configuration (All Transports)

You can configure MCP servers in your config file:

```toml
[mcp]
servers = [
  # Stdio transport
  { command = "npx", args = ["@modelcontextprotocol/server-filesystem", "/tmp"], name = "filesystem" },
  { command = "npx", args = ["@modelcontextprotocol/server-github"], name = "github" },
  # SSE transport
  { name = "playwright", url = "http://playwright-mcp:8080/sse" },
]
```

Then run:

```bash
ravenclaws --exec "Find all Rust files in /tmp and count them"
```

### How It Works

1. RavenClaws connects to MCP servers (via stdio or SSE)
2. Discovers available tools via the `tools/list` endpoint
3. Registers discovered tools in the `ToolRegistry`
4. When the agent calls a tool, RavenClaws sends a `tools/call` request
5. Results are returned to the agent for further processing

## MCP Server Mode

In server mode, RavenClaws exposes its built-in tools (shell, read/write file, web fetch, web search) as MCP tools that other AI agents can discover and use.

### Stdio Server

#### Starting the MCP Stdio Server

```bash
ravenclaws --mcp-server
```

This starts RavenClaws in MCP server mode, listening on stdio for JSON-RPC 2.0 requests.

### SSE Server

RavenClaws also supports MCP over SSE (Server-Sent Events) transport, which allows
remote MCP clients to connect over HTTP.

#### Starting the MCP SSE Server

```bash
# Default port 8081
ravenclaws --mcp-sse-server

# Custom host and port
ravenclaws --mcp-sse-server --mcp-sse-host 127.0.0.1 --mcp-sse-port 9090
```

The SSE server provides two endpoints:

| Endpoint | Method | Description |
|---|---|---|
| `/sse` | GET | SSE stream for receiving JSON-RPC messages |
| `/message` | POST | Send JSON-RPC requests to the server |

#### Connecting from OpenClaw

Add RavenClaws as an MCP server in OpenClaw's config:

```json
{
  "mcpServers": {
    "ravenclaws": {
      "url": "http://ravenclaws:8081/sse"
    }
  }
}
```

#### Connecting from Claude Desktop

```json
{
  "mcpServers": {
    "ravenclaws": {
      "url": "http://localhost:8081/sse"
    }
  }
}
```

#### Connecting from VS Code

```json
{
  "mcp": {
    "servers": {
      "ravenclaws": {
        "url": "http://localhost:8081/sse"
      }
    }
  }
}
```

#### How SSE Server Works

1. Client connects to `GET /sse` and receives an `endpoint` event with the message URL
2. Client sends JSON-RPC requests via `POST /message`
3. Server processes requests through PolicyEngine, Sandbox, and AuditLog
4. Server sends JSON-RPC responses via the SSE stream
5. Multiple concurrent clients are supported via separate SSE connections

### Connecting from Another Agent (Stdio)

Any MCP-compatible client can connect to RavenClaws via stdio:

```json
// Request: List available tools
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "shell",
        "description": "Execute a shell command",
        "inputSchema": {
          "type": "object",
          "properties": {
            "command": { "type": "string" }
          }
        }
      },
      {
        "name": "read_file",
        "description": "Read a file from the filesystem",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": { "type": "string" }
          }
        }
      }
    ]
  }
}
```

### Security

When running in MCP server mode, all tool calls are still subject to:

- **PolicyEngine** — Deny-by-default allow-lists for shell, path, and network access
- **Sandbox** — Workdir jail with resource limits
- **AuditLog** — Tamper-evident logging of all operations

## Use Cases

### IDE Integration (Stdio)

Connect RavenClaws to VS Code via MCP for AI-assisted development:

```bash
# In VS Code MCP settings
{
  "mcp": {
    "servers": {
      "ravenclaws": {
        "command": "ravenclaws",
        "args": ["--mcp-server"]
      }
    }
  }
}
```

### IDE Integration (SSE)

Or connect via SSE transport for remote access:

```bash
# In VS Code MCP settings
{
  "mcp": {
    "servers": {
      "ravenclaws": {
        "url": "http://localhost:8081/sse"
      }
    }
  }
}
```

### Multi-Agent Workflows

Chain multiple RavenClaws instances together:

```bash
# Agent 1: MCP SSE server with filesystem access
ravenclaws --mcp-sse-server --mcp-sse-port 8081 --config agent1.toml &

# Agent 2: MCP client using Agent 1's tools via SSE
ravenclaws --exec "Analyze the codebase and suggest improvements"
```

Or use stdio transport for local-only workflows:

```bash
# Agent 1: MCP server with filesystem access
ravenclaws --mcp-server --config agent1.toml &

# Agent 2: MCP client using Agent 1's tools
ravenclaws --mcp-client "ravenclaws --mcp-server --config agent2.toml" \
  --exec "Analyze the codebase and suggest improvements"
```

## Best Practices

1. **Use allow-lists** — Configure `allowed_paths` and `allowed_shell_commands` in server mode
2. **Audit everything** — Always enable audit logging in production
3. **Sandbox untrusted code** — Enable sandbox for MCP server mode
4. **Limit exposure** — Only expose the tools you need
5. **Monitor** — Use health endpoints to verify MCP connections
