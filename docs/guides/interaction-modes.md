# Interaction Modes

RavenClaws offers **17 distinct ways to interact** — from one-shot CLI commands to
long-running servers, autonomous agents, and library embedding. This guide covers
every mode with examples.

---

## 1. CLI Flags Overview

All modes are accessed via the `ravenclaws` binary:

```bash
ravenclaws [FLAGS] [OPTIONS]
```

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `-c`, `--config` | `RAVENCLAWS_CONFIG` | — | Config file path |
| `-v`, `--verbose` | `RAVENCLAWS_VERBOSE` | `false` | Enable debug logging |
| `--mode` | — | `single` | Agent mode: `single`, `swarm`, `supervisor`, `orchestrate`, `debate`, `review-loop`, `research-synthesize`, `voting` |
| `-e`, `--exec` | — | — | One-shot command prompt |
| `-R`, `--repl` | — | `false` | Interactive REPL |
| `--provider` | `RAVENCLAWS_PROVIDER` | — | Override provider |
| `--endpoint` | `RAVENCLAWS_ENDPOINT` | — | Override LLM endpoint |
| `--model` | `RAVENCLAWS_MODEL` | — | Override model name |
| `--system-prompt` | `RAVENCLAWS_SYSTEM_PROMPT` | — | Override system prompt |
| `--require-approval` | `RAVENCLAWS_REQUIRE_APPROVAL` | `false` | Human-in-the-loop for sensitive tools |
| `--max-iterations` | `RAVENCLAWS_MAX_ITERATIONS` | `10` | Max agent loop iterations |
| `--token-budget` | `RAVENCLAWS_TOKEN_BUDGET` | — | Max tokens per run |
| `--retry-max` | `RAVENCLAWS_RETRY_MAX` | `3` | Max retry attempts |
| `--retry-base-delay-ms` | `RAVENCLAWS_RETRY_BASE_DELAY` | `100` | Retry base delay (ms) |
| `--fallback-chain` | `RAVENCLAWS_FALLBACK_CHAIN` | — | Comma-separated provider fallback chain |
| `--mcp-command` | `RAVENCLAWS_MCP_COMMAND` | — | MCP server command (stdio) |
| `--mcp-args` | `RAVENCLAWS_MCP_ARGS` | — | MCP server arguments |
| `--mcp-env` | `RAVENCLAWS_MCP_ENV` | — | MCP server env vars (KEY=VALUE,...) |
| `--mcp-server` | `RAVENCLAWS_MCP_SERVER` | `false` | Run as MCP server (stdio) |
| `--mcp-sse-server` | `RAVENCLAWS_MCP_SSE_SERVER` | `false` | Run as MCP SSE server |
| `--mcp-sse-host` | `RAVENCLAWS_MCP_SSE_HOST` | `0.0.0.0` | MCP SSE server host |
| `--mcp-sse-port` | `RAVENCLAWS_MCP_SSE_PORT` | `8081` | MCP SSE server port |
| `--serve` | `RAVENCLAWS_SERVE` | `false` | Run as HTTP server |
| `--server-host` | `RAVENCLAWS_SERVER_HOST` | — | HTTP server host override |
| `--server-port` | `RAVENCLAWS_SERVER_PORT` | — | HTTP server port override |
| `--otel-endpoint` | `RAVENCLAWS_OTEL_ENDPOINT` | — | OTLP gRPC endpoint |
| `--otel-service-name` | `RAVENCLAWS_OTEL_SERVICE_NAME` | — | OTel service name |
| `--otel-disabled` | `RAVENCLAWS_OTEL_DISABLED` | `false` | Disable OTel tracing |
| `--background` | `RAVENCLAWS_BACKGROUND` | `false` | Submit background task |
| `--task-status` | `RAVENCLAWS_TASK_STATUS` | — | Check background task status |
| `--task-list` | `RAVENCLAWS_TASK_LIST` | `false` | List all background tasks |
| `--task-cancel` | `RAVENCLAWS_TASK_CANCEL` | — | Cancel a background task |
| `--task-resume` | `RAVENCLAWS_TASK_RESUME` | `false` | Resume incomplete tasks |
| `--scheduler` | `RAVENCLAWS_SCHEDULER` | `false` | Run scheduler with triggers |
| `--webhook-port` | `RAVENCLAWS_WEBHOOK_PORT` | `9090` | Webhook server port |
| `--eval` | `RAVENCLAWS_EVAL` | — | Run eval suite from config |
| `--eval-json` | `RAVENCLAWS_EVAL_JSON` | `false` | Output eval results as JSON |
| `--heartbeat` | `RAVENCLAWS_HEARTBEAT` | `false` | Autonomous heartbeat mode |
| `--heartbeat-goal` | `RAVENCLAWS_HEARTBEAT_GOAL` | — | Heartbeat goal prompt |
| `--heartbeat-tick-interval` | `RAVENCLAWS_HEARTBEAT_TICK_INTERVAL` | `300` | Heartbeat tick interval (s) |
| `--heartbeat-max-ticks` | `RAVENCLAWS_HEARTBEAT_MAX_TICKS` | `0` | Max heartbeat ticks (0=unlimited) |
| `--heartbeat-session` | `RAVENCLAWS_HEARTBEAT_SESSION` | — | Resume heartbeat session ID |
| `--swarm-topology` | `RAVENCLAWS_SWARM_TOPOLOGY` | `star` | Swarm topology |
| `--swarm-max-depth` | `RAVENCLAWS_SWARM_MAX_DEPTH` | `3` | Max swarm recursion depth |
| `--swarm-max-workers` | `RAVENCLAWS_SWARM_MAX_WORKERS` | `100` | Max swarm workers |
| `--swarm-dynamic-roles` | `RAVENCLAWS_SWARM_DYNAMIC_ROLES` | `false` | Enable dynamic role assignment |
| `--swarm-profiles` | `RAVENCLAWS_SWARM_PROFILES` | — | Worker profiles JSON file |
| `--swarm-communication` | `RAVENCLAWS_SWARM_COMMUNICATION` | `false` | Enable inter-agent communication |
| `--swarm-health-monitoring` | `RAVENCLAWS_SWARM_HEALTH_MONITORING` | `false` | Enable swarm health monitoring |
| `--no-final-required` | `RAVENCLAWS_NO_FINAL_REQUIRED` | `false` | Don't require FINAL: marker |
| `--require-final` | `RAVENCLAWS_REQUIRE_FINAL` | `false` | Require FINAL: marker |
| `-I`, `--image` | `RAVENCLAWS_IMAGE` | — | Attach image(s) to message |
| `--pattern-max-rounds` | `RAVENCLAWS_PATTERN_MAX_ROUNDS` | `3` | Max debate rounds |
| `--pattern-max-review` | `RAVENCLAWS_PATTERN_MAX_REVIEW` | `3` | Max review-loop iterations |
| `--pattern-research-agents` | `RAVENCLAWS_PATTERN_RESEARCH_AGENTS` | `3` | Number of research agents |
| `--pattern-voters` | `RAVENCLAWS_PATTERN_VOTERS` | `3` | Number of voters |
| `--pattern-verbose` | `RAVENCLAWS_PATTERN_VERBOSE` | `false` | Show verbose pattern results |

---

## 2. Agent Modes (`--mode`)

### 2a. Single Agent Mode (default)

One agent, one conversation. The simplest mode.

```bash
ravenclaws --mode single
ravenclaws  # same as above, "single" is the default
```

Reads a prompt from stdin, sends it to the LLM, prints the response.

### 2b. Swarm Mode

Multiple parallel agents with different personas working on the same task.

```bash
ravenclaws --mode swarm
ravenclaws --mode swarm --swarm-topology mesh --swarm-max-workers 50
```

Supports 4 topologies: `star`, `mesh`, `hierarchical`, `hybrid`.

### 2c. Supervisor Mode

A supervisor agent decomposes a task, spawns sub-agents, and aggregates results.

```bash
ravenclaws --mode supervisor
```

### 2d. Orchestrate Mode

Full swarm orchestration with self-provisioning sub-agents, dynamic role assignment,
and recursive supervision.

```bash
ravenclaws --mode orchestrate
ravenclaws --mode orchestrate --swarm-dynamic-roles --swarm-communication
```

---

## 3. Multi-Agent Patterns (`--mode`)

Four built-in collaboration strategies, available in both single-provider and
multi-model variants:

### 3a. Debate

Multiple agents debate a topic over several rounds, refining their positions.

```bash
ravenclaws --mode debate
ravenclaws --mode debate --pattern-max-rounds 5 --pattern-verbose
```

### 3b. Review Loop

An agent produces output, a reviewer critiques it, and the agent iterates.

```bash
ravenclaws --mode review-loop
ravenclaws --mode review-loop --pattern-max-review 5
```

### 3c. Research & Synthesize

Multiple research agents gather information, then a synthesizer combines findings.

```bash
ravenclaws --mode research-synthesize
ravenclaws --mode research-synthesize --pattern-research-agents 5
```

### 3d. Voting

Multiple agents vote on the best answer, with configurable voter count.

```bash
ravenclaws --mode voting
ravenclaws --mode voting --pattern-voters 5
```

---

## 4. One-Shot Execution (`--exec`)

Run a single prompt and print the response to stdout. Ideal for scripting and
piping:

```bash
ravenclaws --exec "What is the capital of France?"
echo "Summarize this text" | ravenclaws --exec
ravenclaws --exec "Analyze this image" --image photo.jpg
```

The agent loop runs with tool-use enabled, so the agent can call tools, browse
the web, execute shell commands, etc. — all in one shot.

---

## 5. Interactive REPL (`--repl`)

Full interactive conversation with streaming output:

```bash
ravenclaws --repl
ravenclaws --repl --image diagram.png
```

Type messages line by line. The agent responds with streaming token-by-token
output. Type `Ctrl+C` or `Ctrl+D` to exit.

---

## 6. HTTP Server (`--serve`)

Long-running HTTP server with REST API endpoints:

```bash
ravenclaws --serve
ravenclaws --serve --server-host 0.0.0.0 --server-port 8080
```

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Liveness probe — always 200 when server is running |
| `GET` | `/ready` | Readiness probe — 200 when initialized, 503 during startup |
| `GET` | `/metrics` | Prometheus-style metrics (requests, tokens, tool calls, errors, load) |
| `GET` | `/health/deep` | Deep health check — verifies LLM connectivity |
| `POST` | `/chat` | Send a message, get an agent response (JSON or SSE streaming) |
| `POST` | `/execute` | Submit a background task, returns task ID immediately |
| `GET` | `/tasks/{id}` | Poll background task status and result |
| `GET` | `/tools` | List available tools with JSON schemas |
| `GET` | `/tools/{name}` | Get details of a specific tool |
| `POST` | `/tools/{name}` | Execute a specific tool by name |
| `POST` | `/reload` | Reload configuration (distroless-friendly SIGHUP alternative) |

### Example Usage

```bash
# Chat
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!"}'

# Background task
curl -X POST http://localhost:8080/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Analyze this data"}'

# Check task status
curl http://localhost:8080/tasks/<task-id>

# List tools
curl http://localhost:8080/tools

# Execute a tool
curl -X POST http://localhost:8080/tools/web_fetch \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'

# Health check
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/health/deep

# Metrics
curl http://localhost:8080/metrics

# Reload config
curl -X POST http://localhost:8080/reload
```

---

## 7. MCP Server (`--mcp-server`)

Expose RavenClaws' built-in tools over stdio via the Model Context Protocol (MCP).
Any MCP client (Claude Desktop, VS Code, etc.) can discover and call RavenClaws tools.

```bash
ravenclaws --mcp-server
```

The server speaks JSON-RPC 2.0 over stdio, supporting:
- `initialize` — capability negotiation
- `tools/list` — discover available tools with schemas
- `tools/call` — execute a tool by name (policy-checked and audited)

---

## 8. MCP SSE Server (`--mcp-sse-server`)

Same as MCP server but over HTTP with Server-Sent Events (SSE) transport:

```bash
ravenclaws --mcp-sse-server
ravenclaws --mcp-sse-server --mcp-sse-host 0.0.0.0 --mcp-sse-port 8081
```

---

## 9. MCP Client (`--mcp-command`)

Connect to external MCP servers and register their tools into the agent's tool
registry. The agent can then call those tools alongside built-in tools.

```bash
ravenclaws --mcp-command "npx -y @modelcontextprotocol/server-filesystem /tmp"
ravenclaws --mcp-command "python mcp-server.py" --mcp-args "--port 9000"
ravenclaws --mcp-command "my-server" --mcp-env "API_KEY=sk-...,DEBUG=true"
```

Multiple MCP servers can also be configured in the TOML config file:

```toml
[mcp]
servers = [
  { name = "filesystem", command = "npx", args = ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"] },
  { name = "database", command = "python", args = ["mcp-server.py"], env = { DB_URL = "postgres://..." } },
]
```

---

## 10. Background Tasks (`--background`)

Submit a task and get an ID immediately. The task runs asynchronously and can be
polled later:

```bash
# Submit a task
TASK_ID=$(ravenclaws --background --exec "Analyze this data")
echo "Task ID: $TASK_ID"

# Check status
ravenclaws --task-status "$TASK_ID"

# List all tasks
ravenclaws --task-list

# Cancel a task
ravenclaws --task-cancel "$TASK_ID"

# Resume incomplete tasks on restart
ravenclaws --task-resume
```

Tasks persist to disk and survive process restarts.

---

## 11. Scheduler (`--scheduler`)

Run configured triggers (cron, webhook, file-watch) for proactive 24/7 agents:

```bash
ravenclaws --scheduler
ravenclaws --scheduler --webhook-port 9090
```

Configure triggers in `ravenclaws.toml`:

```toml
[scheduler]
triggers = [
  { type = "cron", schedule = "0 */6 * * *", prompt = "Daily system health check" },
  { type = "webhook", endpoint = "/webhook/github", prompt = "Process GitHub event" },
  { type = "file_watch", path = "/data/incoming", prompt = "Process new file" },
]
```

---

## 12. Heartbeat Mode (`--heartbeat`)

Autonomous long-running agent that runs a persistent assess→plan→act→persist→sleep
loop. The agent works independently over hours, days, or weeks:

```bash
ravenclaws --heartbeat --heartbeat-goal "Monitor system health and report anomalies"
ravenclaws --heartbeat \
  --heartbeat-goal "Watch for security threats" \
  --heartbeat-tick-interval 60 \
  --heartbeat-max-ticks 100

# Resume a previous session
ravenclaws --heartbeat --heartbeat-session "session-uuid"
```

State is persisted to disk and survives restarts.

---

## 13. Eval Mode (`--eval`)

Run evaluation suites to test agent quality and behavior:

```bash
ravenclaws --eval tests/eval/basic-suite.toml
ravenclaws --eval tests/eval/security-suite.toml --eval-json
```

Eval configs define assertions, run traces, and produce text or JSON reports.

---

## 14. Configuration File

RavenClaws uses TOML configuration files. By default it looks for `ravenclaws.toml`
in the current directory, or you can specify a path:

```bash
ravenclaws -c /etc/ravenclaws/config.toml
```

### Full Config Structure

```toml
[llm]
provider = "litellm"           # litellm, openrouter, ollama, openai, anthropic, openai-compatible, azure
endpoint = "http://localhost:4000"
model = "gpt-4o-mini"
api_key = ""                   # prefer env var: RAVENCLAWS__LLM__API_KEY
timeout_secs = 30
system_prompt = "You are RavenClaws..."
token_budget = 100000
retry_max = 3
retry_base_delay_ms = 100
retry_max_delay_ms = 10000

# Multi-model mode: define multiple providers
[[llms]]
provider = "openai"
endpoint = "https://api.openai.com"
model = "gpt-4o"

[[llms]]
provider = "anthropic"
endpoint = "https://api.anthropic.com"
model = "claude-3-opus"

[security]
require_tls = true
token_lifetime_secs = 3600
audit_log = true
prompt_injection_protection = true

[runtime]
workdir = "/tmp/ravenclaws-workdir"
max_agents = 10
health_interval_secs = 60
host = "0.0.0.0"
port = 8080
checkpoint_dir = "/data/checkpoints"
checkpoint_interval = 1

[telemetry]
otel_endpoint = "http://jaeger:4317"
otel_service_name = "ravenclaws"
otel_disabled = false

[web_search]
endpoint = "https://searx.be"
engine = "duckduckgo"
max_results = 5
fetch_content = true

[browser]
cdp_url = "http://127.0.0.1:9222"
request_timeout = 30000

[load]
max_concurrent_requests = 50
rate_limit_per_second = 100
rate_limit_burst = 200
overload_error_threshold = 50
overload_window_secs = 60
shed_load_at_queue_depth = 1000

[heartbeat]
goal = "Monitor system health"
tick_interval_secs = 300
max_iterations_per_tick = 10
workdir = "/tmp/ravenclaws-heartbeat"
enable_tools = true

[swarm]
topology = "star"
max_depth = 3
max_workers = 100
dynamic_role_assignment = false
enable_agent_communication = false
enable_health_monitoring = false

[mcp]
servers = [
  { name = "filesystem", command = "npx", args = ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"] },
]

[ravenfabric]
endpoint = "http://ravenfabric:8080"
agent_id = "ravenclaws-1"
remote_exec = true
```

---

## 15. Environment Variables

Every config field can be overridden via environment variables using the
`RAVENCLAWS__` prefix (double underscore for nested fields):

```bash
export RAVENCLAWS__LLM__PROVIDER=openai
export RAVENCLAWS__LLM__ENDPOINT=https://api.openai.com
export RAVENCLAWS__LLM__API_KEY=sk-...
export RAVENCLAWS__LLM__MODEL=gpt-4o
export RAVENCLAWS__SECURITY__REQUIRE_TLS=false
export RAVENCLAWS__SERVER__HOST=0.0.0.0
export RAVENCLAWS__SERVER__PORT=8080
```

CLI-specific env vars (see the table in §1) use a flat naming convention like
`RAVENCLAWS_PROVIDER`, `RAVENCLAWS_ENDPOINT`, etc.

---

## 16. Docker

### Production (distroless)

```bash
docker build -t ravenclaws:latest .
docker run --rm ravenclaws:latest --version
docker run --rm -p 8080:8080 ravenclaws:latest --serve
```

The production image uses `gcr.io/distroless/cc-debian12:nonroot` — no shell,
no package manager, runs as UID 65532.

### Development (with LiteLLM)

```bash
docker compose up
# RavenClaws at http://localhost:8080
# LiteLLM at http://localhost:4000
```

### Slim (with MCP client support)

```bash
docker build -f Dockerfile.slim -t ravenclaws:slim .
docker run --rm ravenclaws:slim --mcp-command "npx -y @modelcontextprotocol/server-filesystem /tmp"
```

The slim image is Debian-based and includes `nodejs`, `npm`, and `curl`.

---

## 17. Kubernetes

### Quick Deploy

```bash
kubectl apply -f k8s/deployment.yaml
```

### Helm Chart

```bash
helm install ravenclaws charts/ravenclaws/
```

The Helm chart supports 11 configurable resources including ConfigMap, Secrets,
Service, Ingress, NetworkPolicy, PDB, PVC, ServiceMonitor, and RBAC.

---

## 18. Library Usage

RavenClaws is available as a library crate on crates.io:

```toml
[dependencies]
ravenclaws = "1.1"
```

### Basic Chat

```rust
use ravenclaws::config::Config;
use ravenclaws::llm::{create_client, ChatMessage, LLMProviderTrait};

let config = Config::load(None)?;
let llm = create_client(&config.llm)?;
let response = llm.chat(vec![
    ChatMessage::new("user", "Hello!"),
]).await?;
println!("{}", response.choices[0].message.content);
```

### Agent Loop with Tools

```rust
use ravenclaws::config::Config;
use ravenclaws::llm::create_client;
use ravenclaws::agent::{run_agent_loop, AgentLoopConfig};
use ravenclaws::tools::ToolRegistry;

let config = Config::load(None)?;
let llm = create_client(&config.llm)?;
let tool_registry = ToolRegistry::with_config(&config);
let loop_config = AgentLoopConfig::default();
let response = run_agent_loop(
    llm,
    "Analyze this data",
    &config.llm.system_prompt,
    loop_config,
    None,
    Some(tool_registry),
).await?;
println!("{}", response);
```

### Swarm Orchestration

```rust
use ravenclaws::config::Config;
use ravenclaws::llm::create_client;
use ravenclaws::swarm::SwarmOrchestrator;

let config = Config::load(None)?;
let llm = create_client(&config.llm)?;
let mut orchestrator = SwarmOrchestrator::new(
    config.swarm.clone(),
    Some(llm),
    None,
    None,
);
orchestrator.init().await?;
let result = orchestrator.orchestrate("Complete this task").await?;
println!("{}", result);
```

### Full Examples

See the `examples/` directory for runnable programs:

```bash
cargo run --example basic_chat
cargo run --example agent_loop
cargo run --example swarm
cargo run --example mcp_client -- "npx @modelcontextprotocol/server-filesystem /tmp"
cargo run --example heartbeat
```

---

## 19. Multi-Model Mode

When multiple `[[llms]]` sections are defined in config, RavenClaws enters
multi-model mode. All agent modes (single, swarm, supervisor, debate, etc.)
have multi-model variants that round-robin across providers:

```bash
# Uses all configured providers in rotation
ravenclaws --mode single
ravenclaws --mode debate
```

Fallback chains are automatically built from the multi-model config — if one
provider fails, the next is tried.

---

## 20. Multi-Modal Input (`--image`)

Attach images to your message (supported formats: PNG, JPEG, GIF, WebP):

```bash
ravenclaws --exec "Describe this image" --image photo.jpg
ravenclaws --repl --image diagram.png --image chart.png
ravenclaws --exec "Compare these images" -I img1.jpg -I img2.jpg
```

Works in `--exec`, `--repl`, and HTTP server modes.

---

## 21. Graceful Shutdown

All long-running modes (server, heartbeat, scheduler, REPL) support graceful
shutdown via SIGTERM or SIGINT (`Ctrl+C`). The shutdown sequence:

1. Signal received → shutdown flag set
2. In-progress operations complete (with configurable timeout)
3. State is persisted (checkpoints, heartbeat state, background tasks)
4. Clean shutdown logged

---

## Quick Reference

```bash
# One-shot
ravenclaws --exec "Hello"

# Interactive
ravenclaws --repl

# HTTP server
ravenclaws --serve

# MCP server
ravenclaws --mcp-server

# Background task
ravenclaws --background --exec "Long task"

# Autonomous agent
ravenclaws --heartbeat --heartbeat-goal "Monitor system"

# Swarm
ravenclaws --mode swarm

# Multi-agent patterns
ravenclaws --mode debate
ravenclaws --mode review-loop
ravenclaws --mode research-synthesize
ravenclaws --mode voting

# Eval
ravenclaws --eval tests/eval/basic-suite.toml

# Scheduler
ravenclaws --scheduler

# With images
ravenclaws --exec "Describe" --image photo.jpg

# Docker
docker run --rm ravenclaws:latest --exec "Hello"

# Kubernetes
kubectl apply -f k8s/deployment.yaml
```
