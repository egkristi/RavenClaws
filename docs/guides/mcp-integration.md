# MCP Integration

RavenClaws supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) for connecting to external tools and services. This guide covers both MCP client and server modes.

## Overview

MCP provides a standardized way for AI agents to discover and use external tools. RavenClaws can:

- **Act as an MCP client** — Connect to MCP servers and use their tools
- **Act as an MCP server** — Expose RavenClaws' built-in tools to other MCP clients

## MCP Client Mode

In client mode, RavenClaws connects to one or more MCP servers and discovers their available tools. The agent can then use those tools during task execution.

### Starting an MCP Server

First, you need an MCP server running. Here's an example using the official MCP filesystem server:

```bash
npx @modelcontextprotocol/server-filesystem /path/to/allowed/directory
```

### Connecting RavenClaws as a Client

```bash
# Connect to an MCP server via stdio
ravenclaws --mcp-client "npx @modelcontextprotocol/server-filesystem /tmp" --exec "List files in the current directory"
```

### Configuration

You can configure MCP servers in your config file:

```toml
[mcp]
servers = [
  { command = "npx", args = ["@modelcontextprotocol/server-filesystem", "/tmp"], name = "filesystem" },
  { command = "npx", args = ["@modelcontextprotocol/server-github"], name = "github" },
]
```

Then run:

```bash
ravenclaws --exec "Find all Rust files in /tmp and count them"
```

### How It Works

1. RavenClaws spawns the MCP server process
2. Discovers available tools via the `tools/list` endpoint
3. Registers discovered tools in the `ToolRegistry`
4. When the agent calls a tool, RavenClaws sends a `tools/call` request
5. Results are returned to the agent for further processing

## MCP Server Mode

In server mode, RavenClaws exposes its built-in tools (shell, read/write file, web fetch, web search) as MCP tools that other AI agents can discover and use.

### Starting the MCP Server

```bash
ravenclaws --mcp-server
```

This starts RavenClaws in MCP server mode, listening on stdio for JSON-RPC 2.0 requests.

### Connecting from Another Agent

Any MCP-compatible client can connect to RavenClaws:

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

### IDE Integration

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

### Multi-Agent Workflows

Chain multiple RavenClaws instances together:

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
