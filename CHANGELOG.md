# Changelog

All notable changes to RavenClaw are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Async / long-horizon background runs** (`src/background.rs`) — assign-and-walk-away background task execution with disk persistence and resumability across restarts
  - `BackgroundTaskManager` — manages task lifecycle with in-memory index + JSON file persistence
  - `BackgroundTask` — full task struct with id, prompt, status, result, error, timestamps, provider/model metadata
  - `TaskStatus` — Pending → Running → Completed / Failed / Cancelled lifecycle
  - `--background` CLI flag — submit a task and return immediately (prints task ID to stdout)
  - `--task-status <id>` — check status and full details of a specific task
  - `--task-list` — list all tasks with status, creation time, and prompt preview
  - `--task-cancel <id>` — cancel a pending or running task
  - `--task-resume` — on startup, find and re-execute any incomplete tasks from disk
  - Tasks stored as individual JSON files in `<workdir>/tasks/` directory
  - 8 new unit tests covering creation, submission, status transitions, cancellation, listing, persistence across restarts, and error handling
  - 319 total unit tests (+8, 0 regressions)
- **Helm chart** (`charts/ravenclaw/`) — official Helm chart for deploying RavenClaw on Kubernetes
  - 11 configurable Kubernetes resources: ServiceAccount, ConfigMap, Secret, Deployment, Service, Ingress, RBAC (Role + RoleBinding), PersistentVolumeClaim, NetworkPolicy, PodDisruptionBudget, ServiceMonitor
  - Full values.yaml with sensible defaults matching existing `k8s/deployment.yaml`
  - Optional OpenTelemetry and RavenFabric configuration in ConfigMap
  - Prometheus ServiceMonitor support for metrics scraping
  - Helm chart validated with `helm lint` (0 failures)
- **Maintenance Cycle Workflow** in `AGENTS.md` — structured 7-phase SOP for every maintenance cycle: check CI, fix issues, verify on Orbstack, update docs, commit & push, verify CI after push, release if milestone reached.

## [0.7.2] — 2026-06-20

### Added
- **OpenTelemetry tracing** (`src/telemetry.rs`) — opt-in distributed tracing with OTLP exporter
  - `TelemetryConfig` with `--otel-endpoint`, `--otel-service-name`, `--otel-disabled` CLI flags
  - `TelemetryGuard` — flushes and shuts down OTel exporter on drop
  - gRPC OTLP exporter (default) and stdout exporter fallback
  - Feature-gated: `otel-grpc` (default), `otel-stdout` (optional)
  - `#[instrument]` spans on agent loop, HTTP server, tool execution, and LLM provider calls
  - 4 new unit tests covering config, disabled mode, guard drop, and custom settings

### Changed
- **Cargo.toml** — added `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `opentelemetry-stdout`, `tracing-opentelemetry` dependencies
- **Features** — `default = ["otel-grpc"]`, `otel-grpc = ["opentelemetry-otlp"]`, `otel-stdout = ["opentelemetry-stdout"]`
- **Config** — `Config.telemetry` field added with `TelemetryConfig` struct
- **311 unit tests** (+4 for telemetry, +0 regressions)

## [0.7.1] — 2026-02-06

### Added
- **HTTP Server Mode** (`src/server.rs`) — long-running server with health, readiness, and metrics endpoints
  - `GET /health` — liveness probe (always 200 OK)
  - `GET /ready` — readiness probe (200 OK when ready, 503 during startup)
  - `GET /metrics` — Prometheus-style metrics (requests, tokens, tool calls, errors, uptime)
  - `--serve` CLI flag to run in HTTP server mode
  - `--server-host` / `--server-port` CLI overrides
  - `runtime.host` / `runtime.port` config fields (default: `0.0.0.0:8080`)
  - Graceful shutdown on SIGTERM/SIGINT
  - 9 new unit tests covering health, readiness, metrics, uptime, HTTP responses, 404 handling

### Changed
- **k8s deployment** — switched from `--mode single` to `--serve` mode; probes now use HTTP `/health` and `/ready` endpoints instead of `--version` exec

## [0.7.0] — 2026-02-05

### Added
- **MCP Server** (`src/mcp.rs`) — expose RavenClaw's built-in tools over stdio via the Model Context Protocol
  - `McpServer` struct with `run()`, `handle_request()`, `handle_initialize()`, `handle_tools_list()`, `handle_tools_call()`
  - Supports `initialize`, `notifications/initialized`, `tools/list`, `tools/call` MCP methods
  - All tool calls policy-checked via `PolicyEngine` and logged to `AuditLog`
  - `--mcp-server` CLI flag to run in MCP server mode
  - 7 new unit tests covering initialization, tool listing, tool execution, error handling

### Changed
- **ROADMAP.md** — updated to v0.7.0 (MCP Server + Observability Foundations); MCP Server marked complete
- **Config** — `RuntimeConfig` now has `host` (Option<String>) and `port` (u16) fields; `Config` derives `Default`
- **Test count**: 291 → 307 (+7 MCP Server + 9 HTTP Server tests)

### Planned
- Agent communication — structured message passing; conflict resolution across agents (v0.6.2)
- OpenTelemetry tracing (v0.7)
- Prometheus metrics integration (v0.7)
- Human-in-the-loop approvals (v0.7)

## [v0.6.1] — 2026-06-19

### Added
- **RavenFabric client module** (`src/ravenfabric.rs`) — full HTTP client for RavenFabric REST API
  - `RavenFabricClient` struct with `new()`, `health()`, `list_agents()`, `execute()`, `broadcast()` methods
  - `ExecuteRequest` / `ExecuteResponse` / `RemoteAgent` types with serde serialization
  - `RavenFabricError` enum with `NotConfigured`, `ConnectionFailed`, `RequestFailed` variants
  - 12 unit tests covering: no-endpoint, with-endpoint, disabled config, error display, connection refused (3), serialization (2), deserialization (3)
- **RavenFabric wiring into all agent modes** — client initialized in `main.rs` from config, passed to all 6 agent mode functions (`run_single`, `run_swarm`, `run_supervisor`, `run_single_multi`, `run_swarm_multi`, `run_supervisor_multi`)
- **RavenFabric status logging** — each agent mode logs whether RavenFabric remote execution is available on startup

### Fixed
- **aarch64 build hanging in CI** — Cross-compilation step (`apt-get install gcc-aarch64-linux-gnu`) kept hanging indefinitely on x86_64 GitHub Actions runners. Switched to native `ubuntu-24.04-arm` runner for aarch64 builds, eliminating the need for cross-compilation entirely. This is faster and more reliable.
- **Duplicate "Fixed" section in CHANGELOG.md** — Removed duplicate entry for aarch64 build fix.

## [v0.6.0] — 2026-06-18

### Added
- **Swarm Mode (Single-Provider)** — parallel execution of 3 agents with different personas (analytical, creative, pragmatic); results collected with agent attribution; tokio task spawning for true parallelism
- **Supervisor Mode (Single-Provider)** — task decomposition into subtasks via LLM prompting; sub-agent spawning; result aggregation and final synthesis; security integration (PolicyEngine, Sandbox, AuditLog)
- **Swarm Mode (Multi-Model)** — parallel agents across different LLM providers; provider/model attribution; cost control (capped at 3 agents)
- **Supervisor Mode (Multi-Model)** — provider-aware task decomposition; round-robin supervisor LLM selection; subtask assignment to specific providers based on strengths
- **Git hooks system** — pre-commit and pre-push hooks for automated verification
  - `.githooks/pre-commit` — fast checks: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked`, binary size check, secrets scan
  - `.githooks/pre-push` — comprehensive checks: full pre-commit + release build + binary integrity + Docker build + security scan
  - `.githooks/setup.sh` — install/check/remove hooks with `git config core.hooksPath`
- **CI/CD hardening** — `DEBIAN_FRONTEND=noninteractive`, `-o Dpkg::Options::=--force-confdef`, `timeout-minutes: 20`, and retry logic (3 attempts) for cross-compilation dependency install
- **Node.js 24 migration** — `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` in all 3 workflow files
- **CodeQL v4 migration** — all `github/codeql-action/*` updated from `@v3` to `@v4`

### Fixed
- **Build Fixes After Upstream Merge (2026-06-02)** — 13+ compilation errors across 6 files resolved:
  - Merge artifacts in `src/main.rs` (duplicate `system_prompt`, stray brace, missing `warn` import, missing `LLMProvider::Anthropic` match arm)
  - Type/lifetime issues in `src/agent.rs` (`&str`/`String` mismatch, `tokio::spawn` lifetime, missing `.clone()` on `Arc`)
  - Formatting/borrow issues in `src/llm.rs`, `src/mcp.rs` (double borrow, moved field)
  - Missing config fields in `src/config.rs` (47+ test constructors updated)
  - MCP test assertion fix (`protocol_version` → `protocolVersion`)
  - Retry disabled in 7 error-path mockito tests
- **Exec mode test** — fixed `check_llm_response_quality` in `scripts/lib/common.sh` to detect agent loop progress instead of non-existent log message
- **apt-get hanging in CI** — `x86_64-unknown-linux-musl` build was getting stuck indefinitely; added `DEBIAN_FRONTEND=noninteractive` and `timeout-minutes`
- **aarch64-unknown-linux-gnu build timeout** — Build & Release #68 failed; added retry loop (3 attempts) and extended timeout to 20 minutes
- **22 pre-existing clippy dead_code warnings** — resolved by replacing deprecated struct usage in tests and adding `#[allow(dead_code)]` to intentionally unused types

### Changed
- Updated ROADMAP.md, ISSUES.md, README.md, AGENTS.md for v0.6 implementation status
- Increased LOC from ~8,900 to ~9,400 (+500 for v0.6 features)
- All 277+ unit tests passing across 9 source modules
- Binary size: ~3.4 MB (arm64 macOS release build)
- All modes use `FINAL:` marker detection for completion
- Supervisor modes support up to 15 iterations for complex task decomposition
- Subtask agents run with 5-iteration limit each
- Full security wiring (policy, sandbox, audit) preserved in supervisor mode

---

## [v0.5.3] — 2026-06-07

### Added
- Native Anthropic provider (`AnthropicClient`) with direct Claude API support
- Tool use support for Anthropic (native function calling format)
- Token tracking for Anthropic responses
- Unit tests for AnthropicClient (94 verification tests total)

### Changed
- Updated ROADMAP.md to mark v0.5.3 as complete

---

## [v0.5.2] — 2026-06-06

### Added
- MCP (Model Context Protocol) client with stdio transport
- MCP tool discovery and registration into ToolRegistry
- `run_agent_loop_with_mcp()` for MCP-integrated agent execution
- CLI flags: `--mcp-command`, `--mcp-args`, `--mcp-env`

### Changed
- Agent loop now supports both built-in and MCP-discovered tools

---

## [v0.5.1] — 2026-06-06

### Added
- Retry logic with exponential backoff and jitter
- Provider fallback chain with circuit breaker
- Token budget tracking (`TokenBudget` struct)
- Cost estimation for multi-provider runs

### Changed
- `LLMConfig` now includes `retry_max`, `retry_base_delay_ms`, `token_budget`

---

## [v0.5.0] — 2026-06-06

### Added
- Unified `OpenAICompatibleClient` for OpenAI, OpenRouter, Ollama, LiteLLM
- Eliminated code duplication across 4 provider clients
- Structured function calling (OpenAI Tools format)

### Changed
- Refactored `src/llm.rs` to use trait-based architecture
- All providers now implement `LLMProviderTrait`

---

## [v0.4.0] — 2026-06-03

### Added
- Security features wired to agent loop (commit `51e42b0`)
- `PolicyEngine` with deny-by-default tool validation
- `Sandbox` for shell execution isolation
- `AuditLog` with HMAC-SHA256 tamper-evident chaining
- Tool abstraction with registry and 4 built-in tools

### Changed
- Agent loop now enforces policy checks before tool execution
- All tool calls are audited with event types

---

## [v0.3.0] — 2026-05-28

### Added
- Agent loop with ReAct pattern (perceive→plan→act→observe)
- `--exec` one-shot mode with streaming
- Interactive REPL (`--repl`) with `/exit`, `/reset` commands
- Conversation memory with configurable max history
- System prompt / persona support

---

## [v0.2.0] — 2026-05-20

### Added
- RavenFabric verification (SHA256 checksums)
- Version wiring from Cargo.toml
- Cross-compilation fixes for ARM64 Linux
- Error propagation improvements

---

## [v0.1.0] — 2026-05-15

### Added
- Initial release
- Single agent mode (single-provider and multi-model)
- 5 LLM providers: LiteLLM, OpenAI, OpenRouter, Ollama, Anthropic (stub)
- CLI with config file and env-var overrides
- Container and Kubernetes manifests with security hardening
- CI/CD pipeline with security scanning
