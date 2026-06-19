# RavenClaw Verification

This document describes how RavenClaw is verified across all deployment targets, what each test covers, and how to run the verification suite.

## Quick Start

```bash
# Prerequisites: LiteLLM running on localhost:4000, Docker (Orbstack), kubectl
cargo build --release
./scripts/verify.sh
```

## Modular Verification Architecture

The verification system is built as a **modular suite** of independent test scripts, orchestrated by a central dispatcher:

```
scripts/
├── verify.sh                          # Main orchestrator
├── lib/
│   ├── common.sh                      # Shared library (colors, paths, logging, test runner)
│   ├── test-litellm.sh                # LiteLLM connectivity tests
│   ├── test-local.sh                  # macOS binary tests
│   ├── test-docker.sh                 # Docker container tests
│   ├── test-linux.sh                  # Cross-compiled Linux binary tests
│   ├── test-k8s.sh                    # Kubernetes deployment tests
│   ├── test-security.sh               # Security & binary integrity tests
│   ├── test-performance.sh            # Performance benchmarks
│   └── test-llm-quality.sh            # LLM response quality tests
```

Each module is **self-contained** and can be run independently:

```bash
# Run a single module standalone
./scripts/lib/test-local.sh
./scripts/lib/test-docker.sh
./scripts/lib/test-k8s.sh
```

## Verification Suite

The verification suite runs **94 tests** across **8 modules**, covering **4 deployment targets**. Each test produces a detailed log in `target/verification-results/`.

In addition, **307 Rust unit tests** run via `cargo test` covering all 11 source modules (agent, config, error, llm, tools, mcp, server, policy, audit, sandbox, ravenfabric).

### Usage

```bash
./scripts/verify.sh                    # Run all 94 tests
./scripts/verify.sh --list             # List all available modules
./scripts/verify.sh --quick            # Quick smoke test (24 tests: litellm + local + security)
cargo test                             # Run 307 Rust unit tests
./scripts/verify.sh --all              # Run all modules (same as no flag)
./scripts/verify.sh --litellm          # LiteLLM connectivity only
./scripts/verify.sh --local            # Local macOS binary only
./scripts/verify.sh --docker           # Docker container only
./scripts/verify.sh --linux            # Linux binary only
./scripts/verify.sh --k8s              # Kubernetes only
./scripts/verify.sh --security         # Security & binary integrity only
./scripts/verify.sh --performance      # Performance benchmarks only
./scripts/verify.sh --llm-quality      # LLM response quality only
./scripts/verify.sh --build            # Build binaries first, then run all tests

cargo test                             # Run 277 Rust unit tests
```

### Test Categories

#### 1. LiteLLM Connectivity (4 tests)
- **Health check**: Verifies LiteLLM is reachable at `http://localhost:4000/health/readiness`
- **Models list**: Confirms the models endpoint returns available models
- **Model availability**: Checks required models (`best-coding`, `best-chat`, `fast`, `cheap`) are present
- **Model count**: Reports total available models (currently 32)

#### 2. Local macOS Binary (12 tests)
- **Binary exists**: Checks the release binary is present at `target/release/ravenclaw`
- **--version**: Binary prints version string
- **--help**: Binary prints help text
- **Config loading (TOML)**: Loads `tests/config/ravenclaw-test.toml` successfully
- **Config loading (env vars)**: Loads config from `RAVENCLAW__*` environment variables
- **Single agent LLM chat**: Sends a chat request via LiteLLM and receives a response
- **Multi-model mode**: Tests all configured providers respond (uses `tests/config/ravenclaw-multi-test.toml`)
- **CLI provider override**: `--provider`, `--endpoint`, `--model` flags override config
- **Verbose logging**: `--verbose` flag enables debug output
- **Error handling (missing config)**: Running without config produces a non-zero exit code
- **Error handling (invalid mode)**: Invalid `--mode` value produces a non-zero exit code
- **--exec mode**: One-shot command execution works

#### 3. Docker Container (10 tests)
- **Docker build**: Multi-stage Dockerfile builds successfully
- **Image exists**: Built image is present in local registry
- **--version**: Container binary prints version
- **--help**: Container binary prints help
- **LLM connectivity (single)**: Container reaches LiteLLM via host network
- **LLM connectivity (multi-model)**: Container tests multiple providers
- **LLM connectivity (env-only)**: Container works with env-var config (no config file)
- **Docker Compose**: `docker-compose.yml` config is valid
- **Non-root user**: Container runs as `nonroot` user (uid 65532)
- **No privileged mode**: Container does not run as root

#### 4. Linux Binary (6 tests)
- **--version**: Cross-compiled Linux binary (`aarch64-unknown-linux-gnu`) runs via Docker
- **--help**: Linux binary prints help text
- **LLM connectivity (single)**: Linux binary reaches LiteLLM via host network
- **LLM connectivity (multi-model)**: Linux binary tests multiple providers
- **ELF format**: Binary is an ELF executable
- **Stripped**: Binary is stripped (release build)

#### 5. Kubernetes (13 tests)
- **Cluster connectivity**: `kubectl cluster-info` succeeds
- **Node ready**: K8s node is in Ready state
- **Manifest application**: `k8s/deployment-test.yaml` applies without errors
- **Pod startup**: Pod reaches Running state within 60s
- **Startup logs**: Pod logs show "RavenClaw starting"
- **Config loading**: Pod logs show "Configuration loaded" (ConfigMap works)
- **LLM client init**: Pod logs show "LLM client initialized"
- **Provider ready**: Pod logs show "Provider ready"
- **LLM response**: Pod logs show "Agent response received"
- **Resource limits**: Pod has resource limits configured (256Mi memory, 250m CPU)
- **Non-root user**: Pod runs with `runAsUser: 65532`
- **Read-only root filesystem**: Pod has `readOnlyRootFilesystem: true`
- **All capabilities dropped**: Pod has `capabilities.drop: ["ALL"]`
- **ConfigMap**: ConfigMap exists and contains valid config

#### 6. Security & Binary Integrity (8 tests)
- **Architecture**: Binary is `aarch64 Mach-O 64-bit`
- **Release build**: Binary is a release build (Mach-O executable)
- **No hardcoded API keys**: No OpenAI-style keys (`sk-...`) in binary strings
- **No hardcoded credentials**: No GitHub tokens (`ghp_...`) in binary strings
- **Binary size**: Under 5MB (currently ~3.4MB)
- **No secrets in strings**: No credential patterns in binary strings
- **No setuid/setgid**: Binary does not require elevated privileges
- **Cargo.lock present**: Reproducible builds via lockfile

#### 7. Performance (5 benchmarks)
- **Startup time**: Binary starts in under 100ms (currently ~7ms)
- **Config loading time**: Config parses in under 50ms (currently ~6ms)
- **LLM response time**: Average of 3 runs (currently ~900ms)
- **Binary size**: Reports exact size (currently ~3,500KB)
- **Memory usage**: Approximate footprint via vmmap

#### 8. LLM Response Quality (30+ tests)
- **Individual model tests**: Tests each available LiteLLM model (currently 30 chat models)
- **Reasoning test**: Tests with a reasoning prompt via `best-coding`
- **Multi-model quality**: Tests all configured providers simultaneously
- **Response diversity**: Verifies different models give different responses

## Test Configurations

### Single-provider config (`tests/config/ravenclaw-test.toml`)
```toml
[llm]
provider = "litellm"
endpoint = "http://localhost:4000"
model = "best-coding"
timeout_secs = 60

[security]
require_tls = false
audit_log = false

[runtime]
workdir = "/tmp/ravenclaw-test"
max_agents = 5
health_interval_secs = 10
```

### Multi-model config (`tests/config/ravenclaw-multi-test.toml`)
```toml
[llm]
provider = "litellm"
endpoint = "http://localhost:4000"
model = "best-coding"
timeout_secs = 60

[[llms]]
provider = "litellm"
endpoint = "http://localhost:4000"
model = "best-coding"
timeout_secs = 60

[[llms]]
provider = "litellm"
endpoint = "http://localhost:4000"
model = "claude-sonnet"
timeout_secs = 60

[[llms]]
provider = "litellm"
endpoint = "http://localhost:4000"
model = "deepseek-v4-pro"
timeout_secs = 60

[security]
require_tls = false
```

### K8s test config (`tests/config/ravenclaw-k8s-test.toml`)
```toml
[llm]
endpoint = "http://host.docker.internal:4000"
model = "best-coding"
timeout_secs = 30

[security]
require_tls = false
audit_log = true

[runtime]
workdir = "/workspace"
max_agents = 3
health_interval_secs = 10
```

## Deployment Targets

| Target | Build Method | Verified By | Tests |
|--------|-------------|-------------|-------|
| macOS (aarch64) | `cargo build --release` | `--local` | 12 |
| Linux (aarch64) | `cross build --release --target aarch64-unknown-linux-gnu` | `--linux` | 6 |
| Linux (x86_64) | `cross build --release --target x86_64-unknown-linux-gnu` | `--linux` | 6 |
| Docker (multi-arch) | `docker buildx build --platform linux/amd64,linux/arm64` | `--docker` | 10 |
| Kubernetes | `kubectl apply -f k8s/deployment-test.yaml` | `--k8s` | 13 |

## Environment Requirements

| Prerequisite | Required For | Check |
|-------------|-------------|-------|
| LiteLLM on localhost:4000 | All LLM tests | `curl http://localhost:4000/health/readiness` |
| Docker (Orbstack) | Docker + Linux + K8s tests | `docker info` |
| kubectl | K8s tests | `kubectl cluster-info` |
| Rust release binary | Local tests | `target/release/ravenclaw` |
| Cross-compiled Linux binary | Linux tests | `target/aarch64-unknown-linux-gnu/release/ravenclaw` |

## CI/CD Pipeline

The GitHub Actions workflow (`.github/workflows/build.yml`) runs verification as part of the release process:

1. **Lint & Test**: `cargo fmt --check`, `cargo clippy`, `cargo test`
2. **Build binaries**: Cross-compiles for all 5 targets
3. **Build containers**: Multi-arch Docker images pushed to GHCR + Docker Hub
4. **Security scan**: Trivy vulnerability scanning
5. **Signing**: Cosign keyless signing (Sigstore)
6. **SBOM**: Syft software bill of materials
7. **Release**: GitHub Release with binary artifacts + crates.io publish

## Test Results

Latest full run: **88/94 passed, 6 skipped** (2 Jun 2026)

```
  Total:   94
  Passed:  88
  Failed:  0
  Skipped: 6
  ✓ ALL VERIFICATIONS PASSED
```

The 6 skipped tests are LLM models configured in LiteLLM but without active API keys (`claude-sonnet`, `claude-opus`, `qwen2.5-coder`, `qwen3.5-397b-cloud`, `qwen3-vl-235b-cloud`, `best-chat`). These are expected skips — the models are registered but unavailable.

### Quick Smoke Test Results

```
  Total:   24
  Passed:  24
  Failed:  0
  Skipped: 0
  ✓ ALL VERIFICATIONS PASSED
```

The quick smoke test covers LiteLLM connectivity (4), local macOS binary (12), and security (8).

### Detailed Results by Module

| Module | Tests | Passed | Skipped |
|--------|-------|--------|---------|
| LiteLLM Connectivity | 4 | 4 | 0 |
| Local macOS Binary | 12 | 12 | 0 |
| Docker Container | 10 | 10 | 0 |
| Linux Binary | 6 | 6 | 0 |
| Kubernetes | 13 | 13 | 0 |
| Security & Integrity | 8 | 8 | 0 |
| Performance | 5 | 5 | 0 |
| LLM Quality | 36 | 30 | 6 |
| LLM Quality | 36 | 30 | 6 |

Detailed logs: `target/verification-results/YYYYMMDD-HHMMSS-*.log`
