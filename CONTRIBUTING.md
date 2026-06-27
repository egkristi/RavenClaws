# Contributing to RavenClaws

Thank you for your interest in contributing to RavenClaws! 🐦‍⬛

RavenClaws is a lightweight, secure Rust agent framework with multi-provider LLM
support. We welcome contributions that align with our five pillars: **Secure,
Small, Efficient, Robust, and Simple**.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Coding Conventions](#coding-conventions)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Feature Requests](#feature-requests)
- [License](#license)

## Code of Conduct

This project is governed by our [Code of Conduct](CODE_OF_CONDUCT.md).
By participating, you agree to uphold its principles.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/YOUR_USERNAME/RavenClaws.git
   cd RavenClaws
   ```
3. **Set up git hooks:**
   ```bash
   .githooks/setup.sh
   ```
4. **Build and test:**
   ```bash
   cargo build --release
   cargo test --locked
   cargo clippy --locked --all-targets -- -D warnings
   cargo fmt --check
   ```

## Development Setup

### Prerequisites

- **Rust** 1.86+ (install via [rustup](https://rustup.rs/))
- **Docker** (for container testing)
- **Orbstack** or **minikube** (for K8s testing, optional)

### Quick Start

```bash
# Build the binary
cargo build --release

# Run unit tests
cargo test --locked

# Run the full verification suite
./scripts/verify.sh --quick
```

### Project Structure

```
src/
├── main.rs      — CLI entry point
├── lib.rs       — Library crate entry, public API
├── agent.rs     — Agent loop, swarm, supervisor, REPL
├── llm.rs       — LLM provider abstraction + 6 clients
├── config.rs    — Config structs, TOML/env loading
├── tools.rs     — Tool abstraction + 5 built-in tools
├── mcp.rs       — MCP client + server (stdio + SSE)
├── server.rs    — HTTP server mode
├── heartbeat.rs — Autonomous heartbeat agent
├── swarm.rs     — Swarm orchestration
├── background.rs— Background task manager
├── scheduler.rs — Scheduling & triggers
├── policy.rs    — Deny-by-default policy engine
├── audit.rs     — Tamper-evident audit log
├── sandbox.rs   — Sandboxed execution
├── eval.rs      — Eval harness
├── telemetry.rs — OpenTelemetry tracing
├── ravenfabric.rs— RavenFabric mesh client
└── error.rs     — Unified error types
```

## Coding Conventions

### Rust Style

- **Formatting:** Standard `rustfmt` (no custom config)
- **Linting:** Standard `clippy` with `-D warnings`
- **Naming:** `snake_case` for functions/variables, `CamelCase` for types/enums,
  `SCREAMING_SNAKE_CASE` for constants
- **Error handling:** `thiserror` for library errors, `anyhow` for application errors
- **Async:** `tokio` runtime with `async-trait` for provider abstraction
- **Logging:** `tracing` with JSON format — use `info!`, `warn!`, `error!` consistently
- **Unimplemented features:** Use `warn!("...not yet implemented")` + return `Ok(())`
  — do not panic or exit with error

### Do NOT

- Use `unsafe` code unless absolutely necessary and documented
- Add Python, Node.js, or other runtime dependencies
- Hardcode API keys, tokens, or credentials
- Add large dependencies (>100KB) without evaluating alternatives
- Use `println!` or `eprintln!` — use `tracing` instead
- Remove `strip = true` or `panic = "abort"` from release profile

### Do

- Add tests for every new feature (both Rust unit tests and shell verification tests)
- Update `CHANGELOG.md` for every feature, fix, or change
- Update `ROADMAP.md` when starting or completing roadmap items
- Update `ISSUES.md` when discovering new bugs or limitations
- Keep the binary under 5MB — if it grows, investigate alternatives
- Use env vars for all secrets — never config files

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --locked

# Run tests for a specific module
cargo test --locked -- config

# Run a specific test
cargo test --locked -- test_config_validation
```

### Verification Suite

The verification suite lives in `scripts/` and tests deployment targets:

```bash
# Full suite
./scripts/verify.sh --all

# Quick smoke test
./scripts/verify.sh --quick

# Single module
./scripts/verify.sh --local
```

### Test Coverage Mandate

- **Every new feature** must include both Rust unit tests and shell verification tests
- **Every bug fix** must include a test that reproduces the bug and verifies the fix
- **Every config field** must have a test that exercises it
- **Every LLM provider** must have a test in `test-llm-quality.sh`

## Pull Request Process

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** following the coding conventions above

3. **Run all checks locally:**
   ```bash
   cargo fmt --check
   cargo clippy --locked --all-targets -- -D warnings
   cargo test --locked
   ```

4. **Update documentation** if your change affects user-facing behavior:
   - `CHANGELOG.md` — add entry under `[Unreleased]`
   - `ROADMAP.md` — update if implementing a roadmap item
   - `ISSUES.md` — update if fixing a known issue
   - `docs/guides/` — update relevant guides
   - `website/public/docs/` — update matching HTML pages

5. **Commit your changes:**
   ```bash
   git add -A
   git commit -m "Descriptive summary of changes"
   ```
   Pre-commit hooks will run automatically (fmt, clippy, tests, binary size, secrets).

6. **Push and create a PR:**
   ```bash
   git push origin feature/my-feature
   ```
   Then open a pull request on GitHub.

### PR Requirements

- Clear description of what the PR does and why
- All CI checks must pass (fmt, clippy, test, security scans)
- Test coverage for new code
- Documentation updates for user-facing changes
- Signed commits preferred

## Feature Requests

Feature requests are tracked in [ROADMAP.md](ROADMAP.md). To suggest a new feature:

1. Check if it's already listed in ROADMAP.md
2. Open a GitHub Discussion or Issue
3. Explain the use case and why it aligns with the five pillars

## License

By contributing, you agree that your contributions will be licensed under the
[AGPL-3.0-or-later](LICENSE) license. For commercial licensing, see
[LICENSING.md](LICENSING.md).

---

*Thank you for helping make RavenClaws the best it can be!* 🐦‍⬛
