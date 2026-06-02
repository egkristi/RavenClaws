# 🐦‍⬛ RavenClaw — AI Agent Instructions

This file contains structured instructions for AI coding agents working on the RavenClaw codebase. It defines the project's architecture, conventions, common tasks, and guardrails.

---

## Project Overview

RavenClaw is a **lightweight, secure Rust agent framework** with multi-provider LLM support. It runs as a single binary with zero runtime dependencies.

- **Language:** Rust (edition 2021)
- **Version:** 0.1.0 (pre-release / developer preview)
- **License:** AGPL-3.0-or-later + Commercial
- **Repository:** https://github.com/egkristi/RavenClaw
- **Build:** `cargo build --release` (~3MB stripped binary, ~7ms startup)

### Architecture (4 modules)

```
src/
├── main.rs      — CLI entry point (clap), config loading, mode dispatch
├── agent.rs     — Agent implementations (single, swarm stubs, supervisor stubs)
├── llm.rs       — LLM provider abstraction (trait + 4 clients + multi-model manager)
├── config.rs    — Config structs, TOML/env loading, validation
└── error.rs     — Unified error types
```

### Current State

| Feature | Status |
|---|---|
| Single agent mode | ✅ Working — sends prompt, logs response |
| Multi-provider (LiteLLM, OpenAI, OpenRouter, Ollama) | ✅ Working |
| Multi-model manager | ✅ Working — iterates all configured providers |
| CLI with env-var overrides | ✅ Working |
| OpenAI-compatible API support | ✅ Working — any `/v1/chat/completions` endpoint |
| Container security (non-root, read-only FS, dropped caps) | ✅ Working |
| Verification suite (94 tests, 8 modules) | ✅ Working |
| `--exec` mode | ❌ Dead code — CLI arg parsed but never used |
| Swarm mode | ❌ Stub — warns "not yet implemented", exits 0 |
| Supervisor mode | ❌ Stub — warns "not yet implemented", exits 0 |
| Tool-use / function calling | ❌ Not implemented |
| Agent loop / ReAct planning | ❌ Not implemented — one-shot send-and-exit |
| Streaming responses | ❌ Not implemented — `stream: None` hardcoded |
| Conversation memory | ❌ Not implemented — in-memory only, lost on exit |
| RavenFabric integration | ❌ Not implemented — crate commented out in Cargo.toml |
| GitHub Actions CI/CD | ❌ Not implemented |
| Pre-built binaries / releases | ❌ Not implemented |

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
- **`--exec` mode is dead code** — CLI arg parsed but never used in `main.rs`. [No issue]

## High
- **Swarm/Supervisor stubs exit silently** — `--mode swarm` and `--mode supervisor` warn "not yet implemented" and exit 0 instead of returning an error. [No issue]

## Medium
- **Only 2 Rust unit tests** — `cargo test` runs in <1s and tests almost nothing. [No issue]
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
| `main.rs` | CLI parsing, config loading, mode dispatch | Agent logic, LLM calls, config structs |
| `agent.rs` | Agent run functions (single, swarm, supervisor) | LLM client creation, config parsing |
| `llm.rs` | `LLMProviderTrait`, client implementations, `MultiModelManager` | Agent logic, config structs |
| `config.rs` | `Config`, `LLMConfig`, validation, env loading | Agent logic, HTTP requests |
| `error.rs` | `RavenClawError` enum, `Result<T>` alias | Everything else |

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
    └── test-llm-quality.sh— LLM response quality (36 tests)
```

### Running Tests

```bash
./scripts/verify.sh --all          # Full suite (94 tests)
./scripts/verify.sh --quick        # Smoke test (litellm + local + security)
./scripts/verify.sh --litellm      # Single module
./scripts/verify.sh --build        # Build + all tests
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
- **ConfigMap jsonpath** — Keys with dots (e.g., `ravenclaw.toml`) are unreliable. Use `go-template='{{index .data "key"}}'`.

---

## Deployment Targets

| Target | Binary Location | How to Build |
|---|---|---|
| macOS (aarch64) | `target/release/ravenclaw` | `cargo build --release` |
| macOS (x86_64) | `target/x86_64-apple-darwin/release/ravenclaw` | `cargo build --release --target x86_64-apple-darwin` |
| Linux ARM64 | `target/aarch64-unknown-linux-gnu/release/ravenclaw` | Cross-compile or Docker build |
| Linux x86_64 | `target/x86_64-unknown-linux-gnu/release/ravenclaw` | Cross-compile or Docker build |
| Docker | `ghcr.io/egkristi/ravenclaw:latest` | `docker build -t ravenclaw:latest .` |
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
- **Do** use env vars for all secrets — never config files

---

## Release Checklist

Before tagging v0.1.0:

- [ ] Fix `--exec` dead code (wire up or remove)
- [ ] Fix swarm/supervisor stubs (return clear error instead of silent success)
- [ ] Add GitHub Actions CI/CD workflow
- [ ] Add release workflow (build binaries + push Docker images on tag)
- [ ] Expand `cargo test` beyond 2 unit tests
- [ ] Run full verification suite: `./scripts/verify.sh --all`
- [ ] Update version in `Cargo.toml` if needed
- [ ] Tag and push: `git tag v0.1.0 && git push --tags`

---

*RavenClaw — Small. Sleek. Secure. Supreme.* 🐦‍⬛
