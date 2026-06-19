# 🐦‍⬛ RavenClaw — The Ultimate AI Agentic Worker

**Secure · Small · Efficient · Robust · Simple. Built in Rust.**

[![License](https://img.shields.io/badge/license-AGPLv3%20%2B%20Commercial-blue.svg)](LICENSING.md)
[![CI](https://github.com/egkristi/RavenClaw/actions/workflows/build.yml/badge.svg)](.github/workflows/build.yml)
[![Verification](https://img.shields.io/badge/verification-94%20checks-brightgreen)](VERIFICATION.md)
[![Binary](https://img.shields.io/badge/binary-~3.4MB-blue)]()
[![Status](https://img.shields.io/badge/status-v0.7.1-brightgreen)](ROADMAP.md)

RavenClaw is a lightweight, secure Rust agent framework with multi-provider LLM
support. One static binary, zero runtime dependencies — no Python, no Node, no JVM.

> **Status: v0.7.1 (2026-06-20).** The provider layer (5 providers), one-shot execution (`--exec`),
> reproducible multi-arch builds, verification + supply-chain pipeline, agent loop, tool-use, MCP client,
> retry/fallback chains, token budgets, native Anthropic integration, **swarm mode**, **supervisor mode**,
> **RavenFabric mesh client**, **MCP server**, and **HTTP server mode** all work today.
> This README marks ✅ built vs. 📋 planned — honestly. Trust is a feature; we don't inflate it.

| Footprint | Security | Providers | Deployment |
|---|---|---|---|
| **~3.4 MB binary** | **Memory-safe Rust** | **5 providers** | **Binary · Docker · K8s** |
| **0 runtime deps** | **Signed images + SBOM** | **Multi-model** | **307 unit tests + 94 verification checks** |

---

## Vision

RavenClaw aims to be the **ultimate AI agentic assistant and worker** — and the
**preferred alternative** to the field: Nemoclaw, Hermes Agent, TrustClaw, ZeroClaw,
PicoClaw, NanoClaw, Claude Cowork, Manus, Perplexity Computer, Kimi Claw, and Vellum.

We don't aim to win by out-featuring them. We win by refusing to compromise on five
pillars at once:

- **Secure** — memory-safe Rust (`unsafe` forbidden), fail-closed, no creds in config, verified supply chain.
- **Small** — one static binary (~3 MB), distroless image, lean dependency tree.
- **Efficient** — native performance, low memory, fast cold start, streaming everywhere.
- **Robust** — graceful degradation, provider fallback, deterministic config, verified across 4 deployment targets.
- **Simple** — one command to run, sensible defaults, no external services required for single-agent use.

See the **[ROADMAP](ROADMAP.md)** for how we get from here to there.

---

## Why RavenClaw?

### Small & efficient

- **~3.4 MB** stripped release binary (measured) — no interpreter, no runtime image baggage.
- **Single static binary** — no Python, no Node, no JVM, zero runtime dependencies.
- Native Rust with `lto` + `panic=abort`. Design targets (benchmarked toward v1.0 via the [verification suite](VERIFICATION.md)): **< 50 ms** cold start, **< 20 MB** RSS, **< 15 MB** binary across all targets.

### Secure & trustworthy

- **Memory-safe Rust** — whole classes of memory-corruption bugs eliminated at compile time.
- **No credentials in config** — environment variables and Kubernetes Secrets only.
- **Hardened containers** — distroless, non-root, read-only root filesystem, dropped capabilities, seccomp.
- **Verified supply chain** — multi-arch images signed with **Cosign**, **SBOM** (Syft) and build **provenance** attestation, plus **CodeQL**, **cargo-audit**, **cargo-deny**, **Trivy**, **Hadolint**, **Kubescape**, and **OSSF Scorecard** in CI.
- **TLS enforced** by default for non-local endpoints.
- **Deny-by-default tool policy** — `PolicyEngine` validates all tool calls against shell/path/network allow-lists.
- **Sandboxed tool execution** — workdir jail, resource limits, and timeouts via `Sandbox`.
- **Tamper-evident audit log** — HMAC-SHA256 chained, structured JSON trail of every tool call.

### Multi-provider, multi-model

- **LiteLLM** — OpenAI-compatible proxy fronting 100+ models.
- **OpenAI** — native GPT-4o, o-series, and more.
- **OpenRouter** — unified API for many hosted models.
- **Ollama** — local, private, air-gapped models.
- **Anthropic** — direct Claude API (Sonnet, Opus, Haiku) with native tool use.
- **Multi-model mode** — round-robin + intelligent fallback chains with circuit breaker (v0.5.1+).

### Verified across every target

- **307 Rust unit tests** (incl. `mockito`-backed provider request/response/error paths for all 5 providers, plus 12 RavenFabric client tests), runnable anywhere via `cargo test`.
- Plus a **94-check verification suite** (`scripts/verify.sh`) spanning **9 modules** across **4 deployment targets** — local binary, Docker, cross-compiled Linux, and Kubernetes — including security and performance checks.
- *Note:* the 94 verification checks are **system/integration level** (shell-orchestrated, requiring live services such as LiteLLM/Docker/kubectl).

---

## Quick Start

### 30 seconds to your first agent

```bash
# Build from source (requires Rust)
git clone https://github.com/egkristi/RavenClaw
cd RavenClaw
cargo build --release

# Run with any provider
export LITELLM_API_KEY="your-key"
export RAVENCLAW__LLM__ENDPOINT="http://localhost:4000"
./target/release/ravenclaw --mode single

# …or run a one-shot task and exit
./target/release/ravenclaw --exec "Summarize the latest release notes"
```

> **Note:** Pre-built binaries publish automatically on tagged releases. See the [GitHub Releases](https://github.com/egkristi/RavenClaw/releases) page for downloads.

### Docker

```bash
docker run --rm -it \
  -e LITELLM_API_KEY="your-key" \
  -e RAVENCLAW__LLM__ENDPOINT="http://litellm:4000" \
  ghcr.io/egkristi/ravenclaw:latest
```

### Docker Compose (with LiteLLM)

```bash
docker compose up -d
docker compose logs -f ravenclaw
```

### Kubernetes

```bash
kubectl apply -f k8s/deployment.yaml
kubectl -n ravenclaw get pods
kubectl -n ravenclaw logs -l app.kubernetes.io/name=ravenclaw
```

> Single mode currently performs one request and exits. A long-running **server
> mode** with `/health` `/ready` `/metrics` is on the roadmap (v0.7); until then,
> prefer the `deployment-test.yaml`/Job-style manifest for k8s smoke tests.

## Configuration

### Environment variables

| Variable | Description | Default |
|---|---|---|
| `RAVENCLAW__LLM__PROVIDER` | Provider: litellm, openrouter, ollama, openai | `litellm` |
| `RAVENCLAW__LLM__ENDPOINT` | LLM endpoint URL | (provider-dependent) |
| `RAVENCLAW__LLM__MODEL` | Default model | `gpt-4o-mini` |
| `LITELLM_API_KEY` | API key for LiteLLM/OpenRouter/OpenAI | (required for cloud) |
| `RAVENCLAW__LLMS` | JSON array for multi-model config | — |
| `RAVENCLAW__RAVENFABRIC__ENDPOINT` | RavenFabric endpoint (optional) | — |
| `RAVENCLAW__SECURITY__REQUIRE_TLS` | Enforce TLS | `true` |
| `RAVENCLAW__RUNTIME__MAX_AGENTS` | Max concurrent agents | `10` |
| `RUST_LOG` | Log level | `info` |

### Single provider mode

**LiteLLM:**
```toml
[llm]
provider = "litellm"
endpoint = "http://litellm:4000"
model = "gpt-4o-mini"
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
model = "anthropic/claude-sonnet-4-20250514"
```

**Anthropic (Native, v0.5.3+):**
```toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
# No endpoint needed — uses https://api.anthropic.com
```

**OpenAI:**
```toml
[llm]
provider = "openai"
model = "gpt-4o"
```

### Multi-model mode

Configure several providers at once (basic round-robin today):

```toml
[[llms]]
provider = "ollama"
endpoint = "http://ollama:11434"
model = "llama3.1"

[[llms]]
provider = "openrouter"
model = "anthropic/claude-sonnet-4-20250514"

[[llms]]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[llms]]
provider = "openai"
model = "gpt-4o"
```

### Full config example

```toml
[llm]
provider = "litellm"
endpoint = "http://litellm:4000"
model = "gpt-4o-mini"
timeout_secs = 30

# RavenFabric for remote execution and mesh coordination (v0.6.1)
[ravenfabric]
endpoint = "http://ravenfabric:8080"
remote_exec = true
allowed_hosts = ["litellm", "ravenfabric"]

[security]
require_tls = true
token_lifetime_secs = 3600   # surface present; enforcement on roadmap (v0.4)
audit_log = true             # surface present; enforcement on roadmap (v0.4)

[runtime]
workdir = "/workspace"
max_agents = 10
health_interval_secs = 60
```

## Agent modes

| Mode | Status | Description |
|---|---|---|
| `single` | ✅ **Working** | Sends prompt to LLM, logs response (agent loop with ReAct planning) |
| `single` (multi-model) | ✅ **Working** | Iterates all configured providers, logs each response |
| `--exec "<task>"` | ✅ **Working** | One-shot task execution with streaming, then exit |
| `--repl` | ✅ **Working** | Interactive REPL with `/exit`, `/reset` commands |
| `swarm` | ✅ **Working** | 3 parallel agents with different personas (single + multi-model); RavenFabric-aware |
| `supervisor` | ✅ **Working** | Task decomposition + sub-agent spawning + result aggregation (single + multi-model); RavenFabric-aware |

## Building from source

### Prerequisites

- [Rust](https://rustup.rs/) 1.86+
- For Linux cross-compilation on macOS: `brew install FiloSottile/musl-cross/musl-cross`

### Build for host

```bash
git clone https://github.com/egkristi/RavenClaw
cd RavenClaw

cargo build --release      # release build for current platform
cargo test                 # unit tests
./scripts/verify.sh        # full 94-check verification suite (needs LiteLLM/Docker/kubectl)
docker build -t ravenclaw:latest .
```

### Cross-compile for all architectures

```bash
rustup target add \
    x86_64-apple-darwin aarch64-apple-darwin \
    x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
    x86_64-unknown-linux-musl

./scripts/build.sh --all                                   # all targets
./scripts/build.sh --target aarch64-unknown-linux-gnu      # one target
```

### Git Hooks (Pre-Commit / Pre-Push)

The project includes git hooks that run automated verification before every commit and push:

```bash
# Install hooks (one-time setup after cloning)
.githooks/setup.sh

# Verify hooks are active
.githooks/setup.sh --check

# Remove hooks (restore defaults)
.githooks/setup.sh --remove
```

**Pre-commit** (fast — runs in seconds):
1. `cargo fmt --check` — formatting
2. `cargo clippy -D warnings` — linting
3. `cargo test --locked` — unit tests
4. Binary size check — warns if over 5MB
5. Secrets scan — no hardcoded API keys/tokens

**Pre-push** (comprehensive — runs in 1-5 minutes):
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

### Multi-arch Docker image

```bash
docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t ghcr.io/egkristi/ravenclaw:latest \
    --push .
```

## Downloads

> **Note:** Pre-built binaries publish automatically on tagged releases. See the
> [GitHub Releases](https://github.com/egkristi/RavenClaw/releases) page for downloads.

| Architecture | Target Triple |
|---|---|
| Apple Silicon (M1+) | `aarch64-apple-darwin` |
| Intel Mac | `x86_64-apple-darwin` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux x86_64 (musl/static) | `x86_64-unknown-linux-musl` |

Container images target both `linux/amd64` and `linux/arm64`.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    RavenClaw Agent                        │
│  ┌──────────────────────────────────────────────────┐    │
│  │         Agent Modes (✅ All Working)               │    │
│  │  Single · --exec · REPL · Swarm · Supervisor      │    │
│  │  (single-provider + multi-model for all modes)    │    │
│  └──────────────────────┬───────────────────────────┘    │
│                         │                                 │
│  ┌──────────────────────┴───────────────────────────┐    │
│  │         Agent Core                                │    │
│  │  perceive → plan → act → observe (ReAct loop)    │    │
│  │  ConversationMemory · max-iteration guard         │    │
│  └──────────────────────┬───────────────────────────┘    │
│                         │                                 │
│  ┌──────────────────────┴───────────────────────────┐    │
│  │         LLM Provider Abstraction Layer            │    │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐   │    │
│  │  │LiteLLM │ │ OpenAI │ │OpenRtr │ │ Ollama │   │    │
│  │  └────────┘ └────────┘ └────────┘ └────────┘   │    │
│  │  ┌──────────┐ ┌──────────────────────┐          │    │
│  │  │Anthropic │ │ MultiModelManager    │          │    │
│  │  └──────────┘ │ round-robin · fallback│         │    │
│  │               │ circuit breaker       │         │    │
│  │               └──────────────────────┘          │    │
│  └──────────────────────┬───────────────────────────┘    │
│                         │                                 │
│  ┌──────────────────────┴───────────────────────────┐    │
│  │         Security & Infrastructure Layer           │    │
│  │  ┌──────────┐ ┌──────────┐ ┌────────┐ ┌──────┐  │    │
│  │  │Policy    │ │ Sandbox  │ │Audit   │ │ MCP  ││Raven  │  │    │
│  │  │Engine    │ │ (jail)   │ │Log     │ │Client││Fabric │  │    │
│  │  └──────────┘ └──────────┘ └────────┘ └──────┘└───────┘  │    │
│  │  TLS · env-only secrets · non-root · RBAC                 │    │
│  └───────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────┐
│   Deployment Targets  │
│  Binary · Docker · K8s │
└──────────────────────┘
```

### What's implemented vs. planned

| Component | Status | Details |
|---|---|---|
| Single agent (single + multi-model) | ✅ Working | Sends prompt(s), logs response(s) |
| LLM providers (5) | ✅ Working | LiteLLM, OpenAI, OpenRouter, Ollama, Anthropic (unified trait) |
| CLI & env-var overrides | ✅ Working | `--provider`, `--endpoint`, `--model`; layered TOML→env→flags |
| Config validation | ✅ Working | TLS enforcement, endpoint checks |
| Container & K8s security | ✅ Working | Distroless, non-root, read-only FS, dropped caps, seccomp, RBAC |
| CI/CD pipeline | ✅ Implemented | fmt + clippy + test, 5-target builds, multi-arch images, Cosign + SBOM + provenance + Trivy, crates.io publish, releases |
| Security scanning | ✅ Implemented | CodeQL, cargo-audit, cargo-deny, Trivy (FS + config), Hadolint, Kubescape, OSSF Scorecard |
| Verification suite | ✅ Working | 94 system/integration checks · 9 modules · 4 targets (`scripts/verify.sh`) |
| Rust unit tests | ✅ Working | 307 tests across 11 modules, incl. `mockito`-backed provider request/response/error paths + 12 RavenFabric client tests |
| Reproducible builds | ✅ Working | `Cargo.lock` committed (`--locked`), multi-arch Docker cross-linker, RavenFabric agent checksum-verified |
| `--exec` one-shot mode | ✅ Working | Run a single task, then exit |
| Interactive REPL | ✅ Working | `--repl` with `/exit`, `/reset` commands |
| Agent loop / ReAct planning | ✅ Working | perceive→plan→act→observe with max-iteration guard |
| Tool-use / function calling | ✅ Working | ToolImpl trait + ToolRegistry + 4 built-in tools + agent loop wiring |
| Streaming responses | ✅ Working | SSE streaming for LiteLLM, default fallback for others |
| Conversation memory | ✅ Working | `ConversationMemory` with configurable max history |
| System prompt / persona | ✅ Working | `LLMConfig.system_prompt`, CLI `--system-prompt`, env var |
| Swarm mode | ✅ Working | 3 parallel agents with different personas (single + multi-model) |
| Supervisor mode | ✅ Working | Task decomposition + sub-agent spawning + result aggregation (single + multi-model) |
| MCP client | ✅ Working | JSON-RPC over stdio, tool discovery and registration |
| MCP server | ✅ **v0.7.0** | Expose RavenClaw tools over stdio via MCP protocol; `--mcp-server` flag; policy-checked and audited |
| HTTP server mode | ✅ **v0.7.1** | Long-running server with `/health`, `/ready`, `/metrics`; `--serve` flag; graceful shutdown |
| Retry / fallback chains | ✅ Working | Exponential backoff, circuit breaker, token budgets |
| Deny-by-default policy | ✅ Working | PolicyEngine with shell/path/network allow-lists |
| Sandboxed execution | ✅ Working | Workdir jail, resource limits, timeouts |
| Tamper-evident audit log | ✅ Working | HMAC-SHA256 chained, structured JSON |
| Multi-model routing | ✅ Working | `next_client()` round-robin wired into agent modes |
| RavenFabric integration | ✅ **v0.6.1** | Full client module (`RavenFabricClient`) with health, list_agents, execute, broadcast; wired into all agent modes; 12 unit tests |

## How RavenClaw intends to win

RavenClaw is positioned against the field — Nemoclaw, Hermes Agent, TrustClaw,
ZeroClaw, PicoClaw, NanoClaw, Claude Cowork, Manus, Perplexity Computer, Kimi Claw,
and Vellum — by category:

- **vs. cloud / hosted assistants** (Claude Cowork, Manus, Perplexity Computer, Kimi Claw): RavenClaw is **self-hostable, offline-capable, and source-available** under AGPLv3. Your data and tool calls never leave infrastructure you control — and there is no phone-home.
- **vs. minimal agent runtimes** (ZeroClaw, PicoClaw, NanoClaw, TrustClaw): RavenClaw matches their footprint while adding a real **security model** (memory-safe core, verified supply chain, deny-by-default tool policy, sandboxing, tamper-evident audit log) plus **multi-provider** routing with fallback chains.
- **vs. SDK / platform plays** (Vellum, Hermes Agent, Nemoclaw): RavenClaw is a **single dependency-light binary**, not a service you rent or a framework you marry. Embed it, ship it, forget it.

| Our commitment | How we back it |
|---|---|
| Memory-safe core | Rust with `unsafe` forbidden |
| Tiny footprint | ~3.4 MB binary, distroless image, 0 runtime deps |
| Trustworthy releases | Cosign signing · SBOM · provenance · CodeQL · Trivy · OSSF Scorecard |
| Runs anywhere, privately | Self-hostable, air-gappable, no telemetry |
| Honest about status | ✅/📋 markers everywhere; benchmarks published, not asserted |

> Where we intend to lead — measurably, by v1.0: smallest footprint in class,
> sub-50 ms cold start, zero known CVEs, fully self-hostable, signed +
> SBOM-attested. These are targets we will benchmark and publish, not marketing.

## Roadmap

See **[ROADMAP.md](ROADMAP.md)** for the full phased plan and the
[feature gap analysis](ROADMAP.md#features-required-to-become-the-preferred-alternative)
versus the field.

**✅ v0.2 — build honest & green (complete):** `Cargo.lock` committed (reproducible
`--locked` builds), multi-arch Docker cross-linker fixed, RavenFabric agent
checksum-verified, `--exec` wired, swarm/supervisor fail loudly, version synced, and
a 100+-test `mockito`-backed Rust suite.

**✅ v0.3 — a real agent (complete):** the perceive→plan→act→observe loop, interactive REPL,
conversation memory, and streaming.

**✅ v0.4 — tools, safety & MCP (complete):** function-calling, built-in tools behind a
deny-by-default policy, sandboxing, a tamper-evident audit log, and **MCP client** —
the single highest-leverage step to tap the entire tool ecosystem.

**✅ v0.5 — providers and routing (complete):** unified OpenAI-compatible client, retry/fallback
chains with circuit breaker, token budgets, MCP client integration, native Anthropic provider.

**✅ v0.6 — swarm, supervisor (complete):** parallel swarm agents, task-decomposing supervisor,
both single-provider and multi-model variants.

**The five that matter most** toward being *preferred*: MCP (v0.5.2) · agent loop +
tools + sandbox (v0.3–v0.4) · local-first security model (v0.4) · async/background +
scheduling (v0.7) · RavenFabric distributed execution (v0.6.1).

## License

RavenClaw uses a **dual-license model**:

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

**RavenClaw** — Secure · Small · Efficient · Robust · Simple. Simply the best. 🐦‍⬛
