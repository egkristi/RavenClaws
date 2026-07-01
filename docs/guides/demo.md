# RavenClaws Demo

This guide explains how to record and share a terminal demo of RavenClaws features using asciinema.

## Prerequisites

- **asciinema** — install via pip: `pip3 install asciinema`
- **RavenClaws binary** — build with `cargo build --release`
- **LiteLLM** (optional) — running on `localhost:4000` for the exec mode demo

## Quick Start

```bash
# Record the demo
asciinema rec --title "RavenClaws v1.1.0 Demo" \
  --command "./scripts/demo.sh" \
  --overwrite demo.cast

# Upload to asciinema.org
asciinema upload demo.cast
```

## Demo Script

The automated demo script is at `scripts/demo.sh`. It showcases 11 sections:

| # | Section | What it shows |
|---|---|---|
| 1 | **Version & Help** | `--version` and `--help` output |
| 2 | **Binary Profile** | File size, architecture, linked libraries |
| 3 | **Configuration** | TOML config files (single + multi-model) |
| 4 | **Source Modules** | Module count, public API surface, line counts |
| 5 | **Test Suite** | `cargo test` results (507+ unit tests) |
| 6 | **One-Shot Exec** | `--exec` mode with LLM response |
| 7 | **HTTP Server** | `--serve` mode with health/ready/metrics endpoints |
| 8 | **MCP Server** | `--mcp-server` mode for tool exposure |
| 9 | **Docker & K8s** | Dockerfile and Kubernetes deployment manifests |
| 10 | **Website** | Landing page, styles, JS, security headers |
| 11 | **Verification Suite** | Test modules and orchestrator script |

## Running Without Recording

```bash
./scripts/demo.sh
```

This runs the same demo steps but outputs directly to the terminal without recording.

## Linking asciinema to an Account

To preserve recordings beyond 7 days, link the CLI to your asciinema.org account:

```bash
asciinema upload demo.cast
# Follow the link printed to authenticate
```

## Latest Demo

The latest demo recording is available at:

**https://asciinema.org/a/Qd8hCw6TVdrnNYLE** (v1.1.0)

> **Note:** Unauthenticated uploads are automatically deleted after 7 days.
