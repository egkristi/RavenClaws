# Changelog

All notable changes to RavenClaws are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **`--no-final-required` CLI flag** — New `AgentLoopConfig.no_final_required` field and `--no-final-required` CLI flag. When set, any non-tool-call LLM response is treated as completion (no `FINAL:` marker required). Wired to both `run_agent_loop` and `run_agent_loop_with_mcp`. Useful for models that don't reliably emit `FINAL:` convention.
- **Agent loop response logging** — Added `debug!` log after each LLM response in both agent loops, showing response length and first 500 characters of content. Helps diagnose silent failures and model behavior.
- **Default system prompt `FINAL:` instructions** — Updated `default_system_prompt()` in `config.rs` to include `FINAL:` usage instructions, guiding models to signal task completion.
- **`agent_count` serde alias** — Added `#[serde(alias = "agent_count")]` to `SwarmConfig.max_workers` for backward compatibility with configs using the old field name.

### Fixed
- **Heartbeat goal error message** — Improved error message when no goal is provided for heartbeat mode. Now includes a concrete example: `--heartbeat-goal "Monitor system health and report anomalies"`.
- **`--provider anthropic` CLI flag** — Anthropic provider is now selectable via `--provider anthropic` (previously fell through to LiteLLM).
- **`--webhook-port` CLI flag** — Scheduler now respects the `--webhook-port` value instead of hardcoding port 9090.
- **Audit log mutex poisoning** — Replaced 7 `unwrap()` calls on `self.entries.lock()` with proper error handling via `lock_entries()` helper. Mutex poisoning no longer panics the audit log hot path.
- **README env var prefix** — Fixed `RAVENCLAW__` → `RAVENCLAWS__` (missing final S) in Quick Start, Docker, and env var table sections.
- **README `--mode single` reference** — Quick Start now uses `--exec` instead of `--mode single`.

### Changed
- **Provider count updated** — Landing page, docs, and metadata updated from 5 to 6 LLM providers.
- **Provider Strategy section updated** — ROADMAP.md Provider Strategy updated to reflect implemented `openai-compatible` provider.

### Added
- **MCP SSE transport** — Full SSE transport implementation for the MCP protocol. Client-side: `McpTransportConfig::Sse` connects to SSE endpoints, receives `endpoint` events, and sends JSON-RPC requests via HTTP POST. Server-side: `McpSseServer` provides `GET /sse` (SSE stream) and `POST /message` (JSON-RPC) endpoints with policy enforcement and audit logging. 7 tests covering server creation, initialization, tools/list, tools/call, error handling, config serialization, and connection failure.
- **Generic `openai-compatible` provider** — New `LLMProvider::OpenAICompatible` variant unlocks vLLM, llama.cpp, LM Studio, TGI, Groq, Together AI, Fireworks, DeepInfra, and any custom OpenAI-compatible endpoint. Config: `provider = "openai-compatible"`, CLI: `--provider openai-compatible`. ~50 LOC in `config.rs` + `llm.rs` + `main.rs`.
- **Missing library re-exports** — `HeartbeatAgent`, `SwarmOrchestrator`, `BackgroundTaskManager`, `Scheduler`, `McpClient`, `McpServer`, `EvalRunner`, `TelemetryGuard`, `RavenFabricClient`, and `run_server` are now re-exported from `src/lib.rs` for library users.
- **Complete CLI flags documentation** — All 45+ CLI flags now documented in `docs/guides/configuration.md` and `website/public/docs/configuration.html` (was 8 flags).

## [0.9.3] — 2026-06-27

### Added
- **rpi5 deployment feedback fixes** — Fixed 6 of 12 issues from real-world Raspberry Pi 5 deployment: added `#[serde(alias = "flat")]` to `SwarmTopology::Star` for backward compatibility, fixed all swarm/heartbeat/telemetry/server docs field names (42 incorrect occurrences across 6 files), made OpenTelemetry opt-in by default (`otel_disabled` default changed to `true`). Remaining items tracked in ISSUES.md and ROADMAP.md v1.0 hardening checklist.
- **Provider strategy documented** — Added comprehensive Provider Strategy section to ROADMAP.md covering: generic `openai-compatible` provider (unlocks vLLM, llama.cpp, LM Studio, TGI, Groq, Together AI, Fireworks, DeepInfra, and any custom OpenAI-compatible endpoint), vLLM docs/verification tests, llama.cpp docs/verification tests, and Azure OpenAI adapter. Deferred native AWS Bedrock and Gemini/Vertex (reachable via LiteLLM/OpenRouter). Tool-calling fidelity documented per backend.
- **Library API (`src/lib.rs`)** — Added `[lib]` section to `Cargo.toml` and created `src/lib.rs` with public re-exports of all 18 modules. RavenClaws is now usable as both a binary and a library crate (`ravenclaws`).
- **Performance benchmarks** — Verified v1.0 targets: 5.2 MB stripped binary, 5.2 ms cold start. Both well under v1.0 targets (< 15 MB, < 50 ms).
- **API stability guarantees** — `#[non_exhaustive]` added to all public enums (`RavenClawsError`, `ConfigError`, `LLMError`, `ToolError`, `LLMProvider`, `OpenAICompatibleProvider`, `CircuitState`, `ToolCategory`) and public structs (`Config`, `LLMConfig`, `SecurityConfig`, `RuntimeConfig`, `RavenFabricConfig`, `TelemetryConfig`, `SchedulerConfig`, `WebSearchConfig`). Doc comments added to all public types with stability notes.
- **Documentation guides** (`docs/guides/`) — Created comprehensive guides: getting-started, configuration reference, swarm-mode, mcp-integration, and heartbeat-mode. Each guide includes setup instructions, configuration examples, best practices, and use cases.
- **Runnable examples** (`examples/`) — Created 5 runnable Rust examples: `basic_chat` (minimal library usage), `agent_loop` (full agent loop with tools), `swarm` (multi-agent orchestration), `mcp_client` (MCP server connection), `heartbeat` (autonomous agent). Each example is documented with usage instructions.
- **README FAQ section** — Added comprehensive FAQ covering common questions: differentiation, API keys, offline use, security model, mode differences, library usage, upgrade paths, deployment targets, and licensing.
- **Library API re-exports** (`src/lib.rs`) — Added `pub use` re-exports for commonly used types. Added MSRV note (Rust 1.86+), semver guarantees section, and feature flag documentation.

### Removed
- **Deprecated LLM client types** — `LiteLLMClient`, `OpenRouterClient`, `OpenAIClient` (deprecated since v0.5.0) removed. Use `OpenAICompatibleClient` with the appropriate `OpenAICompatibleProvider` variant instead.
- **Legacy `execute_tool_call` function** — Deprecated since v0.4, replaced by `execute_tool_call_with_security` with full PolicyEngine/Sandbox/AuditLog integration.
- **Unused `run_exec_stream` function** — Streaming exec functionality is handled by the agent loop internally.
- **Unused `futures::StreamExt` import** — No longer needed after `run_exec_stream` removal.

### Changed
- **ROADMAP.md** — Updated for v1.0 scope: v0.10 features deferred to post-1.0. v1.0 now focuses on hardening + docs + API stability. Added completed items for deprecated type removal, dead code elimination, library API establishment, performance benchmarks, zero CVEs, API stability, and complete docs. Updated stats to 18 modules, 452 tests. Updated progress to 14/38 v1.0 items.
- **AGENTS.md** — Updated architecture diagram to include `lib.rs`, `eval.rs`, `ravenfabric.rs`. Updated module responsibilities table. Updated build stats (5.2 MB, 5 ms). Updated tool count to 5 built-in tools.
- **README.md** — Updated binary size references from ~3.4 MB to ~5.2 MB. Updated status to v0.9.3. Updated test count to 452. Added library crate mention. Added library usage guide with code example and module table. Updated verification badge to 114 checks. Added library crate badge. Added documentation section with links to all guides. Updated library code example to use new `pub use` re-exports.
- **VERIFICATION.md** — Updated module count from 16 to 18. Updated stale test counts (311→452). Removed duplicate `cargo test` line.
- **deny.toml** — Fixed typo in license exception name (`RavenClawss` → `RavenClaws`).

### Fixed
- **Cargo.toml** — Added missing `homepage`, `documentation`, `readme`, and `exclude` fields for crates.io publication readiness.
- **Dockerfile** — Pinned base image digests for reproducible builds (`rust:1.86-slim-bookworm` and `gcr.io/distroless/cc-debian12:nonroot` now use `@sha256:...` digests).

## [0.9.2] — 2026-06-25

### Added
- **Inter-agent communication bus** (`src/swarm.rs`) — Swarm agents can now share information and coordinate via a shared message bus.
  - `AgentMessage` struct with UUID, sender, recipient, message type, content, timestamp, and metadata
  - `MessageType` enum: Information, Question, Result, Error, Coordination, Generic
  - `AgentMessageBus` with send, receive, filter, and broadcast capabilities
  - `SwarmOrchestrator::new_with_bus()` for shared bus across sub-orchestrators
  - Task prompts enriched with message bus context for informed decision-making
  - Results broadcast back to the bus for peer awareness
  - CLI flags: `--swarm-communication` (env: `RAVENCLAW_SWARM_COMMUNICATION`)
  - 14 unit tests covering all message bus operations
- **Swarm health & telemetry** (`src/swarm.rs`) — Production-grade health monitoring for swarm agents with heartbeat tracking, dead-agent detection, and aggregate metrics.
  - `SwarmHealthMonitor` — tracks per-worker heartbeats, detects degraded/unhealthy/dead agents, and identifies replacement candidates
  - `WorkerHealthStatus` — four-state health model: Healthy, Degraded, Unhealthy, Dead
  - `WorkerTelemetry` — per-worker metrics: tasks completed/failed, error count, avg duration, messages sent/received, iteration count
  - `SwarmMetrics` — aggregate swarm health: total/healthy/degraded/unhealthy/dead workers, task throughput, worker utilization, error rate, communication latency
  - Heartbeat protocol with configurable interval (default: 5s), max missed beats (default: 3), and replacement timeout (default: 30s)
  - Health monitoring integrated into `execute_with_profile()` and `recursive_supervise_impl()` — workers auto-register, heartbeats update on task completion, failures are tracked
  - Health monitor shared across sub-orchestrators via `Arc<RwLock<>>` for recursive supervision
  - Periodic health check logging in supervisor loop (every 3 iterations)
  - Public accessors: `health_metrics()` and `worker_telemetry()` on `SwarmOrchestrator`
  - CLI flag: `--swarm-health-monitoring` (env: `RAVENCLAW_SWARM_HEALTH_MONITORING`)
  - 22 unit tests covering all health monitoring operations
  - 452 total unit tests (0 regressions)

## [0.9.1] — 2026-06-23

### Added
- **Self-provisioning of sub-agents** (`src/swarm.rs`) — RavenClaws dynamically spawns new agent instances based on task decomposition. Supervisor mode becomes recursive: supervisors spawn sub-supervisors, creating task decomposition trees of arbitrary depth.
  - `SwarmOrchestrator` — core orchestrator with recursive supervision, task analysis, role assignment, and result aggregation
  - `WorkerProfile` — declarative profile with persona, allowed_tools, provider/model overrides, resource limits, and delegation capability
  - `SwarmTopology` — four topologies: Star, Mesh, Hierarchical, Hybrid
  - `SwarmConfig` — configurable max_depth (default: 3), max_workers (default: 100), dynamic_role_assignment, profiles
  - 5 built-in worker profiles: researcher, creative, executor, reviewer, supervisor
  - Recursive supervision via `Box::pin` to avoid Rust's recursive async fn limitation
  - LLM-based dynamic role assignment (`analyze_task_roles`) with fallback to default roles
  - CLI flags: `--swarm-topology`, `--swarm-max-depth`, `--swarm-max-workers`, `--swarm-dynamic-roles`, `--swarm-profiles`
  - Config section: `[swarm]` in `ravenclaws.toml`
  - Mode dispatch: `--mode orchestrate` for both single-provider and multi-model paths
  - `MultiModelManager` made `Clone` for sub-orchestrator spawning
  - 17 unit tests covering all profiles, config serde, orchestrator construction, depth limits, task analysis fallback
  - 416 total unit tests (0 regressions)

## [0.9.0] — 2026-06-22

### Added
- **`token_lifetime_secs` enforcement** — `SecurityConfig.token_lifetime_secs` is now honored at runtime. When set to a non-zero value, agent sessions automatically terminate after the configured duration, enforcing credential/session expiry.
  - `AgentLoopConfig.token_lifetime_secs` — new field (default: 0 = unlimited)
  - Wired into both `run_agent_loop` and `run_agent_loop_with_mcp` — checked before each iteration
  - Session start time tracked via `std::time::Instant`
  - On expiry: returns `RavenClawsError::SecurityViolation` with elapsed time details
  - Audit log records `SecurityViolation` event with elapsed time, limit, and iteration
  - Removed `#[allow(dead_code)]` from `config.rs` `SecurityConfig.token_lifetime_secs`
  - 393 total unit tests (0 regressions)
- **Autonomous heartbeat agent** (`src/heartbeat.rs`) — persistent background loop that operates without human supervision, with configurable tick interval, progress assessment, planning, and execution.
  - `HeartbeatConfig` — config struct with goal, tick_interval_secs, max_iterations_per_tick, workdir, max_ticks, enable_tools
  - `HeartbeatState` — persisted state (id, goal, tick, progress, assessments, plans, results) with JSON serialization
  - `HeartbeatAgent` — full implementation with assess→plan→act→persist→sleep loop
  - State persistence to `workdir/heartbeat-<id>.json` — survives restarts and resumes from last checkpoint
  - LLM-driven goal completion detection (responds to `GOAL_COMPLETE` / `[DONE]` markers)
  - Agent loop integration for tool execution during each tick
  - CLI flags: `--heartbeat`, `--heartbeat-goal`, `--heartbeat-tick-interval`, `--heartbeat-max-ticks`, `--heartbeat-session`
  - Config section: `[heartbeat]` in `ravenclaws.toml`
  - 8 unit tests covering config defaults, state lifecycle, serialization, and prompt building
  - 401 total unit tests (0 regressions)
- **Long-horizon task persistence** — task state survives restarts; agent resumes from last checkpoint with full context.
  - `HeartbeatState` persisted to `workdir/heartbeat-<id>.json` after every tick
  - `HeartbeatAgent::new()` auto-resumes from saved state on restart
  - `BackgroundTaskManager` persists all tasks as individual JSON files in `<workdir>/tasks/`
  - `--task-resume` flag re-executes incomplete tasks on startup
  - 401 total unit tests (0 regressions)

## [0.8.0] — 2026-06-22

### Added
- **Prompt-injection defense** (`src/policy.rs`) — two-layer LLM output security that detects and blocks prompt-injection attempts before they reach the agent loop
  - `InjectionDetector` — scans LLM responses for 50+ known injection/jailbreak patterns (instruction override, system prompt extraction, DAN jailbreak, token smuggling, meta-instruction attacks)
  - `InjectionVerdict` — `Clean` or `Suspicious(reason)` result type
  - Instruction-boundary enforcement — detects attempts to ignore/disregard/override system instructions
  - Output schema validation — validates JSON in tool call arguments, detects unbalanced code blocks, enforces maximum response length (100KB)
  - Wired into both `run_agent_loop` and `run_agent_loop_with_mcp` — checks every LLM response before processing
  - `SecurityConfig.prompt_injection_protection` — enable/disable via config (default: enabled)
  - `AgentLoopConfig.prompt_injection_protection` — per-invocation control
  - `AuditEventType::SecurityViolation` — new audit event type for injection detection
  - All violations are logged to audit log with reason, iteration, and content preview
  - 390 total unit tests (0 regressions)
- **`zeroize` for secret material** — API keys in `LLMConfig` and HMAC secret key in `AuditLog` are zeroized on drop, preventing secret leakage from memory dumps
  - `use zeroize::Zeroize` in `config.rs` and `audit.rs`
  - `impl Drop for LLMConfig` — zeroizes `api_key` field
  - `impl Drop for AuditLog` — zeroizes `key` field
  - Replaced `atty` dependency with `std::io::IsTerminal` (Rust 1.70+ stable)
  - 390 total unit tests (0 regressions)
- **Web search + content extraction tool** (`src/tools.rs`) — search the web and extract readable content from results
  - `WebSearchTool` with SearXNG JSON API and DuckDuckGo HTML backends
  - `WebSearchConfig` in `config.rs` — configurable endpoint, engine, max_results, fetch_content
  - `html_to_text()` — strips HTML tags, extracts title, normalizes whitespace, decodes HTML entities
  - `strip_html_tags()`, `extract_href()`, `urlencoding()` helper functions
  - `ToolRegistry::with_web_search_config()` — configurable web search registration
  - 20 new unit tests covering tool definition, config, HTML extraction, URL encoding, error handling
  - 390 total unit tests (+20, 0 regressions)
- **Eval harness** (`src/eval.rs`) — golden-task evaluation framework with run inspection
  - `EvalConfig`/`EvalTask`/`EvalRunner` — TOML-based eval suite configuration with 7 assertion types (contains, not_contains, exact, regex, non_empty, min_length, max_length)
  - `RunTrace` — full step-by-step trace of agent runs including LLM calls and tool calls
  - `EvalReport` — human-readable text and machine-readable JSON output formats
  - `--eval <path>` CLI flag — run an eval suite from a TOML config file
  - `--eval-json` CLI flag — output eval results as JSON
  - 24 Rust unit tests covering all assertion types, config parsing, report formatting, and error handling
  - Sample eval configs in `tests/eval/basic-suite.toml` and `tests/eval/security-suite.toml`
  - 20 verification tests in `scripts/lib/test-eval.sh` registered in `verify.sh` as `--eval` module
  - 353 total unit tests (+24, 0 regressions) → 390 (+37 for web search + scheduling + background)
- **Scheduling & triggers** (`src/scheduler.rs`) — cron, webhook, and file-watch activation for proactive 24/7 agents
  - `TriggerConfig` — configurable trigger with name, prompt, system_prompt, and trigger type
  - `TriggerType` enum — `Cron { expression }`, `Webhook { secret }`, `Watch { path, events, debounce_secs }`
  - `Scheduler` — manages trigger lifecycle with `start()`/`stop()` methods
  - Cron triggers — parses cron expressions via `cron` crate, sleeps until next scheduled time
  - Webhook triggers — TCP listener on configurable port (default 9090), JSON-RPC style POST handler
  - Watch triggers — filesystem monitoring via `notify` crate with debouncing and event filtering
  - `--scheduler` CLI flag — runs scheduler mode with all configured triggers
  - `--webhook-port` CLI flag — override webhook listener port (env: `RAVENCLAW_WEBHOOK_PORT`)
  - All triggers submit tasks to `BackgroundTaskManager` for execution
  - 17 new unit tests covering config serialization, cron parsing, scheduler lifecycle, webhook response format, and all trigger types
  - 353 total unit tests (+17, 0 regressions)
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
- **Helm chart** (`charts/ravenclaws/`) — official Helm chart for deploying RavenClaws on Kubernetes
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
- **MCP Server** (`src/mcp.rs`) — expose RavenClaws's built-in tools over stdio via the Model Context Protocol
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

---

[Unreleased]: https://github.com/egkristi/RavenClaws/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/egkristi/RavenClaws/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/egkristi/RavenClaws/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/egkristi/RavenClaws/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/egkristi/RavenClaws/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/egkristi/RavenClaws/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/egkristi/RavenClaws/compare/v0.5.3...v0.6.0
[0.5.3]: https://github.com/egkristi/RavenClaws/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/egkristi/RavenClaws/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/egkristi/RavenClaws/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/egkristi/RavenClaws/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/egkristi/RavenClaws/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/egkristi/RavenClaws/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/egkristi/RavenClaws/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/egkristi/RavenClaws/releases/tag/v0.1.0
