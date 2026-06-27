# Configuration Reference

RavenClaws uses a layered configuration system. Each layer overrides the previous one:

1. **Default values** (built into the binary)
2. **Config file** (`ravenclaws.toml` in the current directory, or specified via `--config`)
3. **Environment variables** (prefixed with `RAVENCLAWS__`, using `__` as separator)
4. **CLI flags** (highest priority)

## Config File Location

By default, RavenClaws looks for `ravenclaws.toml` in the current directory. You can specify a different path:

```bash
ravenclaws --config /path/to/config.toml
```

## Environment Variable Format

Environment variables use the pattern `RAVENCLAWS__SECTION__KEY`. For nested keys, use additional `__` separators:

```bash
export RAVENCLAWS__LLM__PROVIDER="openai"
export RAVENCLAWS__LLM__MODEL="gpt-4o"
export RAVENCLAWS__LLM__ENDPOINT="https://api.openai.com/v1"
export RAVENCLAWS__LLM__API_KEY="sk-..."
export RAVENCLAWS__LLM__MAX_TOKENS="4096"
export RAVENCLAWS__LLM__TEMPERATURE="0.7"
export RAVENCLAWS__LLM__SYSTEM_PROMPT="You are a helpful assistant."
```

## Full Configuration

### `[llm]` — LLM Provider Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `provider` | string | `"litellm"` | LLM provider: `litellm`, `openai`, `openrouter`, `ollama`, `anthropic`, `openai-compatible` |
| `endpoint` | string | — | API endpoint URL |
| `api_key` | string | — | API key (prefer env var `RAVENCLAWS__LLM__API_KEY`) |
| `model` | string | `"gpt-4o-mini"` | Model name |
| `max_tokens` | integer | `4096` | Maximum tokens in response |
| `temperature` | float | `0.7` | Response temperature (0.0–2.0) |
| `system_prompt` | string | — | System prompt / persona |
| `max_history` | integer | `50` | Max conversation turns to retain |

### `[runtime]` — Runtime Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_iterations` | integer | `25` | Max agent loop iterations |
| `request_timeout_secs` | integer | `120` | LLM request timeout |
| `sandbox_dir` | string | `"/tmp/ravenclaws"` | Sandbox working directory |
| `audit_log_path` | string | `"audit.log"` | Audit log file path |
| `policy_file` | string | — | Policy allow-list file |

### `[security]` — Security Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `allowed_shell_commands` | array | `[]` | Allowed shell commands (empty = deny all) |
| `allowed_paths` | array | `[]` | Allowed file system paths |
| `allowed_domains` | array | `[]` | Allowed network domains |
| `sandbox_enabled` | boolean | `true` | Enable sandboxed execution |

### `[swarm]` — Swarm Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_workers` | integer | `100` | Maximum number of swarm workers |
| `profiles` | array | — | Worker personality profiles (array of tables) |
| `topology` | string | `"star"` | Swarm topology: `star`, `mesh`, `hierarchical`, `hybrid` |

### `[heartbeat]` — Heartbeat Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `goal` | string | — | **Required.** Agent's autonomous mission prompt |
| `tick_interval_secs` | integer | `300` | Sleep interval between cycles (seconds) |
| `max_ticks` | integer | `0` | Max ticks (0 = unlimited) |
| `max_iterations_per_tick` | integer | `5` | Max agent loop iterations per tick |
| `enable_tools` | boolean | `true` | Enable tool calling during heartbeat ticks |
| `workdir` | string | `"/workspace"` | Working directory for state persistence |

### `[background]` — Background Task Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `tasks_dir` | string | `"background-tasks"` | Task persistence directory |
| `max_concurrent` | integer | `10` | Max concurrent background tasks |

### `[scheduler]` — Scheduler Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `triggers` | array | `[]` | Trigger configurations (cron, webhook, file-watch) |

### `[server]` — HTTP Server Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `host` | string | `"0.0.0.0"` | Server bind address |
| `port` | integer | `8080` | Server port |

### `[telemetry]` — OpenTelemetry Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `otel_disabled` | boolean | `true` | Disable OpenTelemetry tracing (opt-in) |
| `otel_endpoint` | string | `"http://localhost:4317"` | OTLP collector endpoint |
| `otel_service_name` | string | `"ravenclaws"` | Service name for traces |

## Example Configurations

### Minimal (LiteLLM)

```toml
[llm]
provider = "litellm"
endpoint = "http://localhost:4000"
api_key = "sk-litellm-key"
model = "gpt-4o-mini"
```

### Multi-provider (fallback chain)

```toml
[llm]
provider = "multi"
model = "gpt-4o-mini"

[llm.providers.litellm]
endpoint = "http://localhost:4000"
api_key = "${LITELLM_API_KEY}"

[llm.providers.openai]
endpoint = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
```

### Secure sandbox with policy

```toml
[llm]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.1"

[runtime]
sandbox_dir = "/var/lib/ravenclaws/sandbox"
audit_log_path = "/var/log/ravenclaws/audit.log"

[security]
allowed_shell_commands = ["ls", "cat", "grep", "find"]
allowed_paths = ["/var/lib/ravenclaws/data"]
allowed_domains = ["api.github.com"]
sandbox_enabled = true
```

### Swarm mode

```toml
[llm]
provider = "openai"
api_key = "${OPENAI_API_KEY}"
model = "gpt-4o"

[swarm]
agent_count = 3
topology = "flat"

[swarm.profiles]
coder = "You are an expert software engineer."
researcher = "You are a thorough research analyst."
reviewer = "You are a meticulous code reviewer."
```

## CLI Flags

| Flag | Description |
|------|-------------|
| `--config <path>` | Config file path |
| `-m, --mode <mode>` | Agent mode: `single`, `swarm`, or `supervisor` (default: `single`) |
| `-e, --exec <prompt>` | One-shot execution mode |
| `-R, --repl` | Interactive REPL mode |
| `--serve` | HTTP server mode (long-running with `/health`, `/ready`, `/metrics`) |
| `--mcp-server` | MCP server mode (expose tools over stdio via MCP protocol) |
| `--mcp-command <cmd>` | MCP client command (stdio transport, e.g. `npx -y @modelcontextprotocol/server-filesystem`) |
| `--mcp-args <args>` | MCP client arguments (space-separated) |
| `--mcp-env <vars>` | MCP client environment variables (KEY=VALUE, comma-separated) |
| `--heartbeat` | Autonomous heartbeat mode (persistent assess→plan→act→sleep loop) |
| `--heartbeat-goal <text>` | Goal prompt for heartbeat mode |
| `--heartbeat-tick-interval <secs>` | Tick interval in seconds (default: 300) |
| `--heartbeat-max-ticks <N>` | Maximum ticks (0 = unlimited) |
| `--heartbeat-session <id>` | Heartbeat session ID for resuming |
| `--swarm-topology <type>` | Swarm topology: `star`, `mesh`, `hierarchical`, `hybrid` (default: `star`) |
| `--swarm-max-depth <N>` | Maximum recursion depth for hierarchical swarm (default: 3) |
| `--swarm-max-workers <N>` | Maximum workers in the swarm (default: 100) |
| `--swarm-dynamic-roles` | Enable dynamic role assignment in swarm mode |
| `--swarm-profiles <path>` | Worker profiles file path (JSON) |
| `--swarm-communication` | Enable inter-agent communication in swarm mode |
| `--swarm-health-monitoring` | Enable swarm health monitoring |
| `--background` | Submit a background task and return immediately |
| `--task-status <id>` | Check status of a background task |
| `--task-list` | List all background tasks |
| `--task-cancel <id>` | Cancel a background task |
| `--task-resume` | Resume incomplete background tasks on startup |
| `--scheduler` | Run the scheduler with configured triggers (cron, webhook, file-watch) |
| `--eval <path>` | Run eval suite from config file |
| `--eval-json` | Output eval results as JSON |
| `--provider <name>` | Override LLM provider: `litellm`, `openai`, `openrouter`, `ollama`, `anthropic` |
| `--endpoint <url>` | Override LLM endpoint URL |
| `--model <name>` | Override model name |
| `--system-prompt <text>` | Override system prompt |
| `--require-approval` | Require human approval for sensitive tool calls (HITL) |
| `--max-iterations <N>` | Maximum iterations for the agent loop (default: 10) |
| `--token-budget <N>` | Token budget per run (stops when exceeded) |
| `--retry-max <N>` | Retry max attempts (default: 3) |
| `--retry-base-delay-ms <N>` | Retry base delay in ms (default: 100) |
| `--fallback-chain <providers>` | Enable provider fallback chain (comma-separated) |
| `--server-host <host>` | HTTP server host (overrides config) |
| `--server-port <port>` | HTTP server port (overrides config) |
| `--webhook-port <port>` | Webhook server port (default: 9090) |
| `--otel-endpoint <url>` | OpenTelemetry OTLP gRPC endpoint |
| `--otel-service-name <name>` | OpenTelemetry service name |
| `--otel-disabled` | Disable OpenTelemetry tracing |
| `-v, --verbose` | Enable verbose logging |
| `-V, --version` | Print version and exit |
| `-h, --help` | Print help and exit |
