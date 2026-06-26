# 🐦‍⬛ RavenClaws — AI Agent Instructions

This file contains structured instructions for AI coding agents working on the RavenClaws codebase. It defines the project's architecture, conventions, common tasks, and guardrails.

---

## Vision

RavenClaws aims to be the **ultimate AI agentic assistant and worker** — and the **preferred alternative** to the field: Nemoclaw, Hermes Agent, TrustClaw, ZeroClaw, PicoClaw, NanoClaw, Claude Cowork, Manus, Perplexity Computer, Kimi Claw, and Vellum.

We don't aim to win by out-featuring them. We win by refusing to compromise on five pillars at once:

- **Secure** — memory-safe Rust (`unsafe` forbidden), fail-closed, no creds in config, verified supply chain.
- **Small** — one static binary (~5 MB), distroless image, lean dependency tree.
- **Efficient** — native performance, low memory, fast cold start, streaming everywhere.
- **Robust** — graceful degradation, provider fallback, deterministic config, verified across 4 deployment targets.
- **Simple** — one command to run, sensible defaults, no external services required for single-agent use.

---

## Project Overview

RavenClaws is a **lightweight, secure Rust agent framework** with multi-provider LLM support. It runs as a single binary with zero runtime dependencies.

- **Language:** Rust (edition 2021)
- **Version:** 0.9.2 (autonomous heartbeat + self-provisioning swarm + inter-agent communication + swarm health & telemetry)
- **License:** AGPL-3.0-or-later + Commercial
- **Repository:** https://github.com/egkristi/RavenClaws
- **Domain:** https://RavenClaws.io
- **Build:** `cargo build --release` (~5.2 MB stripped binary, ~5 ms startup)
- **Library:** Available as `ravenclaws` on crates.io (binary + library crate)

### Architecture (18 modules)

```
src/
├── lib.rs       — Library crate entry point, public API re-exports
├── main.rs      — CLI entry point (clap), config loading, mode dispatch
├── agent.rs     — Agent implementations (single, swarm, supervisor, REPL, ConversationMemory, agent loop with tool wiring)
├── background.rs— Background task manager (async long-horizon runs, disk persistence, resumability)
├── scheduler.rs — Scheduling & triggers (cron, webhook, file-watch activation for proactive 24/7 agents)
├── heartbeat.rs — Autonomous heartbeat agent (persistent assess→plan→act→persist→sleep loop, state persistence, resumability)
├── swarm.rs     — Swarm orchestration (self-provisioning sub-agents, recursive supervision, WorkerProfile, SwarmTopology, dynamic role assignment)
├── llm.rs       — LLM provider abstraction (trait + 5 clients + multi-model manager + streaming)
├── config.rs    — Config structs, TOML/env loading, validation
├── error.rs     — Unified error types
├── tools.rs     — Tool abstraction (ToolImpl trait, ToolRegistry, ToolCall, ToolResult) + 5 built-in tools (shell, read/write file, web fetch, web search)
├── mcp.rs       — MCP client (JSON-RPC 2.0 over stdio, tool discovery) + MCP server (expose tools over stdio)
├── server.rs    — HTTP server mode (health, readiness, metrics endpoints, graceful shutdown)
├── telemetry.rs — OpenTelemetry tracing (OTLP gRPC/stdout exporter, TelemetryGuard, #[instrument] spans)
├── policy.rs    — Deny-by-default policy engine (shell, path, network allow-lists)
├── audit.rs     — Tamper-evident audit log (HMAC-SHA256 chained, structured JSON)
├── sandbox.rs   — Sandboxed execution (workdir jail, path resolution, resource limits, timeouts)
├── eval.rs      — Eval harness (assertions, run traces, text/JSON reports)
└── ravenfabric.rs— RavenFabric mesh client (health, list_agents, execute, broadcast)
```

### Current State

| Feature | Status |
|---|---|
| Single agent mode | ✅ Working — sends prompt, logs response |
| Multi-provider (LiteLLM, OpenAI, OpenRouter, Ollama, Anthropic) | ✅ Working |
| Multi-model manager | ✅ Working — iterates all configured providers, round-robin routing |
| CLI with env-var overrides | ✅ Working |
| OpenAI-compatible API support | ✅ Working — any `/v1/chat/completions` endpoint |
| Container security (non-root, read-only FS, dropped caps) | ✅ Working |
| Library crate (ravenclaws on crates.io) | ✅ Working — binary + library |
| Verification suite (452 tests, 18 modules, 0 failures) | ✅ Working |
| `--exec` mode | ✅ Working — one-shot command execution with response to stdout |
| Streaming responses | ✅ Working — SSE streaming for LiteLLM, default fallback for others |
| Conversation memory | ✅ Working — `ConversationMemory` struct with configurable max history |
| Interactive REPL | ✅ Working — `--repl` flag with stdin loop, streaming output |
| System prompt / persona | ✅ Working — `LLMConfig.system_prompt`, CLI `--system-prompt`, env var |
| Swarm mode | ✅ Working — 3 parallel agents with different personas (single + multi-model) |
| Supervisor mode | ✅ Working — task decomposition + sub-agent spawning + result aggregation (single + multi-model) |
| Self-provisioning swarm orchestration | ✅ v0.9.0 — recursive supervisor spawning, WorkerProfile, SwarmTopology, dynamic role assignment, 5 built-in profiles |
| Inter-agent communication bus | ✅ v0.9.1 — AgentMessageBus with send/receive/broadcast, MessageType enum, shared bus across sub-orchestrators |
| Swarm health & telemetry | ✅ v0.9.2 — SwarmHealthMonitor with heartbeat tracking, dead-agent detection, aggregate metrics, 22 unit tests |
| Tool-use / function calling | ✅ Working — ToolImpl trait + ToolRegistry + 5 built-in tools + agent loop wiring |
| Agent loop / ReAct planning | ✅ Working — perceive→plan→act→observe with max-iteration guard, tool call detection |
| Deny-by-default policy | ✅ Working — PolicyEngine with shell/path/network allow-lists |
| Sandboxed execution | ✅ Working — workdir jail, path resolution, resource limits, timeouts |
| Tamper-evident audit log | ✅ Working — HMAC-SHA256 chained, structured JSON, verification |
| MCP client | ✅ Working — JSON-RPC 2.0 over stdio, tool discovery and registration |
| MCP server | ✅ v0.7.0 — expose RavenClaws tools over stdio via MCP protocol; `--mcp-server` flag; policy-checked and audited |
| HTTP server mode | ✅ v0.7.1 — long-running server with `/health`, `/ready`, `/metrics`; `--serve` flag; graceful shutdown |
| OpenTelemetry tracing | ✅ v0.7.2 — opt-in distributed tracing with OTLP gRPC/stdout exporter; `#[instrument]` spans on agent loop, HTTP server, tools, LLM calls |
| Helm chart | ✅ v0.7.3 — official Helm chart for K8s deployment with 11 configurable resources |
| Async background runs | ✅ v0.8.0 — assign-and-walk-away execution with disk persistence and resumability |
| Scheduling & triggers | ✅ v0.8.0 — cron, webhook, and file-watch activation for proactive 24/7 agents |
| Autonomous heartbeat | ✅ v0.9.0 — persistent assess→plan→act→persist→sleep loop with state persistence and resumability |
| Long-horizon task persistence | ✅ v0.9.0 — task state survives restarts; heartbeat resumes from last checkpoint; background tasks persist to disk |
| Retry / fallback chains | ✅ Working — exponential backoff, circuit breaker, token budgets |
| RavenFabric integration | ✅ Working — HTTP client with health, list_agents, execute, broadcast; wired to all modes |
| GitHub Actions CI/CD | ✅ Implemented — fmt + clippy + test, 5-target builds, multi-arch images, Cosign + SBOM + provenance + Trivy, crates.io publish, releases |
| Security scanning | ✅ Implemented — CodeQL, cargo-audit, cargo-deny, cargo-outdated, cargo-udeps, Trivy (FS + config), Hadolint, Kubescape, OSSF Scorecard, dependency review |
| Pre-built binaries / releases | 📋 Wired, untagged — CI produces them on tag; none released yet |

---

## Documentation Conventions

### CHANGELOG.md — Tracking Implemented Features & Fixes

All implemented features, resolved issues, and completed changes **must** be documented in `CHANGELOG.md` with the following format:

```markdown
## [Unreleased]

### Added
- New feature description (#issue-number)

### Fixed
- Bug fix description (#issue-number)

### Changed
- Modification to existing feature (#issue-number)

### Removed
- Feature or code removal (#issue-number)
```

**When to update:** Every time a PR is merged or a feature branch is completed. Never commit a feature without a corresponding CHANGELOG entry.

### ISSUES.md — Tracking Known Problems

All identified bugs, technical debt, and known limitations **must** be documented in `ISSUES.md` with severity labels:

```markdown
## Critical
- **k8s Deployment enters CrashLoopBackOff** — binary exits after one request, no server mode yet. [No issue]

## High
- **RavenFabric integration not wired** — config struct exists, binary in container, but runtime wiring pending. [No issue]

## Medium
- **22 pre-existing clippy dead_code warnings** — infrastructure types not yet wired to agent loop. [No issue]
```

**Severity levels:**
- **Critical** — Blocks a release or causes incorrect behavior
- **High** — Significant missing functionality or risk
- **Medium** — Important but non-blocking
- **Low** — Nice-to-have improvements

### ROADMAP.md — Tracking Planned Features

All planned features and feature requests **must** be documented as checklists in `ROADMAP.md` with priority labels:

```markdown
### Priority: High
- [ ] **Tool-use (function calling)** — The #1 missing piece. Agent must call tools, not just chat.

### Priority: Medium
- [ ] **Streaming responses** — Real-time token-by-token output for interactive use
```

**Priority levels:**
- **High** — Required for next release or core functionality
- **Medium** — Important but can wait for a later release
- **Low** — Nice-to-have, no fixed timeline

**When a feature is completed:** Move it from ROADMAP.md to CHANGELOG.md under the appropriate `[Unreleased]` section header (`### Added`, `### Fixed`, etc.). Never leave a completed feature in ROADMAP.md — it must be migrated to CHANGELOG.md to keep both documents accurate.

### Test Coverage Mandate

**All critical functionality MUST have automated tests.** Specifically:

- **Every new feature** must include both:
  1. Rust unit tests in the relevant `src/*.rs` module (via `#[cfg(test)] mod tests`)
  2. Shell verification tests in `scripts/lib/` (for integration-level coverage)
- **Every bug fix** must include a test that reproduces the bug and verifies the fix
- **Every config field** must have a test that exercises it
- **Every LLM provider** must have a test in `test-llm-quality.sh`
- **Every deployment target** must have tests in the corresponding `test-*.sh` module

If a feature cannot be tested (e.g., hardware-dependent), document the reason in the test file.

---

## Code Conventions

### Rust Style

- **Formatting:** Standard `rustfmt` (no custom config)
- **Linting:** Standard `clippy`
- **Naming:** Snake_case for functions/variables, CamelCase for types/enums, SCREAMING_SNAKE_CASE for constants
- **Error handling:** `thiserror` for library errors, `anyhow` for application-level errors
- **Async:** `tokio` runtime with `async-trait` for provider abstraction
- **Logging:** `tracing` with JSON format — use `info!`, `warn!`, `error!` consistently
- **Unimplemented features:** Use `warn!("...not yet implemented")` + return `Ok(())` — do not panic or exit with error

### Module Responsibilities

| Module | Owns | Does NOT own |
|---|---|---|
| `lib.rs` | Library crate entry, public API re-exports | Agent logic, LLM calls, config structs |
| `main.rs` | CLI parsing, config loading, mode dispatch | Agent logic, LLM calls, config structs |
| `agent.rs` | Agent run functions (single, swarm, supervisor, REPL, agent loop) | LLM client creation, config parsing |
| `background.rs` | `BackgroundTaskManager`, `BackgroundTask`, `TaskStatus`, disk persistence | Agent logic, LLM calls |
| `scheduler.rs` | `Scheduler`, `TriggerConfig`, `TriggerType`, cron/webhook/watch runners | Agent logic, LLM calls |
| `heartbeat.rs` | `HeartbeatAgent`, `HeartbeatConfig`, `HeartbeatState`, assess→plan→act→persist→sleep loop | Agent logic, LLM calls |
| `swarm.rs` | `SwarmOrchestrator`, `WorkerProfile`, `SwarmConfig`, `SwarmTopology`, recursive supervision, dynamic role assignment | Agent logic, LLM calls |
| `llm.rs` | `LLMProviderTrait`, client implementations, `MultiModelManager` | Agent logic, config structs |
| `config.rs` | `Config`, `LLMConfig`, validation, env loading | Agent logic, HTTP requests |
| `error.rs` | `RavenClawsError` enum, `Result<T>` alias | Everything else |
| `tools.rs` | `ToolImpl` trait, `ToolRegistry`, `ToolCall`, `ToolResult`, 5 built-in tools | Agent logic, LLM calls |
| `policy.rs` | `PolicyEngine` with shell/path/network allow-lists | Tool execution, LLM calls |
| `audit.rs` | `AuditLog` with HMAC-SHA256 chaining | Tool execution, policy decisions |
| `sandbox.rs` | `Sandbox` with workdir jail, resource limits, timeouts | Tool execution, LLM calls |

### Adding a New LLM Provider

1. Add a variant to `LLMProvider` enum in `config.rs`
2. Create a client struct in `llm.rs` implementing `LLMProviderTrait`
3. Add the client to the `create_client()` factory function
4. Add the provider mapping in `main.rs` CLI override section
5. Add test config in `tests/config/`
6. Add verification tests in `scripts/lib/test-llm-quality.sh`
7. Add CHANGELOG.md entry under "Added"
8. Mark the corresponding ROADMAP.md checklist item as complete

---

## Verification System

The verification suite lives in `scripts/` and is **separate from `cargo test`** (which only has 2 unit tests).

### Structure

```
scripts/
├── verify.sh              — Main orchestrator
└── lib/
    ├── common.sh          — Shared library (colors, paths, logging, test runner)
    ├── test-litellm.sh    — LiteLLM connectivity (4 tests)
    ├── test-local.sh      — macOS binary (12 tests)
    ├── test-docker.sh     — Docker container (10 tests)
    ├── test-linux.sh      — Linux cross-compile (6 tests)
    ├── test-k8s.sh        — Kubernetes (13 tests)
    ├── test-security.sh   — Binary integrity (8 tests)
    ├── test-performance.sh— Benchmarks (5 benchmarks)
    ├── test-llm-quality.sh— LLM response quality (36 tests)
    ├── test-swarm.sh      — Swarm & sub-agent (10 tests)
    └── test-eval.sh       — Eval harness (20 tests)
```

### Running Tests

```bash
./scripts/verify.sh --all          # Full suite (114 tests)
./scripts/verify.sh --quick        # Smoke test (litellm + local + swarm + eval + security)
./scripts/verify.sh --litellm      # Single module
./scripts/verify.sh --build        # Build + all tests
```

### Git Hooks (Pre-Commit / Pre-Push)

The project includes git hooks for automated verification before commits and pushes:

```
.githooks/
├── pre-commit    — Fast checks: fmt, clippy, tests, binary size, secrets scan
├── pre-push      — Full checks: pre-commit + release build + Docker + security
└── setup.sh      — Install/check/remove hooks
```

**Install:**
```bash
.githooks/setup.sh          # Configure git to use .githooks
.githooks/setup.sh --check  # Verify hooks are active
.githooks/setup.sh --remove # Restore default hooks
```

**What pre-commit checks:**
1. `cargo fmt --check` — formatting
2. `cargo clippy -D warnings` — linting
3. `cargo test --locked` — unit tests
4. Binary size check — warns if over 5MB
5. Secrets scan — no hardcoded API keys/tokens

**What pre-push additionally checks:**
1. Full pre-commit suite
2. Release build (`cargo build --release`)
3. Binary integrity (architecture, stripped, size)
4. Docker build (if Docker available)
5. Security scan (secrets, setuid, Cargo.lock)

**Skip hooks (emergency only):**
```bash
git commit --no-verify
git push --no-verify
```

### Test Runner Functions

- `run_test "name" command args...` — Runs command, captures output to `target/verification-results/`, logs PASS/FAIL
- `run_test_verbose "name" command args...` — Same but shows full output on failure
- `check_llm_response_quality log_file model_name` — Checks that LLM response is non-empty

### Common Pitfalls

- **Test names with `/`** — Creates subdirectories in results. Use spaces or hyphens instead.
- **`bash -c` quoting** — Use escaped double quotes `\"` inside `bash -c` strings. Avoid nested single quotes.
- **macOS `file` command** — Doesn't say "stripped" for stripped binaries. Check for "Mach-O" instead.
- **Distroless containers** — No shell, no `cat`, no `id`. Use `docker image inspect` for user checks.
- **K8s `runAsNonRoot`** — Requires numeric UID. Use `runAsUser: 65532` instead.
- **ConfigMap jsonpath** — Keys with dots (e.g., `ravenclaws.toml`) are unreliable. Use `go-template='{{index .data "key"}}'`.

---

## Deployment Targets

| Target | Binary Location | How to Build |
|---|---|---|
| macOS (aarch64) | `target/release/ravenclaws` | `cargo build --release` |
| macOS (x86_64) | `target/x86_64-apple-darwin/release/ravenclaws` | `cargo build --release --target x86_64-apple-darwin` |
| Linux ARM64 | `target/aarch64-unknown-linux-gnu/release/ravenclaws` | Cross-compile or Docker build |
| Linux x86_64 | `target/x86_64-unknown-linux-gnu/release/ravenclaws` | Cross-compile or Docker build |
| Docker | `ghcr.io/egkristi/ravenclaws:latest` | `docker build -t ravenclaws:latest .` |
| Kubernetes | `k8s/deployment.yaml` | `kubectl apply -f k8s/deployment.yaml` |

### Docker

- Multi-stage build: `rust:1.86-slim-bookworm` → `gcr.io/distroless/cc-debian12:nonroot`
- User: `nonroot` (UID 65532)
- No shell, no package manager, minimal attack surface
- HEALTHCHECK runs `--version`

### Kubernetes

- Production: `k8s/deployment.yaml` — in-cluster LiteLLM, full RBAC, Secrets
- Testing: `k8s/deployment-test.yaml` — hostNetwork for local LiteLLM, no Secrets

---

## Common Tasks

### Fixing `--exec` Mode

The `--exec` CLI flag is parsed in `main.rs` but never used. To fix:
1. In `main.rs`, after config loading, check `if let Some(prompt) = args.exec`
2. Pass the prompt as the user message content instead of the hardcoded "Ready. Awaiting instructions."
3. The agent should send the exec prompt and print the response to stdout (not just log it)

### Adding a New Test Module

1. Create `scripts/lib/test-myfeature.sh` with a function `test_myfeature()`
2. Source `common.sh` at the top
3. Add to `MODULES` array in `scripts/verify.sh`: `"myfeature:test-myfeature.sh:test_myfeature:My Feature Description"`
4. Use `run_test` and `check_llm_response_quality` from common.sh

### Adding a New Config Field

1. Add field to the appropriate struct in `config.rs`
2. Add a `#[serde(default = "default_fn")]` attribute with a default function
3. Add validation in `Config::validate()` if needed
4. Add env var mapping in `Config::load()` if needed
5. Update test configs in `tests/config/`

---

## Maintenance Cycle Workflow

This is the **standard operating procedure** for every maintenance cycle. Follow these steps in order, every time.

### Phase 1: Check CI Status

```bash
curl -s "https://api.github.com/repos/egkristi/RavenClaws/actions/runs?per_page=6&branch=master&event=push" | python3 -c "
import json,sys
data=json.load(sys.stdin)
for r in data.get('workflow_runs',[])[:6]:
    print(f\"#{r['run_number']:5d}  {r['name'][:25]:25s}  {r['status']:15s}  {str(r['conclusion']):10s}  {r['head_commit']['message'][:60] if r.get('head_commit') else '---'}\")"
```

- Check all 3 workflows: **Build & Release**, **Container Build**, **Security Scan**
- If any are **in_progress**, wait for them to complete
- If any **failed**, investigate and fix before proceeding
- If all green, proceed to Phase 2

### Phase 2: Fix Issues

1. **ISSUES.md** — Read the file, address each issue by severity (Critical → High → Medium → Low)
2. **VS Code Problems Tab** — Open the problems tab (`Cmd+Shift+M`), fix all errors/warnings
3. **ROADMAP.md** — Read the file, pick the next uncompleted feature and implement it

### Phase 3: Verify Locally on Orbstack

Run the full verification suite against all deployment targets available on Orbstack:

```bash
# 1. Local binary (macOS)
./scripts/verify.sh --local

# 2. Docker container (Orbstack Docker)
./scripts/verify.sh --docker

# 3. Kubernetes (Orbstack K8s)
./scripts/verify.sh --k8s

# 4. Linux VM (Orbstack cross-compile)
./scripts/verify.sh --linux

# 5. Full suite (all of the above + security + performance + LLM quality)
./scripts/verify.sh --all
```

- If any tests **fail**, fix the issue and re-run
- If any issues arise, **register them in ISSUES.md** with appropriate severity

### Phase 4: Update Documentation

When a feature is finished or a fix is complete, update **all** relevant documents:

| Document | When to update |
|---|---|
| `ROADMAP.md` | Feature completed → move from ROADMAP to CHANGELOG. CI status updated. |
| `CHANGELOG.md` | Every feature, fix, or change → add under `[Unreleased]` |
| `README.md` | User-facing features or config changes |
| `ISSUES.md` | New bugs discovered, issues resolved, CI status updated |
| `VERIFICATION.md` | Tests added, changed, or removed |
| `AGENTS.md` | Workflow changes, new conventions, architecture changes |

### Phase 5: Commit & Push

```bash
# Stage all changes
git add -A

# Pre-commit hooks run automatically (fmt, clippy, 416 tests, binary size, secrets)
git commit -m "Descriptive summary of changes"

# Pre-push hooks run automatically (pre-commit + release build + Docker + security)
git push
```

- If pre-commit hooks **fail**, fix and re-commit
- If pre-push hooks **fail**, fix and re-push

### Phase 6: Verify CI After Push

After pushing, **monitor all 3 GitHub Actions workflows to completion**:

1. **Build & Release** — Check that all 5 build targets succeed
2. **Container Build** — Check multi-arch images build and push
3. **Security Scan** — Check CodeQL, cargo-audit, cargo-deny, etc.

```bash
# Poll until all complete
curl -s "https://api.github.com/repos/egkristi/RavenClaws/actions/runs?per_page=6&branch=master&event=push" | python3 -c "
import json,sys
data=json.load(sys.stdin)
for r in data.get('workflow_runs',[])[:6]:
    print(f\"#{r['run_number']:5d}  {r['name'][:25]:25s}  {r['status']:15s}  {str(r['conclusion']):10s}  {r['head_commit']['message'][:60] if r.get('head_commit') else '---'}\")"
```

- If any pipeline **fails**, investigate, fix, and re-push
- If any issues arise, **register them in ISSUES.md**

### Phase 7: Release (mandatory for all versions)

**Every completed version MUST be released.** No exceptions. Whether it's a minor version with new features or a patch version with bug fixes — if it's merged to master, it ships. A release includes:
- Version bump in `Cargo.toml`
- Changelog section for the new version
- Signed git tag
- GitHub Release with binary assets
- Container images pushed to GHCR and Docker Hub
- crates.io publication

### Cycle Checklist

```markdown
## Maintenance Cycle Checklist

- [ ] Phase 1: CI all green
- [ ] Phase 2: ISSUES.md addressed
- [ ] Phase 2: VS Code problems fixed
- [ ] Phase 2: ROADMAP.md progress made
- [ ] Phase 3: Verification suite passes (--all)
- [ ] Phase 4: All relevant docs updated
- [ ] Phase 5: Committed & pushed (hooks pass)
- [ ] Phase 6: CI all green after push
- [ ] Phase 7: Release (mandatory for all versions)
```

---

## Guardrails

### Do NOT

- **Do not** add Python, Node.js, or other runtime dependencies — the binary must be self-contained
- **Do not** hardcode API keys, tokens, or credentials anywhere in the codebase
- **Do not** remove the `strip = true` or `panic = "abort"` from release profile
- **Do not** use `unsafe` code unless absolutely necessary and documented
- **Do not** add large dependencies (>100KB) without evaluating alternatives
- **Do not** change the distroless base image without security review
- **Do not** remove the `readOnlyRootFilesystem: true` or `capabilities.drop: ["ALL"]` from K8s manifests
- **Do not** make swarm/supervisor modes exit with error — keep the stub pattern until implemented

### Do

- **Do** use `tracing` for all logging (not `println!` or `eprintln!`)
- **Do** add tests for every new feature (both Rust unit tests and shell verification tests)
- **Do** update CHANGELOG.md for every implemented feature, fix, or change
- **Do** update ISSUES.md when discovering new bugs or limitations
- **Do** update ROADMAP.md when starting or completing roadmap items
- **Do** update VERIFICATION.md when adding or changing verification tests
- **Do** update README.md when adding user-facing features
- **Do** keep the binary under 5MB — if it grows, investigate alternatives
- **Do** run `.githooks/setup.sh` after cloning the repo to enable pre-commit/pre-push hooks
- **Do** update `.githooks/` when adding new verification checks that should run before commits
- **Do** use env vars for all secrets — never config files

---

## Release Process

**Policy: Every completed version MUST be released.** No exceptions. Whether it's a minor version with new features or a patch version with bug fixes — if it's merged to master, it ships.

When all TODOs and features for a version milestone are completed, GitHub Actions for the final commit are green, all tests pass, test coverage is good, and everything else is ready — follow this release process.

### Phase 1: Pre-Release Checks

Before bumping the version, verify everything is clean:

```bash
# 1. Full Rust test suite — all 149+ tests must pass
cargo test --locked 2>&1

# 2. Clippy must be clean (no warnings, no errors)
cargo clippy --locked --all-targets -- -D warnings 2>&1

# 3. Formatting must be clean
cargo fmt --check 2>&1

# 4. Full verification suite (build + all test modules)
./scripts/verify.sh --build 2>&1

# 5. Check test coverage is adequate (no untested critical paths)
#    - Every config field has a test
#    - Every LLM provider has a test
#    - Every error variant has a test
#    - Every CLI argument has a test
#    - Every deployment target has verification tests
```

If any of these fail, **do not proceed** — fix the issue first.

### Phase 2: Version Bump

Update the version in `Cargo.toml`:

```bash
# For a patch release (bug fixes only):
sed -i '' 's/^version = "0\.1\.0"/version = "0.1.1"/' Cargo.toml

# For a minor release (new features, backward-compatible):
sed -i '' 's/^version = "0\.1\.0"/version = "0.2.0"/' Cargo.toml

# For a major release (breaking changes):
sed -i '' 's/^version = "0\.1\.0"/version = "1.0.0"/' Cargo.toml
```

Update the version reference in `AGENTS.md` itself (Project Overview section) and any other docs that reference the version.

### Phase 3: Update Changelog

Move all `[Unreleased]` entries into a new `[vX.Y.Z]` section:

```markdown
## [v0.1.0] - 2026-06-02

### Added
- (all added features from Unreleased)

### Fixed
- (all fixes from Unreleased)

### Changed
- (all changes from Unreleased)
```

Then clear the `[Unreleased]` section headers (keep the structure, remove old entries).

### Phase 4: Commit & Tag

```bash
# Commit the version bump and changelog update
git add -A
git commit -m "Release v0.1.0"

# Tag the release (signed tag preferred)
git tag -s v0.1.0 -m "RavenClaws v0.1.0"

# Push everything (triggers CI/CD pipelines)
git push --follow-tags
```

Pushing the tag triggers the following GitHub Actions workflows:

| Workflow | Trigger | What it does |
|---|---|---|
| `build.yml` | Tag push `v*` | `check` (fmt+clippy+test) → `build-binaries` (5 targets) → `containers` (multi-arch, multi-registry, sign, SBOM, Trivy) → `publish-cratesio` → `release` (GitHub Release with assets) |
| `container.yml` | Tag push `v*` | Builds + pushes multi-arch containers, signs with Cosign, generates attestation, Trivy scan, SBOM |
| `security-scan.yml` | Tag push `v*` | CodeQL, cargo-audit, cargo-deny, cargo-outdated, cargo-udeps, OSSF Scorecard |

### Phase 5: Monitor CI/CD

After pushing the tag, **monitor all GitHub Actions workflows to completion**:

1. Go to https://github.com/egkristi/RavenClaws/actions
2. Verify the `check` job passes (fmt + clippy + test)
3. Verify all 5 `build-binaries` matrix jobs succeed:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
   - `x86_64-unknown-linux-musl`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
4. Verify `containers` job builds and pushes multi-arch images to both GHCR and Docker Hub
5. Verify Cosign signing completes
6. Verify artifact attestation is generated
7. Verify Trivy vulnerability scan passes (exit code 0)
8. Verify SBOM is generated and uploaded
9. Verify `publish-cratesio` publishes to crates.io
10. Verify `release` job creates the GitHub Release with all binary assets attached

If **any job fails**, investigate and fix before proceeding. Do not create a new tag until the issue is resolved.

### Phase 6: Post-Release Binary & Container Verification

After all CI/CD pipelines pass, run a full battery of tests against the **newly released** binaries and container images:

```bash
# ── 6a. Pull and verify the released container image ──────────
docker pull ghcr.io/egkristi/ravenclaws:v0.1.0
docker run --rm ghcr.io/egkristi/ravenclaws:v0.1.0 --version

# ── 6b. Run full verification suite against the release ───────
# (This tests local binary, Docker container, K8s manifests, etc.)
./scripts/verify.sh --build

# ── 6c. Verify binary integrity ───────────────────────────────
# Download a released binary and check its SHA256
# (Replace URL with actual release asset URL)
curl -sL https://github.com/egkristi/RavenClaws/releases/download/v0.1.0/ravenclaws-x86_64-apple-darwin.tar.gz \
  -o /tmp/ravenclaws-release.tar.gz
tar xzf /tmp/ravenclaws-release.tar.gz -C /tmp/
./scripts/verify.sh --security

# ── 6d. Verify container security properties ──────────────────
docker image inspect ghcr.io/egkristi/ravenclaws:v0.1.0 --format '{{.Config.User}}'
# Must output "65532" (nonroot user)

# ── 6e. Verify multi-arch support ─────────────────────────────
docker buildx imagetools inspect ghcr.io/egkristi/ravenclaws:v0.1.0
# Must show both linux/amd64 and linux/arm64 manifests

# ── 6f. Verify container signature ────────────────────────────
cosign verify ghcr.io/egkristi/ravenclaws:v0.1.0 \
  --certificate-identity-regexp "https://github.com/egkristi/RavenClaws" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"

# ── 6g. Verify SBOM exists ────────────────────────────────────
# Download SBOM from GitHub Release assets or regenerate:
syft ghcr.io/egkristi/ravenclaws:v0.1.0 -o spdx-json=sbom-verify.spdx.json
```

### Phase 7: Final Validation

```bash
# ── 7a. Verify crates.io package ──────────────────────────────
cargo search ravenclaws
# Should show the new version

# ── 7b. Verify GitHub Release ─────────────────────────────────
# Check https://github.com/egkristi/RavenClaws/releases
# Must have:
#   - Release notes (auto-generated from changelog)
#   - 5 binary archives (.tar.gz/.zip) with SHA256 checksums
#   - SBOM artifact
#   - Container image signatures

# ── 7c. Verify the binary works end-to-end ────────────────────
/tmp/ravenclaws --version
/tmp/ravenclaws --help
echo "Hello" | /tmp/ravenclaws --exec "Say hello back" 2>&1 || true
```

### Release Abort Criteria

If any of the following occur during the release process, **abort immediately** and do not create/push a tag:

1. Any Rust test fails (`cargo test --locked`)
2. Clippy or fmt check fails
3. Any verification suite module fails (`./scripts/verify.sh --all`)
4. Any CI job in the build workflow fails
5. Trivy scan finds CRITICAL or HIGH vulnerabilities (exit code 1)
6. Cosign signing fails
7. Container image does not run as nonroot user (UID 65532)
8. Multi-arch manifest is missing either linux/amd64 or linux/arm64
9. Binary SHA256 checksums do not match after download
10. GitHub Release is missing required assets

**If aborting:** Delete the tag locally and remotely, fix the issue, then restart from Phase 1:

```bash
git tag -d v0.1.0
git push --delete origin v0.1.0
```

### Release Checklist Template

Copy this into a new issue or comment when starting a release:

```markdown
## Release vX.Y.Z Checklist

### Phase 1: Pre-Release
- [ ] `cargo test --locked` — all tests pass
- [ ] `cargo clippy --locked --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean
- [ ] `./scripts/verify.sh --build` — full suite passes
- [ ] Test coverage is adequate

### Phase 2: Version Bump
- [ ] Version updated in `Cargo.toml`
- [ ] Version references updated in docs

### Phase 3: Changelog
- [ ] `[Unreleased]` entries moved to `[vX.Y.Z]` section
- [ ] Release date added

### Phase 4: Commit & Tag
- [ ] Commit pushed: `git push`
- [ ] Tag pushed: `git push --follow-tags`

### Phase 5: CI/CD Monitoring
- [ ] `check` job passes
- [ ] All 5 `build-binaries` jobs succeed
- [ ] `containers` job succeeds (multi-arch, multi-registry)
- [ ] Cosign signing completes
- [ ] Artifact attestation generated
- [ ] Trivy scan passes (no CRITICAL/HIGH)
- [ ] SBOM generated and uploaded
- [ ] `publish-cratesio` succeeds
- [ ] GitHub Release created with all assets

### Phase 6: Post-Release Verification
- [ ] Container image pulls and runs
- [ ] `./scripts/verify.sh --build` passes against release
- [ ] Binary SHA256 checksums verified
- [ ] Container runs as nonroot (UID 65532)
- [ ] Multi-arch manifest verified (amd64 + arm64)
- [ ] Cosign signature verified
- [ ] SBOM verified

### Phase 7: Final Validation
- [ ] crates.io package published
- [ ] GitHub Release has all assets
- [ ] Binary works end-to-end
```

---

*RavenClaws — Small. Sleek. Secure. Supreme.* 🐦‍⬛
