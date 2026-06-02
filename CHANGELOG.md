# Changelog

All notable changes to RavenClaw will be documented in this file.

## [Unreleased]

### Added
- Agent loop (perceive→plan→act→observe) — `AgentLoopConfig` + `run_agent_loop()` with max-iteration guard and `FINAL:` marker detection
- `--max-iterations` CLI flag and `RAVENCLAW_MAX_ITERATIONS` env var for agent loop configuration
- 6 new agent loop tests: config defaults, FINAL completion, max iterations reached, LLM error, empty response, custom config

### Fixed
- CI workflows: Trivy action updated to `v0.36.0`, Kubescape action migrated to `kubescape/github-action@main`, CodeQL upload-sarif updated to `@v4`, `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` added to all workflows
- Removed unused `rustls = "0.23"` and `zeroize = "1.8"` dependencies

### Added
- Streaming responses — `LLMProviderTrait::chat_stream()` with SSE parsing for LiteLLM, default non-streaming fallback for other providers
- System prompt / persona configuration — `LLMConfig.system_prompt` field with CLI `--system-prompt` and `RAVENCLAW_SYSTEM_PROMPT` env var override
- Conversation memory — `ConversationMemory` struct with configurable max history, automatic trimming of oldest messages
- Interactive REPL mode — `--repl` / `-R` flag for stdin-based continuous conversation with streaming output, `/exit`, `/quit`, `/reset` commands
- `futures` crate dependency for streaming support
- `reqwest` `stream` feature enabled for `bytes_stream()` SSE parsing
- 8 new tests: system_prompt default, system_prompt custom, ConversationMemory (5), REPL CLI flag
- `--exec` mode now fully wired — one-shot command execution with response printed to stdout
- Comprehensive Rust unit tests: 149 tests across all modules (was 3)
- `serial_test` crate for serializing env-dependent tests to prevent env var leakage
- `Config::load()` now safely handles `RAVENCLAW__LLMS` env var by saving/restoring it around serde deserialization
- Manual `Default` implementations for `RavenFabricConfig`, `SecurityConfig`, and `RuntimeConfig` matching serde defaults
- CLI `--version` now uses `env!("CARGO_PKG_VERSION")` instead of hardcoded string
- Test coverage for config validation, LLM client creation, error types, CLI argument parsing, and agent stubs
- 15 new `mockito`-based HTTP tests covering all 4 LLM providers (LiteLLM, OpenAI, OpenRouter, Ollama) with success, auth failure (401), rate limit (429), server error (500), and invalid JSON response paths
- 8 new config edge case tests: TLS disabled, TLS with CA, TLS with cert+key, multi-provider config, custom LiteLLM config, custom Ollama config, custom OpenAI config, custom OpenRouter config
- 4 new agent tests: multi-model stubs, `--exec` error propagation, agent type check
- 4 new error tests: async network error, IO error, debug formatting, Send+Sync trait bounds
- RavenFabric agent SHA256 checksum verification in Dockerfile
- Cross-compilation linkers (`gcc-aarch64-linux-gnu`, `gcc-x86_64-linux-gnu`) in Docker build stage
- Cargo target linker configuration for multi-arch Docker builds

### Fixed
- `--exec` dead code — CLI arg was parsed but never used; now sends prompt to LLM and prints response
- Swarm/supervisor stubs now return `Err(RavenClawError::CommandExecution(...))` instead of silently exiting 0
- All 4 LLM client constructors (`LiteLLMClient`, `OpenRouterClient`, `OllamaClient`, `OpenAIClient`) now return `Result<Self, LLMError>` instead of calling `.expect()`
- `create_client()` factory function propagates client construction errors via `?`
- Verification `check_llm_response_quality` now handles `--exec` mode output (stdout-based responses)
- `Cargo.lock` removed from `.gitignore` and committed for reproducible builds
- OpenRouter and OpenAI clients now respect `config.endpoint` when non-empty, falling back to hardcoded defaults (enables mockito testing)
- Docker multi-arch build: cross-linkers installed and cargo target linker configured per-platform

### Added
- **Tool / function-calling abstraction** — `ToolImpl` trait, `ToolRegistry`, `ToolDefinition`, `ToolCall`, `ToolResult` types in `src/tools.rs`
- **4 built-in tools**: `ShellTool` (shell_exec), `ReadFileTool` (read_file), `WriteFileTool` (write_file), `WebFetchTool` (web_fetch) — each with JSON schema definitions
- **Tool wiring into agent loop** — `run_agent_loop()` detects `TOOL_CALL:` / `ARGS:` patterns in LLM responses, executes tools via `ToolRegistry`, injects results as `OBSERVATION:` messages
- **Deny-by-default policy engine** — `PolicyEngine` in `src/policy.rs` with shell command, file path, and network request allow-lists
- **Sandboxed execution** — `Sandbox` in `src/sandbox.rs` with workdir jail, path resolution, resource limits, timeouts, temp file creation, filtered environment
- **Tamper-evident audit log** — `AuditLog` in `src/audit.rs` with HMAC-SHA256 chaining, structured JSON output, verification against tampering
- `enable_tools` field on `AgentLoopConfig` — when enabled, tool definitions are injected into the system prompt
- 6 new dependencies: `hmac 0.12`, `sha2 0.10`, `hex 0.4`, `chrono 0.4`, `rand 0.8`, `url 2.5`
- 100+ new tests across tools (30+), policy (30+), audit (20+), sandbox (15+), and agent loop tool wiring

### Changed
- `RavenFabricConfig`, `SecurityConfig`, `RuntimeConfig` now use manual `Default` impls instead of `#[derive(Default)]` to ensure serde defaults match Rust defaults
- Architecture expanded from 5 to 8 source modules (added `tools.rs`, `policy.rs`, `audit.rs`, `sandbox.rs`)
