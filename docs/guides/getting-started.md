# Getting Started with RavenClaws

This guide walks you through your first RavenClaws agent, from installation to running a multi-step task.

## Prerequisites

- **Rust 1.86+** (for building from source)
- **Docker** (optional, for containerized deployment)
- **An LLM provider** — one of:
  - [LiteLLM](https://litellm.ai/) (recommended for local development)
  - [OpenAI](https://platform.openai.com/)
  - [OpenRouter](https://openrouter.ai/)
  - [Ollama](https://ollama.ai/) (fully local, no API key needed)
  - [Anthropic](https://console.anthropic.com/)

## Installation

### Option 1: Install via Cargo (recommended)

```bash
cargo install ravenclaws
```

### Option 2: Build from source

```bash
git clone https://github.com/egkristi/RavenClaws.git
cd RavenClaws
cargo build --release
# Binary is at target/release/ravenclaws
```

### Option 3: Docker

```bash
docker pull ghcr.io/egkristi/ravenclaws:latest
```

## Quick Start: Your First Agent

### 1. Set up your LLM provider

Set the required environment variables:

```bash
# For LiteLLM (default):
export LITELLM_API_KEY="your-key"
export LITELLM_ENDPOINT="http://localhost:4000"

# Or for OpenAI:
export OPENAI_API_KEY="sk-..."
export RAVENCLAWS__LLM__PROVIDER="openai"

# Or for Ollama (fully local):
export RAVENCLAWS__LLM__PROVIDER="ollama"
export RAVENCLAWS__LLM__ENDPOINT="http://localhost:11434"
export RAVENCLAWS__LLM__MODEL="llama3.1"
```

### 2. Run a one-shot task

```bash
ravenclaws --exec "What is the capital of France?"
```

You should see a response like: `The capital of France is Paris.`

### 3. Start an interactive REPL

```bash
ravenclaws --repl
```

This starts an interactive session where you can have a back-and-forth conversation:

```
╭─ RavenClaws REPL ─────────────────────────────╮
│ Type /exit to quit, /reset to clear history    │
╰────────────────────────────────────────────────╯
You: Summarize the key features of Rust
RavenClaws: Rust is a systems programming language...
You: /exit
Goodbye!
```

### 4. Run a multi-step task with tools

```bash
ravenclaws --exec "Create a file called hello.txt with the text 'Hello, RavenClaws!' and then read it back"
```

This uses the built-in `write_file` and `read_file` tools to complete the task.

## Configuration

RavenClaws can be configured via three layers (each overrides the previous):

1. **Config file** (`ravenclaws.toml`)
2. **Environment variables** (prefixed with `RAVENCLAWS__`)
3. **CLI flags**

### Minimal config file

```toml
[llm]
provider = "litellm"
endpoint = "http://localhost:4000"
api_key = "your-key"
model = "gpt-4o-mini"
```

### Full config file

See the [Configuration Reference](./configuration.md) for all available options.

## Next Steps

- [Configuration Reference](./configuration.md) — all config options
- [Swarm Mode Guide](./swarm-mode.md) — multi-agent orchestration
- [MCP Integration](./mcp-integration.md) — connect to MCP servers
- [Heartbeat Mode](./heartbeat-mode.md) — autonomous long-running agents
- [Migration Guide](../../MIGRATION.md) — upgrading between versions
