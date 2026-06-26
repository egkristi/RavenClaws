# RavenClaws Examples

This directory contains runnable example programs demonstrating RavenClaws library usage.

## Prerequisites

All examples require a configured LLM provider. Set up your config via:

1. A `ravenclaws.toml` file in the project root, or
2. Environment variables (prefixed with `RAVENCLAWS__`)

See the [Getting Started Guide](../docs/guides/getting-started.md) for details.

## Running Examples

```bash
# Run a specific example
cargo run --example basic_chat
cargo run --example agent_loop
cargo run --example swarm
cargo run --example mcp_client -- "npx @modelcontextprotocol/server-filesystem /tmp"
cargo run --example heartbeat
```

## Example Descriptions

| Example | File | Description |
|---------|------|-------------|
| `basic_chat` | `basic_chat.rs` | Minimal chat example — load config, create client, send message |
| `agent_loop` | `agent_loop.rs` | Full agent loop with tools, policy engine, and sandbox |
| `swarm` | `swarm.rs` | Multi-agent swarm with different personas |
| `mcp_client` | `mcp_client.rs` | Connect to an MCP server, discover and call tools |
| `heartbeat` | `heartbeat.rs` | Autonomous long-running agent with state persistence |

## Notes

- Examples use `ravenclaws` as a library crate (import via `use ravenclaws::...`)
- The `mcp_client` example requires an external MCP server command
- The `heartbeat` example runs 5 cycles with 60-second intervals
