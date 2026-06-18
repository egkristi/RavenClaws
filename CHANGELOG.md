# Changelog

All notable changes to RavenClaw are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- RavenFabric integration — secure E2E remote command execution + mesh coordination (v0.6.1)
- Agent communication — structured message passing; conflict resolution across agents (v0.6.1)

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
