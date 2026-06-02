# RavenClaw 🐦‍⬛

**Lightweight, secure Rust agent framework with multi-provider LLM support**

[![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSES/AGPLv3.txt)

Built for efficiency, security, and easy deployment. Supports LiteLLM, OpenRouter, Ollama, and OpenAI.

## Features

- ⚡ **Fast** — Native Rust, optimized for performance
- 🔒 **Secure by default** — No credentials in config, TLS required, minimal permissions
- 🔄 **Multi-provider support** — LiteLLM, OpenRouter, Ollama, OpenAI with unified API
- 🎯 **Multi-model routing** — Run agents across multiple providers simultaneously
- 🕸️ **RavenFabric ready** — Swarm coordination and remote command execution
- 📦 **Easy deployment** — Binary, Docker, Kubernetes (Helm ready)
- 🎯 **Minimal footprint** — Distroless container, <20MB binary

## Quick Start

### Binary

```bash
# Download release
curl -LO https://github.com/egkristi/RavenClaw/releases/latest/download/ravenclaw
chmod +x ravenclaw

# Run with environment variables
export LITELLM_API_KEY="your-key"
export RAVENCLAW__LLM__ENDPOINT="http://localhost:4000"
./ravenclaw --mode single
```

### Docker

```bash
docker run --rm -it \
  -e LITELLM_API_KEY="your-key" \
  -e RAVENCLAW__LLM__ENDPOINT="http://litellm:4000" \
  ghcr.io/egkristi/ravenclaw:latest
```

### Docker Compose (Development)

```bash
# Start RavenClaw + LiteLLM
docker-compose up -d

# View logs
docker-compose logs -f ravenclaw
```

### Kubernetes

```bash
# Deploy to cluster
kubectl apply -f k8s/deployment.yaml

# Check status
kubectl -n ravenclaw get pods
kubectl -n ravenclaw logs -l app.kubernetes.io/name=ravenclaw
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|---|---|---|
| `RAVENCLAW__LLM__PROVIDER` | Provider: litellm, openrouter, ollama, openai | `litellm` |
| `RAVENCLAW__LLM__ENDPOINT` | LLM endpoint URL | (provider-dependent) |
| `RAVENCLAW__LLM__MODEL` | Default model | `gpt-4o-mini` |
| `LITELLM_API_KEY` | API key for LiteLLM/OpenRouter/OpenAI | (required for cloud) |
| `RAVENCLAW__LLMS` | JSON array for multi-model config | - |
| `RAVENCLAW__RAVENFABRIC__ENDPOINT` | RavenFabric endpoint | - |
| `RAVENCLAW__SECURITY__REQUIRE_TLS` | Enforce TLS | `true` |
| `RAVENCLAW__RUNTIME__MAX_AGENTS` | Max concurrent agents | `10` |
| `RUST_LOG` | Log level | `info` |

### Single Provider Mode (TOML)

**LiteLLM:**
```toml
[llm]
provider = "litellm"
endpoint = "http://litellm:4000"
model = "gpt-4o-mini"
api_key = "your-key"  # or use LITELLM_API_KEY env var
timeout_secs = 30
```

**Ollama (local, no API key):**
```toml
[llm]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.1"
timeout_secs = 60
```

**OpenRouter:**
```toml
[llm]
provider = "openrouter"
model = "anthropic/claude-3.5-sonnet"
api_key = "your-openrouter-key"  # or use LITELLM_API_KEY env var
```

**OpenAI:**
```toml
[llm]
provider = "openai"
model = "gpt-4o"
api_key = "your-openai-key"  # or use LITELLM_API_KEY env var
```

### Multi-Model Mode (Multiple Providers)

Configure multiple LLMs for load balancing, fallback, or model-specific routing:

```toml
# Single provider config (fallback)
[llm]
provider = "litellm"
endpoint = "http://litellm:4000"
model = "gpt-4o-mini"

# Multi-model array
[[llms]]
provider = "ollama"
endpoint = "http://ollama:11434"
model = "llama3.1"

[[llms]]
provider = "openrouter"
model = "anthropic/claude-3.5-sonnet"
api_key = "sk-or-xxx"

[[llms]]
provider = "openai"
model = "gpt-4o"
api_key = "sk-xxx"
```

### Full Config Example

```toml
[llm]
provider = "litellm"
endpoint = "http://litellm:4000"
model = "gpt-4o-mini"
timeout_secs = 30

[ravenfabric]
endpoint = "http://ravenfabric:8080"
remote_exec = true
allowed_hosts = ["litellm", "ravenfabric"]

[security]
require_tls = true
token_lifetime_secs = 3600
audit_log = true

[runtime]
workdir = "/workspace"
max_agents = 10
health_interval_secs = 60
```

## Agent Modes

| Mode | Description |
|---|---|
| `single` | Standalone autonomous agent |
| `swarm` | Multiple coordinated agents |
| `supervisor` | Orchestrator for sub-agents |

## Security

- ✅ No credentials in config files (use env vars or K8s Secrets)
- ✅ TLS required for production endpoints
- ✅ Read-only root filesystem (container)
- ✅ Non-root user (container)
- ✅ Dropped capabilities (container)
- ✅ Audit logging enabled by default
- ✅ Token lifetime limits

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.82+ (install via `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- For Linux cross-compilation on macOS: `brew install FiloSottile/musl-cross/musl-cross`

### Build for Host

```bash
git clone https://github.com/egkristi/RavenClaw
cd RavenClaw

# Build release for current platform
cargo build --release

# Run tests
cargo test

# Build Docker image
docker build -t ravenclaw:latest .
```

### Cross-Compile for All Architectures

```bash
# Install cross-compilation targets
rustup target add \
    x86_64-apple-darwin \
    aarch64-apple-darwin \
    x86_64-unknown-linux-gnu \
    aarch64-unknown-linux-gnu \
    x86_64-unknown-linux-musl

# Build for all targets
./scripts/build.sh --all

# Build for a specific target
./scripts/build.sh --target aarch64-unknown-linux-gnu
```

### Multi-Arch Docker Image

```bash
# Build and push multi-arch Docker image (linux/amd64 + linux/arm64)
docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t ghcr.io/egkristi/ravenclaw:latest \
    --push .
```

## Downloads

Pre-built binaries are available for these architectures:

| Architecture | Target Triple | File |
|---|---|---|
| Apple Silicon (M1+) | `aarch64-apple-darwin` | `ravenclaw-aarch64-apple-darwin` |
| Intel Mac | `x86_64-apple-darwin` | `ravenclaw-x86_64-apple-darwin` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `ravenclaw-aarch64-unknown-linux-gnu` |
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` | `ravenclaw-x86_64-unknown-linux-gnu` |
| Linux x86_64 (musl/static) | `x86_64-unknown-linux-musl` | `ravenclaw-x86_64-unknown-linux-musl` |

Docker images support both `linux/amd64` and `linux/arm64` platforms.
```

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│   RavenClaw     │────▶│   LiteLLM       │
│   Agent         │     │   (LLM Proxy)   │
└────────┬────────┘     └─────────────────┘
         │
         │ RavenFabric
         ▼
┌─────────────────┐
│   Swarm         │
│   Coordination  │
└─────────────────┘
```

## Roadmap

- [ ] RavenFabric integration (remote exec)
- [ ] Swarm mode implementation
- [ ] Supervisor mode
- [ ] Helm chart
- [ ] Prometheus metrics
- [ ] OpenTelemetry tracing
- [ ] Plugin system

## License

RavenClaw uses a dual-license model:

- **AGPL-3.0-or-later** — open source core. Free for personal use, OSS projects, and commercial use up to 50 agents / $5M revenue.
- **Commercial** — for large commercial deployments or embedding without AGPL obligations.

See [LICENSING.md](LICENSING.md) for the full breakdown.

## Contributing

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit changes (`git commit -am 'Add feature'`)
4. Push (`git push origin feature/my-feature`)
5. Open a Pull Request

All contributions require signing a Contributor License Agreement (CLA) — see [LICENSING.md](LICENSING.md#contributor-license-agreement-cla).

---

**RavenClaw** — Secure, efficient, fast, lightweight. 🐦‍⬛
