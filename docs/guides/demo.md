# RavenClaws Demo Suite

This guide explains how to record and share terminal demos of RavenClaws features using asciinema.

## Prerequisites

- **asciinema** — install via pip: `pip3 install asciinema`
- **RavenClaws binary** — build with `cargo build --release`
- **LiteLLM** (optional) — running on `localhost:4000` for the exec mode demo

## Demo Suite

RavenClaws v1.2.0 ships with **5 focused demos**, each showcasing a specific set of features for different audiences:

| # | Script | Focus | Audience | Duration |
|---|---|---|---|---|
| 1 | `demo-quickstart.sh` | Version, binary profile, config, exec mode | Users | ~30s |
| 2 | `demo-architecture.sh` | Source modules, test suite, patterns, healing, load | Developers | ~45s |
| 3 | `demo-server-mcp.sh` | HTTP server, MCP server, MCP SSE server | Operators | ~40s |
| 4 | `demo-resilience.sh` | Self-healing engine, circuit breakers, graceful degradation | SREs | ~35s |
| 5 | `demo-deployment.sh` | Docker, K8s, Helm chart, website, verification suite | DevOps | ~40s |

## Quick Start

```bash
# Record a single demo
asciinema rec --title "RavenClaws v1.2.0 — Quickstart" \
  --command "./scripts/demos/demo-quickstart.sh" \
  --overwrite demos/demo-quickstart.cast

# Upload to asciinema.org
asciinema upload demos/demo-quickstart.cast
```

## Record All Demos

```bash
for demo in demo-quickstart demo-architecture demo-server-mcp \
            demo-resilience demo-deployment; do
  asciinema rec --title "RavenClaws v1.2.0 — ${demo#demo-}" \
    --command "./scripts/demos/$demo.sh" \
    --overwrite "demos/$demo.cast"
done

# Upload all
for f in demos/*.cast; do
  asciinema upload "$f"
done
```

## Running Without Recording

```bash
./scripts/demos/demo-quickstart.sh
./scripts/demos/demo-architecture.sh
./scripts/demos/demo-server-mcp.sh
./scripts/demos/demo-resilience.sh
./scripts/demos/demo-deployment.sh
```

## Demo Details

### 1. Quickstart (`demo-quickstart.sh`)
Shows the basics: version info, help output, binary profile (size, architecture, linked libraries), TOML configuration files, and one-shot `--exec` mode with an LLM response.

### 2. Architecture (`demo-architecture.sh`)
Dives into the codebase: 22 source modules, public API surface, 552+ unit tests passing, multi-agent patterns (debate, review-loop, research-synthesize, voting), self-healing engine with circuit breakers, and graceful degradation with token bucket rate limiting.

### 3. Server & MCP (`demo-server-mcp.sh`)
Demonstrates all server modes: HTTP server with `/health`, `/ready`, `/metrics` endpoints, MCP server over stdio for tool exposure, MCP SSE server over HTTP with SSE transport, and CLI flag overview for all server-related options.

### 4. Resilience (`demo-resilience.sh`)
Deep dive into the self-healing system: `SelfHealingEngine` architecture, `HealingCircuitState` (Closed/Open/HalfOpen), `FailureRecord` tracking, exponential backoff with jitter, `LoadManager` and `TokenBucket` for graceful degradation, and unit tests for circuit breakers.

### 5. Deployment (`demo-deployment.sh`)
Covers production deployment: multi-stage Dockerfile with distroless security, Kubernetes manifests with NetworkPolicy, Helm chart configuration, Cloudflare-deployed website with security headers, verification suite with 15 test modules, and git hooks for CI.

## Linking asciinema to an Account

To preserve recordings beyond 7 days, link the CLI to your asciinema.org account:

```bash
asciinema auth
# Follow the link printed to authenticate
```

## Latest Demos

The latest demo recordings are available at:

| Demo | Link |
|---|---|
| **Quickstart** | https://asciinema.org/a/GTnaQAo2EBfohVRJ |
| **Architecture** | https://asciinema.org/a/S0AuroDlPXrQiysy |
| **Server & MCP** | https://asciinema.org/a/WT48ceUWYHMI9pTZ |
| **Resilience** | https://asciinema.org/a/B3CYpEPzfCt1Aadb |
| **Deployment** | https://asciinema.org/a/UWMnW9N4as1kFayD |

> **Note:** Unauthenticated uploads are automatically deleted after 7 days.
