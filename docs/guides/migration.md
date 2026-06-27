# Migration Guide

This document describes breaking changes and migration paths between RavenClaws
versions. RavenClaws follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
— breaking changes only occur in major versions (v0.x → v1.0, v1.0 → v2.0, etc.).

For pre-1.0 versions (v0.x), breaking changes may occur in minor versions but
are documented here with migration instructions.

---

## v0.9 → v1.0

### Summary

v1.0 is the first stable release. The public API is now covered by semver
guarantees. Key changes:

- `#[non_exhaustive]` on all public enums and structs
- Deprecated types removed
- Library crate (`ravenclaws`) stabilized

### Breaking Changes

#### 1. `#[non_exhaustive]` on public types

All public enums and structs now have `#[non_exhaustive]`. This means:

- **Enums**: Match statements must include a wildcard arm (`_ => ...`).
- **Structs**: Cannot be constructed with struct literal syntax outside the crate.
  Use the provided `::new()` or `::default()` methods instead.

**Affected types:**

| Type | Kind | Migration |
|---|---|---|
| `RavenClawsError` | Enum | Add `_ => ...` to match arms |
| `ConfigError` | Enum | Add `_ => ...` to match arms |
| `LLMError` | Enum | Add `_ => ...` to match arms |
| `ToolError` | Enum | Add `_ => ...` to match arms |
| `LLMProvider` | Enum | Add `_ => ...` to match arms |
| `OpenAICompatibleProvider` | Enum | Add `_ => ...` to match arms |
| `CircuitState` | Enum | Add `_ => ...` to match arms |
| `ToolCategory` | Enum | Add `_ => ...` to match arms |
| `Config` | Struct | Use `Config::load()` or `Config::default()` |
| `LLMConfig` | Struct | Use `LLMConfig::new()` or `LLMConfig::default()` |
| `SecurityConfig` | Struct | Use `SecurityConfig::default()` |
| `RuntimeConfig` | Struct | Use `RuntimeConfig::default()` |
| `RavenFabricConfig` | Struct | Use `RavenFabricConfig::default()` |
| `TelemetryConfig` | Struct | Use `TelemetryConfig::default()` |
| `SchedulerConfig` | Struct | Use `SchedulerConfig::default()` |
| `WebSearchConfig` | Struct | Use `WebSearchConfig::default()` |

**Before (v0.9):**
```rust
match error {
    RavenClawsError::Config(e) => ...,
    RavenClawsError::LLM(e) => ...,
    RavenClawsError::Tool(e) => ...,
    RavenClawsError::Network(e) => ...,
    RavenClawsError::Io(e) => ...,
    RavenClawsError::SecurityViolation => ...,
    RavenClawsError::RavenFabric(e) => ...,
}
```

**After (v1.0):**
```rust
match error {
    RavenClawsError::Config(e) => ...,
    RavenClawsError::LLM(e) => ...,
    RavenClawsError::Tool(e) => ...,
    RavenClawsError::Network(e) => ...,
    RavenClawsError::Io(e) => ...,
    RavenClawsError::SecurityViolation => ...,
    RavenClawsError::RavenFabric(e) => ...,
    _ => ...,  // future-proof
}
```

#### 2. Deprecated LLM client types removed

The following types (deprecated since v0.5.0) have been removed:

| Removed Type | Replacement |
|---|---|
| `LiteLLMClient` | `OpenAICompatibleClient::new(OpenAICompatibleProvider::LiteLLM, ...)` |
| `OpenRouterClient` | `OpenAICompatibleClient::new(OpenAICompatibleProvider::OpenRouter, ...)` |
| `OpenAIClient` | `OpenAICompatibleClient::new(OpenAICompatibleProvider::OpenAI, ...)` |

**Before (v0.9):**
```rust
use ravenclaws::LiteLLMClient;
let client = LiteLLMClient::new(config);
```

**After (v1.0):**
```rust
use ravenclaws::{OpenAICompatibleClient, OpenAICompatibleProvider};
let client = OpenAICompatibleClient::new(
    OpenAICompatibleProvider::LiteLLM,
    config,
);
```

#### 3. `execute_tool_call` removed

The legacy `execute_tool_call` function (deprecated since v0.4) has been removed.
Use `execute_tool_call_with_security` instead, which integrates with
`PolicyEngine`, `Sandbox`, and `AuditLog`.

**Before (v0.9):**
```rust
let result = execute_tool_call(&tool_call, &tool_registry).await;
```

**After (v1.0):**
```rust
let result = execute_tool_call_with_security(
    &tool_call,
    &tool_registry,
    &policy_engine,
    &sandbox,
    &audit_log,
).await;
```

#### 4. `run_exec_stream` removed

The unused `run_exec_stream` function has been removed. Streaming exec
functionality is handled internally by the agent loop.

---

## v0.9.1 → v0.9.2

### Summary

v0.9.2 introduced the **inter-agent communication bus** and **swarm health
monitoring** — two major additions to the swarm orchestration system. These
additions are opt-in and do not break existing swarm configurations.

### New Features (No Breaking Changes)

#### 1. Inter-agent Communication Bus

Swarm agents can now communicate via a shared message bus. This is disabled by
default — enable it with `--swarm-communication` or `RAVENCLAW_SWARM_COMMUNICATION=true`.

**New types (public API):**

| Type | Description |
|---|---|
| `AgentMessage` | Message struct with UUID, sender, recipient, type, content, timestamp |
| `MessageType` | Enum: Information, Question, Result, Error, Coordination, Generic |
| `AgentMessageBus` | Shared bus with send, receive, filter, and broadcast |

**Usage:**
```rust
use ravenclaws::{AgentMessageBus, MessageType};

let bus = AgentMessageBus::new();
bus.send(AgentMessage::new(
    "worker-1", "worker-2",
    MessageType::Information,
    "Task completed successfully",
));
let messages = bus.receive("worker-2", None);
```

#### 2. Swarm Health Monitoring

Health monitoring tracks per-worker heartbeats and detects degraded/unhealthy/dead
agents. Disabled by default — enable with `--swarm-health-monitoring` or
`RAVENCLAW_SWARM_HEALTH_MONITORING=true`.

**New types (public API):**

| Type | Description |
|---|---|
| `SwarmHealthMonitor` | Tracks heartbeats, detects failures, identifies replacements |
| `WorkerHealthStatus` | Enum: Healthy, Degraded, Unhealthy, Dead |
| `WorkerTelemetry` | Per-worker metrics (tasks, errors, duration, messages) |
| `SwarmMetrics` | Aggregate swarm health metrics |

**Configuration (in `ravenclaws.toml`):**
```toml
[swarm]
communication_enabled = true
health_monitoring_enabled = true
heartbeat_interval_secs = 5
max_missed_beats = 3
replacement_timeout_secs = 30
```

#### 3. `SwarmOrchestrator::new_with_bus()`

A new constructor that accepts a shared `AgentMessageBus` and `SwarmHealthMonitor`
for use across sub-orchestrators in recursive supervision:

```rust
let bus = Arc::new(AgentMessageBus::new());
let health_monitor = Arc::new(RwLock::new(SwarmHealthMonitor::new(5, 3, 30)));
let orchestrator = SwarmOrchestrator::new_with_bus(
    config, llm_manager, bus, health_monitor,
);
```

### Migration Steps

1. No code changes required — all new features are opt-in
2. To enable communication: add `--swarm-communication` to CLI or set
   `RAVENCLAW_SWARM_COMMUNICATION=true`
3. To enable health monitoring: add `--swarm-health-monitoring` to CLI or set
   `RAVENCLAW_SWARM_HEALTH_MONITORING=true`
4. To configure both: add `[swarm]` section to `ravenclaws.toml` (see above)

---

## v0.8 → v0.9

### Breaking Changes

#### 1. `MultiModelManager` now implements `Clone`

`MultiModelManager` now implements `Clone` to support sub-orchestrator spawning
in swarm mode. If you were storing `MultiModelManager` in a context that doesn't
support `Clone`, wrap it in `Arc` instead.

#### 2. New `[swarm]` config section

Swarm orchestration configuration is now under `[swarm]` in `ravenclaws.toml`:

```toml
[swarm]
max_depth = 3
max_workers = 100
topology = "hierarchical"
dynamic_role_assignment = true
```

#### 3. New `[heartbeat]` config section

Heartbeat agent configuration is now under `[heartbeat]`:

```toml
[heartbeat]
tick_interval_secs = 60
max_ticks = 0  # 0 = unlimited
goal = "Monitor system health"
```

---

## v0.7 → v0.8

### Breaking Changes

#### 1. `BackgroundTaskManager` API

The background task manager now persists tasks to disk. The API has changed:

**Before (v0.7):**
```rust
let manager = BackgroundTaskManager::new();
let id = manager.spawn(task).await;
```

**After (v0.8):**
```rust
let manager = BackgroundTaskManager::new(workdir);
let id = manager.spawn(task).await;
// Tasks are persisted to <workdir>/tasks/<id>.json
```

#### 2. `--require-approval` flag

The `--require-approval` flag now gates sensitive tool calls. When set, the
agent prompts for approval before executing tools in the `requires_approval`
category. See `--help` for details.

#### 3. `zeroize` integration

API keys in `LLMConfig` and HMAC secret keys in `AuditLog` are now zeroized on
drop. This is transparent to most users but may affect debugging if you were
inspecting key values after use.

---

## v0.6 → v0.7

### Breaking Changes

#### 1. MCP Server mode (`--mcp-server`)

The `--mcp-server` flag runs RavenClaws as an MCP server over stdio. In this
mode, the binary does not accept prompts — it exposes tools via the MCP protocol.

#### 2. HTTP Server mode (`--serve`)

The `--serve` flag starts a long-running HTTP server with `/health`, `/ready`,
and `/metrics` endpoints. This is the recommended deployment mode for Kubernetes.

#### 3. OpenTelemetry tracing (opt-in)

Tracing is disabled by default. Enable with `--otel-endpoint` or
`RAVENCLAWS_OTEL_ENDPOINT`. The `otel-grpc` feature is enabled by default;
`otel-stdout` is optional.

#### 4. Helm chart

The official Helm chart is at `charts/ravenclaws/`. See `charts/ravenclaws/values.yaml`
for configuration options.

---

## v0.5 → v0.6

### Breaking Changes

#### 1. Swarm and Supervisor modes

Swarm and supervisor modes are now fully implemented. The `--mode swarm` and
`--mode supervisor` flags now execute real multi-agent workflows instead of
returning "not yet implemented" errors.

#### 2. RavenFabric integration

RavenFabric is now wired into all agent modes. If you have `[ravenfabric]`
configuration, it will be used for remote agent execution. Set `enabled = false`
to disable.

---

## v0.4 → v0.5

### Breaking Changes

#### 1. Unified OpenAI-Compatible Client

`LiteLLMClient`, `OpenRouterClient`, and `OpenAIClient` are deprecated. Use
`OpenAICompatibleClient` with the appropriate `OpenAICompatibleProvider` variant.

**Before (v0.4):**
```rust
let client = LiteLLMClient::new(config);
```

**After (v0.5):**
```rust
let client = OpenAICompatibleClient::new(
    OpenAICompatibleProvider::LiteLLM,
    config,
);
```

#### 2. Retry and Fallback

The `LLMConfig` now includes retry and fallback settings:

```toml
[llm]
retry_max_attempts = 3
retry_base_delay_ms = 100
retry_max_delay_ms = 10000
fallback_provider = "ollama"
```

#### 3. Token Budget

Use `--token-budget <N>` or `RAVENCLAW_TOKEN_BUDGET` to set a token budget.
The agent will stop when the budget is consumed.

---

## v0.3 → v0.4

### Breaking Changes

#### 1. Tool system

The tool system is now based on structured function calling (OpenAI Tools format)
instead of pattern-matching `TOOL_CALL:` / `ARGS:` in the prompt. If you were
manually constructing tool call prompts, switch to the `ToolRegistry` API.

#### 2. Security infrastructure

`PolicyEngine`, `Sandbox`, and `AuditLog` are now wired to the agent loop.
All tool calls are validated against policy before execution. If you were running
without security, you may need to configure allow-lists:

```toml
[security.policy.shell]
allow_commands = ["echo", "ls", "cat", "pwd", "whoami"]

[security.policy.path]
allow_read = ["/app", "/tmp", "/home"]
allow_write = ["/tmp"]
```

---

## v0.2 → v0.3

### Breaking Changes

#### 1. Agent loop

The agent now uses a perceive → plan → act → observe loop. The `--exec` flag
runs a one-shot task through the loop. The `--repl` flag starts an interactive
session.

#### 2. Conversation memory

Conversation history is now tracked across turns. Use `--max-history <N>` to
control the number of turns retained.

---

## v0.1 → v0.2

### Breaking Changes

#### 1. `Cargo.lock` committed

`Cargo.lock` is now committed to the repository. All CI commands use `--locked`
for reproducible builds. If you were ignoring `Cargo.lock`, update your workflow.

#### 2. Multi-arch Docker builds

Docker builds now support `linux/amd64` and `linux/arm64`. Cross-compilation
dependencies are installed automatically.

---

## Appendix: Feature Detection

To check which version of RavenClaws you're running:

```bash
ravenclaws --version
```

To check for available features at compile time:

```rust
#[cfg(feature = "otel-grpc")]
println!("OpenTelemetry gRPC tracing enabled");
```

To check the library version in `Cargo.toml`:

```toml
[dependencies]
ravenclaws = "1.0"
```
