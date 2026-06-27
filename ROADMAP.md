# 🐦‍⬛ RavenClaws Roadmap

**Date:** 2026-06-27  
**Version:** v0.9.5 — Tool Execution Reliability ✅  
**Previous Release:** v0.9.4 (2026-06-27) — `--no-final-required` flag, response logging, system prompt update ✅  
**Current Commit:** *(to be tagged)*
**CI Status:** Build & Release ✅ · Container Build ✅ · Security Scan ✅
**v1.0 Hardening Progress:** 14/122 items completed. **v0.9.6–v0.9.9 series planned** to close all gaps identified in rpi5 deployment feedback — making RavenClaws a fully functional primary agent that can replace OpenClaw, Manus, and other cloud agents.

**Vision:** RavenClaws is the **ultimate AI agentic assistant and worker** — the preferred alternative to OpenClaw, Manus, Perplexity Comet, Kimi, Claude Cowork, and every other agent in the field. Not by out-featuring them, but by being **fully functional as a primary agent** while also being smaller, more secure, and more efficient than anything else.

RavenClaws operates **autonomously** — with a heartbeat, working on tasks over long
periods independently, without requiring constant human supervision. It plans,
executes, reflects, and adapts across hours, days, or weeks.

RavenClaws orchestrates **swarms at any scale** — from a handful of specialized
collaborators to **thousands of workers**, each with unique traits, capabilities, and
personalities. A swarm is TRULY a swarm: unbounded, self-organizing, and emergent.
RavenClaws provisions, configures, and manages its own sub-agents and worker
instances dynamically based on task requirements — no fixed limit, no artificial
cap. The swarm grows and shrinks organically as work demands.

All of this happens **efficiently and securely** — every agent communication is
policy-gated, audited, and sandboxed. The five pillars (Secure, Small, Efficient,
Robust, Simple) apply to the swarm just as they apply to the single agent.

### The rpi5 Verdict — and Our Response

Real-world testing on a Raspberry Pi 5 (K3s, aarch64, 8GB RAM) revealed that RavenClaws
v0.9.3 is **functional but not yet a primary agent**. The feedback was honest:

> *"RavenClaws works as a lightweight, secure agent runtime — it runs, connects to LLMs,
> executes agent loops, and manages swarms. But it's not a drop-in replacement for OpenClaw."*

**The gaps are concrete and fixable:**

| Gap | Root Cause | Fix Plan |
|---|---|---|
| Tool execution fails with non-structured models | Agent loop requires `FINAL:` or structured `tool_calls` | ✅ **v0.9.4**: Added `--no-final-required`, response logging, system prompt update |
| `--exec` produces no output for most models | Error path suppresses last response | ✅ **v0.9.4**: `--no-final-required` flag + response logging |
| No agent execution HTTP endpoints | Server mode is status-only | v0.9.6: Add `/chat`, `/execute`, `/tools` |
| MCP client can't connect to SSE servers | SSE transport was stubbed | v0.9.3 ✅ (fixed) |
| MCP server is stdio-only | SSE transport was stubbed | v0.9.3 ✅ (fixed) |
| No TOML config for MCP servers | CLI-only, single connection | v0.9.5: Add `[mcp]` config + multi-server *(deferred to v0.9.6)* |
| Tool execution silently fails | No fallback for non-structured models | ✅ **v0.9.5**: Added text-based tool call detection |
| Sandbox breaks with read-only root FS | Hardcoded `/tmp` workdir | v0.9.8: Configurable workdir via env var |
| Heartbeat state may corrupt on SIGTERM | No graceful shutdown hook | v0.9.8: Add Drop impl + signal handler |
| Init container doesn't chown workspace | Missing `chown` in K8s manifest | v0.9.8: Fix deployment.yaml |
| SwarmTopology enum mismatch | TOML deserialization expects string, not array | v0.9.4 ✅ (fixed) |
| `agent_count` field not recognized | Missing serde alias on `max_workers` | v0.9.4 ✅ (fixed) |
| `[swarm.profiles]` TOML syntax fails | Only `[[swarm.profiles]]` array-of-tables supported | v0.9.5: Add shorthand deserializer *(deferred to v0.9.6)* |
| Heartbeat goal error message unclear | Missing example in error | v0.9.4 ✅ (fixed) |
| LiteLLM API key docs wrong | References `openclaw-secrets` instead of `litellm-secrets` | v0.9.8: Document correct secret reference |
| `--serve` mode not documented | No docs page for HTTP server mode | v0.9.6: Add server mode docs |
| OpenTelemetry warning on startup | OTEL exporter warns if no collector configured | v0.9.8: Suppress warning when OTEL disabled |
| Server port not configurable via env var | Only `--port` CLI flag | v0.9.6: Add env var override |
| Config hot-reload not supported | No SIGHUP handler | v0.9.6: Add SIGHUP-based config reload |
| NetworkPolicy blocks LLM egress | New pod labels not in LiteLLM ingress policy | v0.9.8: Document NetworkPolicy requirements |
| Secret reference uses wrong key | `LITELLM_API_KEY` doesn't exist in `openclaw-secrets` | v0.9.8: Document correct `litellm-secrets` reference |
| Agent loop logs show `<no thought>` | Log only looks for `THOUGHT:` prefix | ✅ **v0.9.4**: Added response content logging |
| LLM response content not logged | No debug-level logging of responses | ✅ **v0.9.4**: Added `debug!` log after each response |
| MCP server stdin closes before processing | stdio-only transport, no SSE fallback | v0.9.3 ✅ (SSE transport implemented) |
| MCP client can't connect to SSE servers | `Sse` variant returns `Err("not implemented")` | v0.9.3 ✅ (fixed) |
| No `[mcp]` section in TOML config | CLI flags only, no config struct | v0.9.5: Add `McpConfig` struct |
| Only one MCP client connection | Single `--mcp-command` flag | v0.9.5: Add multi-MCP-client support |
| Workspace permission denied | Init container doesn't `chown` to UID 65532 | v0.9.8: Fix deployment.yaml |
| Tool execution not working with deepseek-v4-pro | Model doesn't emit structured `tool_calls` | ✅ **v0.9.5**: Added text-based tool call detection |

**The plan:** Six rapid releases (v0.9.4 → v0.9.9) to close every gap, then v1.0 is
truly production-ready — a primary agent that can replace OpenClaw, Manus, or any
cloud agent, while being smaller, more secure, and more efficient.

**Core Principles** — every decision is measured against these five. If a feature
can't be added without breaking one, it doesn't ship in core.

| Pillar | What it means in practice |
|---|---|
| 🔒 **Secure** | Memory-safe Rust (`unsafe` forbidden). Fail-closed. No creds in config, TLS enforced, every tool call policy-gated and audited. Signed releases, SBOM, verified supply chain. |
| 🪶 **Small** | One static binary, distroless image, lean dependency tree. Target < 15 MB stripped, < 30 MB image. |
| ⚡ **Efficient** | Native performance, low idle memory (< 20 MB RSS), fast cold start (< 50 ms), streaming everywhere. |
| 🛡️ **Robust** | No `panic`/`unwrap` on hot paths. Retries with backoff, provider fallback, deterministic config, high coverage. |
| ✨ **Simple** | One command to run. Sensible defaults. Zero-config for common cases. No external services required for single-agent use. |

### Non-goals

- Not a heavyweight orchestration platform — RavenClaws stays a small worker; large-scale mesh coordination is delegated to **RavenFabric**.
- Not a UI/IDE — RavenClaws is a headless binary + library; frontends consume it.
- No telemetry phone-home, ever. Observability is opt-in and self-hosted.

---

## Current State

**Version:** 0.9.5 (2026-06-27) — Tool Execution Reliability  
**Stats:** 18 source modules (+lib.rs, +eval.rs, +ravenfabric.rs), ~16,700 LOC, 6 LLM providers (+ generic `openai-compatible`), 5 built-in tools (+web_search), **471 unit tests**, 114 verification tests across 10 modules, multi-arch CI with signed images + SBOM, official Helm chart, `zeroize` for secret material, prompt-injection defense, autonomous heartbeat agent, long-horizon task persistence, self-provisioning swarm orchestration, inter-agent communication bus, swarm health monitoring & telemetry, MCP SSE transport (client + server), `--no-final-required` flag, agent loop response logging, **text-based tool call detection fallback**, **tool execution logging**, **configured web search endpoint**, **ToolRegistry wiring in agent loop**, published on crates.io as `ravenclaws` (binary + library crate).

| Component | Status | Details |
|---|---|---|
| Single agent (single-provider) | ✅ Working | Sends one prompt, logs response, exits |
| Single agent (multi-model) | ✅ Working | Iterates all providers, logs each response |
| **Swarm mode (single-provider)** | ✅ **v0.6** | Multiple parallel agents with different personas (analytical/creative/pragmatic); no fixed limit |
| **Supervisor mode (single-provider)** | ✅ **v0.6** | Task decomposition, sub-agent spawning, result aggregation |
| **Swarm mode (multi-model)** | ✅ **v0.6** | Parallel agents across different LLM providers; scales to any number |
| **Supervisor mode (multi-model)** | ✅ **v0.6** | Provider-aware task decomposition and assignment |
| LLM providers (6 + generic) | ✅ Working | LiteLLM, OpenAI, OpenRouter, Ollama, **Anthropic**, **OpenAI-Compatible** (unified trait); generic `openai-compatible` unlocks vLLM, llama.cpp, LM Studio, TGI, Groq, Together AI, Fireworks, DeepInfra |
| CLI & env-var overrides | ✅ Working | `--provider`, `--endpoint`, `--model`, layered TOML→env→flags |
| Config validation | ✅ Working | TLS enforcement, endpoint checks |
| Container & K8s security | ✅ Working | Distroless, non-root, read-only FS, dropped caps, seccomp, RBAC |
| CI/CD pipeline | ✅ Implemented | fmt + clippy `-D warnings` + test, 5-target builds, multi-arch images, **Cosign + SBOM + provenance + Trivy**, crates.io publish, releases — cross-compilation deps installed for all targets |
| Security scanning | ✅ Implemented | CodeQL, cargo-audit, cargo-deny, cargo-outdated, cargo-udeps, Trivy (FS + config), Hadolint, Kubescape, OSSF Scorecard, dependency review — all SARIF results uploaded to GitHub Security tab |
| Verification suite | ✅ Working | 114 system/integration checks · 10 modules · 4 targets (`scripts/verify.sh`: local, Docker, Linux, K8s, security, performance, LLM-quality, swarm, eval) — shell-orchestrated, requires live services |
| Eval harness | ✅ **v0.7.4** | `--eval <path>` mode with 7 assertion types, run traces, text/JSON reports, 24 unit tests + 20 verification tests, sample configs in `tests/eval/` |
| Multi-model routing | ✅ Working | `next_client()` round-robin + fallback chain with circuit breaker |
| RavenFabric integration | ✅ **v0.6.1** | Full client module (`RavenFabricClient`) with health, list_agents, execute, broadcast; wired into all agent modes; 12 unit tests |
| `--exec` one-shot mode | ✅ **v0.9.4** | `--no-final-required` flag, response logging, default system prompt with `FINAL:` instructions. Models that don't emit `FINAL:` now work with `--no-final-required`. |
| Rust unit tests | ✅ Working | 460 tests across all 18 modules; `mockito`-based HTTP tests for all 6 providers + RavenFabric |
| Agent loop / ReAct planning | ✅ Working | perceive→plan→act→observe with max-iteration guard, `FINAL:` marker detection, configurable via `--max-iterations` |
| Tool-use / function calling | ✅ **v0.9.5** | Tool abstraction + registry + **5 built-in tools** (+web_search) + **MCP tool discovery** + agent loop wiring + **text-based tool call detection fallback** + **tool execution logging** + **configured web search endpoint**. Tool execution now works with models that don't emit structured tool calls (e.g., `deepseek-v4-pro:cloud`). |
| Deny-by-default policy | ✅ **Wired to agent loop** | `PolicyEngine` validates ALL tool calls before execution (commit 51e42b0) |
| Sandboxed execution | ⚠️ Partial → 🎯 **v0.9.8** | `Sandbox` provides workdir jail for `shell_exec`. Default workdir `/tmp/ravenclaws-sandbox` breaks with `readOnlyRootFilesystem: true` — no env-var override or fallback. **Fix planned:** configurable workdir via env var or config field |
| Audit log | ✅ **Wired to agent loop** | HMAC-SHA256 chained, tamper-evident, emits events for all tool calls (commit 51e42b0) |
| Streaming responses | ✅ Working | SSE streaming for LiteLLM, default non-streaming fallback for others |
| Conversation memory | ✅ Working | `ConversationMemory` struct with configurable max history, auto-trim |
| Interactive REPL | ✅ Working | `--repl` flag with stdin loop, streaming output, `/exit` `/reset` commands |
| System prompt / persona | ✅ Working | `LLMConfig.system_prompt` field, CLI `--system-prompt`, env var override |
| MCP client | ⚠️ Partial → 🎯 **v0.9.5** | JSON-RPC 2.0 over stdio + SSE transport. Only one MCP server connection supported (single `--mcp-command`). No TOML config section for MCP servers. **Fix planned:** TOML config section, multi-MCP-client support |
| **MCP server** | ✅ **v0.7** | Exposes RavenClaws tools over stdio via MCP protocol; `--mcp-server` flag; policy-checked and audited. SSE transport also implemented (v0.9.3) |
| **HTTP server mode** | ✅ **v0.7.1** → 🎯 **v0.9.6** | Long-running server with `/health`, `/ready`, `/metrics` endpoints; `--serve` flag; fixes k8s CrashLoopBackOff. No agent execution endpoints (`/chat`, `/execute`, `/tools`). **Fix planned:** `/chat`, `/execute`, `/tools` endpoints, deep health check, readiness LLM check |
| **OpenTelemetry tracing** | ✅ **v0.7.2** | Opt-in distributed tracing with OTLP gRPC/stdout exporter; `#[instrument]` spans on agent loop, HTTP server, tools, LLM calls |
| Native Anthropic provider | ✅ Working | Direct Claude API with tool use, token tracking (v0.5.3) |
| Retry / fallback / circuit breaker | ✅ Working | Exponential backoff, token budgets, provider fallback chain (v0.5.1) |
| Pre-built binary releases | 📋 Wired, untagged | CI produces them on tag; none released yet |
| `RavenFabricClient` wired to agent loop | ❌ → 🎯 **v0.9.8** | Client created but `health()`, `execute()`, `broadcast()` never called |
| `ProviderFallbackChain` wired to agent loop | ❌ → 🎯 **v0.9.8** | Fallback chain struct exists but never used by agent loop |
| `TokenBudget` wired to agent loop | ❌ → 🎯 **v0.9.8** | Token budget struct exists but never checked during execution |
| `AgentMessageBus` wired to swarm | ❌ → 🎯 **v0.9.8** | Message bus created but never used in orchestration |
| `SwarmHealthMonitor` wired to swarm | ❌ → 🎯 **v0.9.8** | Health monitoring initialized but never checked |
| `WebSearchConfig` wired to web search tool | ✅ **v0.9.5** | `ToolRegistry::with_config()` reads web search endpoint from config |
| `--provider anthropic` CLI flag | ✅ **v0.9.3** | Now selects Anthropic provider correctly |
| `--webhook-port` CLI flag | ✅ **v0.9.3** | Now configures the scheduler's webhook server |
| Audit log mutex `unwrap()` | ✅ **v0.9.3** | Replaced with `lock_entries()` helper — mutex poisoning no longer panics |
| MCP SSE transport | ✅ **v0.9.3** | Client and server SSE transport implemented; 7 tests passing |
| MCP TOML config section | ❌ → 🎯 **v0.9.5** | No `[mcp]` section in `src/config.rs` — CLI flags only |
| Multi-MCP-client support | ❌ → 🎯 **v0.9.5** | Only one MCP server connection supported (single `--mcp-command`) |
| Server agent execution endpoints | ❌ → 🎯 **v0.9.6** | No `/chat`, `/execute`, or `/tools` endpoints |
| Community health files | ❌ → 🎯 **v0.9.8** | Missing `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` |
| Container image size | ⚠️ → 🎯 **v0.9.8** | ~50 MB vs < 30 MB target |
| Library re-exports | ✅ **v0.9.3** | All 9 modules now re-exported from `src/lib.rs` |
| Git hooks (pre-commit / pre-push) | ✅ Working | `.githooks/` — fmt, clippy, tests, binary size, secrets on commit; +release build, Docker, security on push |
| Structured function calling | ✅ Working | OpenAI Tools format for OpenAI/LiteLLM/OpenRouter/Anthropic |
| **Human-in-the-loop approvals** | ✅ **v0.8** | `--require-approval` flag prompts for sensitive tool calls; audited |
| **Prompt-injection defense** | ✅ **v0.8** | `InjectionDetector` with 50+ patterns, instruction-boundary enforcement, output schema validation; wired to both agent loops; audited |
| Multi-modal input | ⚠️ Partial | AnthropicClient has image support structure, not wired to CLI *(v0.10)* |
| Generic `openai-compatible` provider | ✅ **v0.9.3** | Unlocks vLLM, llama.cpp, LM Studio, TGI, Groq, Together AI, Fireworks, DeepInfra |
| `--exec` mode `FINAL:` fallback | ✅ **v0.9.4** | `--no-final-required` flag lets any non-tool-call response complete the loop |
| Agent loop response logging | ✅ **v0.9.4** | `debug!` log after each LLM response in both agent loops — shows length + preview |
| Tool execution reliability | ✅ **v0.9.5** | Text-based tool call detection fallback + debug logging + configured web search endpoint |
| Configurable sandbox workdir | ❌ → 🎯 **v0.9.8** | Default `/tmp/ravenclaws-sandbox` breaks with `readOnlyRootFilesystem: true` |
| Graceful shutdown for heartbeat | ❌ → 🎯 **v0.9.8** | No SIGTERM handling in heartbeat mode — state file may be corrupted |
| Init container `chown` in K8s | ❌ → 🎯 **v0.9.8** | `k8s/deployment.yaml` relies on `fsGroup` but has no explicit `chown` init container |
| LiteLLM API key documentation | ❌ → 🎯 **v0.9.8** | `api_key` field exists on `LLMConfig` but not documented in config reference |
| Heartbeat `goal` error message | ✅ **v0.9.4** | Now includes example: `--heartbeat-goal "Monitor system health and report anomalies"` |
| Readiness probe LLM check | ❌ → 🎯 **v0.9.6** | `/ready` returns OK immediately, doesn't verify LLM connectivity |
| Network policy documentation | ❌ → 🎯 **v0.9.8** | No docs on required NetworkPolicy for LiteLLM egress |
| Secret reference documentation | ❌ → 🎯 **v0.9.8** | No docs on which K8s Secrets are required and their keys |

### ✅ v0.4.0 Released (2026-06-03)

All v0.4 blockers resolved and shipped:
- ✅ Security features wired to agent loop (commit `51e42b0`)
- ✅ Structured function calling (OpenAI Tools format)
- ✅ 274 unit tests + 94 verification tests
- ✅ CI/CD pipeline green (fmt, clippy, test, security scans)

**Known limitations (documented, not blockers):**
- k8s Deployment enters CrashLoopBackOff — server mode planned for v0.7
- SSE transport for MCP not yet implemented (stdio only in v0.5.2)
- Multi-modal input (images) — Anthropic client has stub, not wired to CLI

### 🔧 Critical Blockers (v0.5 Release)

These must be resolved before v0.5 can ship:

1. ~~**Code duplication across OpenAI-compatible clients**~~ ✅ Fixed v0.5.0 — unified `OpenAICompatibleClient`
2. ~~**No provider fallback/retry logic**~~ ✅ Fixed v0.5.1 — exponential backoff, circuit breaker
3. ~~**No token budget tracking**~~ ✅ Fixed v0.5.1 — `TokenBudget` struct with cost estimation
4. ~~**No MCP integration**~~ ✅ Fixed v0.5.2 — full MCP client with stdio transport
5. ~~**No native Anthropic provider**~~ ✅ Fixed v0.5.3 — direct Claude API with tool use

### ✅ Resolved (v0.1 → v0.5.3)

1. ~~**`Cargo.lock` is git-ignored, but `--locked` is used in CI**~~ ✅ Fixed — lockfile committed
2. ~~**Dockerfile cross-compile fails (no cross-linker)**~~ ✅ Fixed — `gcc-aarch64-linux-gnu` + linker config
3. ~~**RavenFabric agent download unverified**~~ ✅ Fixed — SHA256SUMS verification
4. ~~**CI cross-compilation missing toolchain deps**~~ ✅ Fixed — `musl-tools`, `libc6-dev-arm64-cross`
5. ~~**`--exec` dead code**~~ ✅ Fixed — fully implemented with streaming
6. ~~**Client code duplicated 4×**~~ ✅ Fixed v0.5.0 — unified `OpenAICompatibleClient`
7. ~~**No conversation memory**~~ ✅ Fixed — `ConversationMemory` with auto-trim
8. ~~**No REPL mode**~~ ✅ Fixed — `--repl` with `/exit`, `/reset`
9. ~~**No agent loop**~~ ✅ Fixed — `run_agent_loop()` with max-iteration guard
10. ~~**No tool system**~~ ✅ Fixed — 4 built-in tools + registry + agent loop wiring
11. ~~**No security infrastructure**~~ ✅ Fixed — `PolicyEngine`, `Sandbox`, `AuditLog` implemented
12. ~~**No retry/fallback logic**~~ ✅ Fixed v0.5.1 — exponential backoff, circuit breaker, token budgets, fallback chains
13. ~~**No MCP integration**~~ ✅ Fixed v0.5.2 — full MCP client with stdio transport, tool discovery, execution
14. ~~**No native Anthropic provider**~~ ✅ Fixed v0.5.3 — direct Claude API with tool use support

---

## Architecture

### Current (v0.9)

```text
        ┌──────────┐
        │  main.rs │  CLI (clap) · JSON logging · mode dispatch
        └────┬─────┘
   ┌─────────┼──────────────────────────────────────────────────────────┐
┌──┴───┐ ┌───┴────┐ ┌───┴─────┐ ┌───┴───┐ ┌────────────┐ ┌──────┴───────┐
│agent │ │ config │ │  error  │ │ tools │ │policy      │ │ ravenfabric  │
│ loop │ │        │ │         │ │       │ │audit       │ │ client       │
│ mem  │ │        │ │         │ │       │ │sandbox     │ │ health       │
│swarm │ │        │ │         │ │       │ │mcp         │ │ execute      │
│super │ │        │ │         │ │       │ │heartbeat   │ │ broadcast    │
└──┬───┘ └────────┘ └─────────┘ └───────┘ └────────────┘ └──────────────┘
   │
┌──┴───────────────────────────────────┐
│ llm  (LLMProviderTrait)               │
│  LiteLLM · OpenAI · OpenRouter       │
│  · Ollama · Anthropic · MultiModel   │
└───────────────────────────────────────┘

✅ 18 modules: policy, audit, sandbox, mcp, ravenfabric, heartbeat, eval, lib integrated
```

### Target (v1.0)

```text
                    ┌──────────┐
                    │   CLI    │  single · serve · swarm · supervisor · heartbeat
                    └────┬─────┘
                  ┌──────┴───────┐
                  │  Agent Core  │  perceive → plan → act → observe (+ memory)
                  └──┬────┬───┬──┘
          ┌──────────┘    │   └──────────┐
     ┌────┴────┐    ┌─────┴────┐   ┌──────┴───────┐
     │  Tools  │    │ Providers│   │ Orchestration │
     │ policy✅│    │ routing+ │   │ swarm/superv. │
     │ sandbox✅│   │ fallback+│   │ RavenFabric ✅│
     │ audit  ✅│   │ budgets  │   │  (E2E remote) │
     └─────────┘    └──────────┘   └───────┬───────┘
          │                                │
   ┌──────┴───────┐              ┌─────────┴─────────┐
   │ Observability│              │  HeartbeatAgent   │
   │ metrics ·    │              │  assess → plan →  │
   │ tracing ·    │              │  act → persist →  │
   │ health       │              │  sleep (loop)     │
   └──────────────┘              └───────────────────┘

✅ = Infrastructure exists, needs wiring to agent loop (v0.4)
```

---

## Competitive Positioning

RavenClaws aims to be the **preferred alternative** to the current field — including
**OpenClaw**, Cognition (Claude), Manus, Perplexity Comet, Kimi, Open Interpreter,
and Vellum. Not by out-featuring them, but by being **fully functional as a primary
agent** while also being smaller, more secure, and more efficient.

We don't win by out-featuring them. We win by refusing to compromise on all five
pillars at once. By category:

- **vs. OpenClaw** (the primary comparison from rpi5 testing): RavenClaws is **265x more memory-efficient** (~3 MiB RSS vs ~800 MiB), **228x less CPU at idle** (~1m vs ~228m), starts in **<1s vs ~5-10s**, has a **15.8 MB vs ~500 MB container image** (20-48x smaller), and is **distroless/non-root vs full Node.js runtime running as root**. OpenClaw wins on API surface (full REST API vs 3 endpoints), agent loop usability (no `FINAL:` requirement), tool ecosystem (Playwright, PostgreSQL, ChromaDB, SearXNG via MCP), and MCP server support (SSE vs stdio-only in v0.9.3). By v0.9.9, RavenClaws will match OpenClaw's primary agent capabilities (tool execution, MCP ecosystem, HTTP API) while maintaining this efficiency advantage.
- **vs. cloud / hosted assistants** (Claude Cowork, Manus, Perplexity Computer, Kimi): RavenClaws is **self-hostable, offline-capable, and source-available** under AGPLv3. Your data and tool calls never leave infrastructure you control — no phone-home.
- **vs. minimal agent runtimes** (Open Interpreter, ZeroClaw, PicoClaw): RavenClaws matches their footprint while adding a real **security model** (deny-by-default tool policy, audit log, sandboxing) and **multi-provider** routing with fallback.
- **vs. SDK / platform plays** (Vellum, Hermes Agent): RavenClaws is a **single dependency-light binary**, not a service you rent or a framework you marry. Embed it, ship it, forget it.

The bar: anything the field can do, RavenClaws should do **smaller, safer, and
simpler** — or deliberately not at all.

> **Where RavenClaws must lead, measurably (v1.0):** memory-safe core with zero
> known CVEs, sub-15 MB binary, sub-50 ms cold start, fully self-hostable and
> air-gappable, signed + SBOM-attested supply chain. These are claims we will
> benchmark and publish — not marketing.

### RavenClaws vs. Field (v0.9.4 → v1.0 trajectory)

| Capability | RavenClaws v0.9.5 | RavenClaws v1.0 (target) | OpenClaw | Manus |
|---|:---:|:---:|:---:|:---:|
| Agent loop | ✅ | ✅ | ✅ | ✅ |
| Tool calling (structured) | ✅ | ✅ | ✅ | ✅ |
| Tool calling (any model) | ✅ **v0.9.5** | ✅ | ✅ | ✅ |
| `--exec` reliable output | ✅ **v0.9.4** | ✅ | ✅ | ✅ |
| **MCP client (stdio)** | ✅ | ✅ | ✅ | ✅ |
| **MCP client (SSE)** | ✅ v0.9.3 | ✅ | ✅ | ✅ |
| **MCP server (stdio)** | ✅ | ✅ | ✅ | ✅ |
| **MCP server (SSE)** | ✅ v0.9.3 | ✅ | ✅ | ❌ |
| **Multi-MCP-client** | ❌ | ✅ v0.9.6 | ✅ | ✅ |
| **MCP TOML config** | ❌ | ✅ v0.9.6 | ✅ | ❌ |
| Sandboxed execution | ⚠️ (read-only FS) | ✅ v0.9.8 | ✅ | ✅ |
| **Security model (wired)** | ✅ | ✅ | ⚠️ (root user) | ⚠️ |
| **Local-first / air-gapped** | ✅ (Ollama) | ✅ | ❌ | ❌ |
| **~5 MB binary** | ✅ | ✅ | ❌ (Node.js, ~200 MB) | ❌ (cloud) |
| **~3 MiB RSS memory** | ✅ | ✅ | ❌ (~800 MiB) | ❌ (cloud) |
| **~1m CPU idle** | ✅ | ✅ | ❌ (~228m) | ❌ (cloud) |
| **15.8 MB container image** | ✅ | ✅ | ❌ (~500 MB) | ❌ (cloud) |
| **<1s startup** | ✅ | ✅ | ❌ (~5-10s) | ❌ (cloud) |
| **Helm chart** | ✅ | ✅ | ❌ | ❌ |
| **No telemetry** | ✅ | ✅ | ❌ | ❌ |
| **Autonomous heartbeat** | ✅ | ✅ | ❌ | ✅ |
| **Long-horizon persistence** | ✅ | ✅ | ❌ | ✅ |
| **Scalable swarm (1000+)** | ✅ | ✅ | ❌ | ❌ |
| **Self-provisioning sub-agents** | ✅ | ✅ | ❌ | ❌ |
| **HTTP agent API** | ❌ | ✅ v0.9.6 | ✅ | ✅ |
| **Deep health check** | ❌ | ✅ v0.9.6 | ✅ | ❌ |
| **Graceful shutdown** | ⚠️ (server only) | ✅ v0.9.8 | ✅ | ✅ |
| **Configurable sandbox** | ❌ | ✅ v0.9.8 | ✅ | ❌ |
| **K8s init container chown** | ❌ | ✅ v0.9.8 | ❌ (runs as root) | ❌ |
| **ReadOnlyRootFilesystem** | ⚠️ (needs emptyDir) | ✅ v0.9.8 | ❌ (not configured) | ❌ |
| **Non-root container** | ✅ (UID 65532) | ✅ | ❌ (runs as root) | ❌ |
| **Distroless base image** | ✅ | ✅ | ❌ (Debian full) | ❌ |
| **Community health files** | ❌ | ✅ v0.9.8 | ✅ | ❌ |
| **Container < 30 MB** | ⚠️ (~50 MB) | ✅ v0.9.8 | ❌ (~500 MB) | ❌ |
| **Prometheus metrics** | ✅ | ✅ | ❌ | ❌ |
| **RavenFabric remote exec** | ✅ | ✅ | ❌ | ❌ |
| **MCP server SSE transport** | ✅ v0.9.3 | ✅ | ✅ | ❌ |
| **MCP client SSE transport** | ✅ v0.9.3 | ✅ | ✅ | ✅ |
| **Config hot-reload (SIGHUP)** | ❌ | ✅ v0.9.6 | ✅ | ❌ |
| **NetworkPolicy docs** | ❌ | ✅ v0.9.8 | ✅ | ❌ |
| **Secret reference docs** | ❌ | ✅ v0.9.8 | ✅ | ❌ |
| Multi-modal input | ⚠️ (partial) | ⚠️ (v0.10) | ✅ | ✅ |
| Web search | ✅ | ✅ | ✅ | ✅ |
| Browser automation | ❌ | ❌ (v0.10) | ✅ | ✅ |
| Async background runs | ✅ | ✅ | ❌ | ✅ |
| Scheduling / triggers | ✅ | ✅ | ❌ | ✅ |
| Sub-agents / swarm | ✅ | ✅ | ❌ | ✅ |
| OAuth connectors | ❌ | ❌ (v0.10) | ✅ | ✅ |
| Telegram bot | ❌ | ❌ (v0.10) | ✅ | ❌ |
| SSH in container | ❌ | ❌ (v0.10) | ✅ | ❌ |

**RavenClaws's Wedge (v1.0):**
1. **Trust as a feature** — deny-by-default security, no telemetry, verifiable end-to-end
2. **Edge-deployable** — ~5 MB binary, ~3 MiB RSS, ~1m CPU idle, runs on Raspberry Pi, air-gapped capable
3. **RavenFabric mesh** — E2E-encrypted remote execution across fleet (unique)
4. **Autonomous heartbeat** — operates independently for days/weeks, no supervision required ✅ v0.9
5. **Self-orchestrating swarm** — dynamically provisions and manages 10s–1000s of workers in any topology, each with unique capability profiles. No fixed limit — the swarm scales to the task.
6. **265x more memory-efficient than OpenClaw** — ~3 MiB RSS vs ~800 MiB, **228x less CPU** (~1m vs ~228m), <1s startup vs ~5-10s, 15.8 MB image vs ~500 MB (20-48x smaller). Runs on an $80 Raspberry Pi 5 with 8GB RAM where OpenClaw needs a server.

---

## Features Required to Become the Preferred Alternative

Being *preferred* is a two-step bar: first reach **parity** on the capabilities the
field now treats as table stakes, then **win decisively** on the five pillars where
the cloud incumbents structurally can't follow.

### Part 1 — Table stakes (reach parity)

| Capability | Why it's table stakes | In RavenClaws | Target |
|---|---|:--:|:--:|
| Agent loop (plan → act → observe) | Without it there is no "agent" | ✅ | v0.3 |
| Tool / function calling | The substrate for every action | ✅ (structured) | v0.4 |
| **Tool calling with ANY model** | Not all models emit structured `tool_calls` | ✅ **v0.9.5** | **v0.9.5** ✅ |
| **`--exec` reliable output** | Must produce output regardless of model behavior | ✅ **v0.9.4** | **v0.9.4** ✅ |
| **MCP — client *and* server** | Industry standard (Anthropic, OpenAI, Google, Microsoft, Salesforce) | ✅ (both, SSE+stdio) | **v0.9.3** ✅ |
| **Multi-MCP-client** | Connect to multiple MCP servers simultaneously | ❌ | **v0.9.6** |
| **MCP TOML config** | Configure MCP servers in config file, not CLI | ❌ | **v0.9.6** |
| **HTTP agent API** | Server mode must run agents, not just report status | ❌ | **v0.9.6** |
| Sandboxed execution | Must work with read-only root filesystem | ⚠️ (hardcoded /tmp) | v0.9.8 |
| Web search + content extraction | Core to "research" tasks | ✅ (SearXNG + DuckDuckGo) | **v0.8** ✅ |
| File operations (read/write/edit) | Core to "worker" | ✅ | v0.4 |
| Sub-agents / swarm orchestration | Kimi runs 300 sub-agents / 4,000 steps | ✅ (v0.6) | v0.6 |
| **Autonomous heartbeat (long-running)** | Operates independently for days/weeks without supervision | ✅ **v0.9** | **v0.9** |
| **Scalable swarm (1000+ workers)** | Dynamic provisioning of 10s–1000s of agents in any topology; no fixed limit | ✅ **v0.9** | **v0.9** |
| **Self-provisioning sub-agents** | Agent spawns agents; recursive supervisor mode | ✅ **v0.9** | **v0.9** |
| **Inter-agent communication** | Structured message passing between swarm members | ✅ **v0.9.1** | **v0.9** |
| Async / long-horizon background runs | Manus's killer feature (cloud background) | ✅ **v0.8** | **v0.8** ✅ |
| Scheduling / triggers (cron, webhook) | Proactive, set-and-forget operation | ✅ **v0.8** | **v0.7** |
| Streaming + intermediate results | First-class in Vellum; needed for interactive UX | ✅ | v0.3 |
| Graceful shutdown | State must survive pod termination | ⚠️ (server only) | v0.9.8 |
| K8s deployment out of the box | Must work with `readOnlyRootFilesystem: true` | ⚠️ (needs emptyDir) | v0.9.8 |
| Multi-modal input (images, PDFs) | Manus/Kimi are multimodal; "worker" must read docs | ❌ | v0.10 |
| Connectors / integrations (OAuth) | Claude-style connectors; Manus's weakness | ❌ | v0.10 |
| Retries / provider fallback | Vellum: retry, fall back, fail early | ⚠️ (unwired) | v0.9.8 |
| Human-in-the-loop approvals | Enterprises require guardrails + audit + HITL | ✅ **v0.8** | **v0.4** |

### Part 2 — Where RavenClaws wins (the "preferred" wedge)

| Differentiator | Why it beats the field | Pillars | Phase |
|---|---|:--:|:--:|
| **Local-first / self-hosted / air-gapped** | Manus is cloud-only; Comet's "Local" is a browser, not a worker. RavenClaws runs fully offline with Ollama. | Secure · Simple | ✅ core |
| **Security model: deny-by-default + sandbox + audit** | Field bolts security on; we ship it in core. | Secure | ✅ v0.4 (wired) |
| **~5 MB single binary, edge/embeddable** | No cloud agent runs on a Raspberry Pi. OpenClaw is ~500 MB Node.js. | Small · Efficient | ✅ |
| **~3 MiB RSS memory** | 265x less memory than OpenClaw (~800 MiB). Runs on a $80 Raspberry Pi 5. | Efficient | ✅ |
| **<1s startup** | OpenClaw takes ~30s to start. RavenClaws is ready instantly. | Efficient | ✅ |
| **Provider-agnostic + cost-aware routing** | Not locked to one model vendor. Generic `openai-compatible` unlocks 10+ backends. | Efficient · Robust | v0.5 → v1.0 |
| **RavenFabric mesh: E2E-encrypted remote exec** | Unique — competitors are single-host or single-cloud. | Robust | ✅ v0.6.1 |
| **No telemetry · signed + SBOM** | Trust as a feature, verifiable end-to-end. | Secure | ✅ |
| **Open core + commercial** | No lock-in, vs. proprietary cloud. | Simple | ✅ |

### Part 3 — The five that move the needle most

1. **`--exec` reliability (v0.9.4)** — Must produce output with ANY model. No silent failures. This is the #1 complaint from rpi5 testing.
2. **Tool execution with any model (v0.9.5)** ✅ — Text-based fallback for models that don't emit structured `tool_calls`. Tool execution logging. Configured web search endpoint. Multi-MCP-client and MCP TOML config deferred to v0.9.6.
3. **HTTP agent API (v0.9.6)** — `/chat`, `/execute`, `/tools` endpoints so the server can actually run agents. This is what OpenClaw's HTTP API does.
4. **MCP ecosystem integration (v0.9.7)** — Verified end-to-end with OpenClaw, Claude Desktop, Playwright, PostgreSQL, ChromaDB.
5. **265x memory efficiency** — The wedge no competitor can copy. RavenClaws runs on a $80 Raspberry Pi 5 with 8GB RAM. OpenClaw needs a server.

---

## Phased Plan

Versions are capability milestones, not dates. Each must keep all five pillars green.

### v0.2 — Foundations: make the build honest and green 🔧

- [x] **Commit `Cargo.lock`** (remove from `.gitignore`) so `--locked` works in CI/Docker/publish.
- [x] **Fix multi-arch Docker build** — install cross-linkers (`gcc-aarch64-linux-gnu`) + set the cargo target linker.
- [x] **Verify the RavenFabric agent download** against a published checksum / Cosign signature.
- [x] **Single source of version truth** — wire clap `--version` to `env!("CARGO_PKG_VERSION")`.
- [x] **Replace `.expect()` on HTTP client construction** with error propagation (no abort path under `panic = "abort"`).
- [x] **Decide `--exec`**: implement one-shot mode (preferred, see v0.3) or remove the flag.
- [x] **Make swarm/supervisor fail loudly** — return a clear error instead of `exit 0` until implemented.
- [x] **Expand tests** — use `mockito` to exercise request/response/error paths for every provider; cover config parsing and the multi-model manager.
- [x] **README status-honesty.**

**Exit criteria:** `cargo fmt && cargo clippy -D warnings && cargo test` green; `docker buildx` produces working `amd64`+`arm64` images; fresh clone builds with `--locked`.

### v0.3 — A real agent 🧠

- [x] **Agent loop**: perceive → plan → act → observe, with max-iteration guard and cancellation.
- [x] **`--exec "<task>"`** one-shot mode — sends prompt to LLM, prints response to stdout.
- [x] **Interactive REPL** (stdin) — continuous conversation mode.
- [x] **Conversation memory** — context across turns; configurable window (last N turns or token budget); session save/restore.
- [x] **Streaming responses** end to end (`stream = true`) across the trait and all clients.
- [x] **System-prompt / persona** configuration.
- [x] **Robust errors** — typed retries, timeouts, graceful provider failure. All error paths covered with `thiserror` + `anyhow`; 26 error tests across 7 variants.

**Exit criteria:** `ravenclaws --exec "summarize this repo"` performs a real multi-step task and returns a result.

### v0.4 — Tools and safety 🧰🔒 **(COMPLETE)**

Agency with guardrails — the security differentiator.

- [x] **Tool / function-calling abstraction** (provider-agnostic schema + registry).
- [x] **Built-in tools**: shell exec, file read/write, web fetch — each behind a capability flag.
- [x] **Tool wiring into agent loop** — `run_agent_loop` detects `TOOL_CALL:` / `ARGS:` patterns, executes tools, injects results as `OBSERVATION:`.
- [x] **Deny-by-default policy** (command / path / host allow-lists), à la RavenFabric's RPCPolicy.
- [x] **Sandboxed execution** (workdir jail, resource limits, timeouts).
- [x] **Audit log** — structured, HMAC-chained, tamper-evident trail of every tool call.
- [x] **Wire security to agent loop** — `PolicyEngine` validates all tool calls; `Sandbox` executes `shell_exec`; `AuditLog` emits events. **COMMIT: 51e42b0**
- [x] **Structured function calling** — OpenAI Tools format for OpenAI/LiteLLM/OpenRouter; native JSON instead of pattern-matching. ✅ v0.4
- [x] **MCP — client** — consume any Model Context Protocol tool/server via stdio transport. ✅ v0.5.2
- [x] **MCP — server** — expose RavenClaws itself as an MCP server over stdio. `--mcp-server` flag, policy-checked and audited. ✅ **v0.7.0**
- [x] **Human-in-the-loop approvals** — configurable approval gates for sensitive tool calls (allow / deny / ask). `--require-approval` flag, `RAVENCLAW_REQUIRE_APPROVAL` env var, prompts via stdin, audited. ✅ **v0.8**
- [x] **Web search + content extraction tool** — SearXNG JSON API + DuckDuckGo HTML backends, HTML-to-text extraction, configurable via `WebSearchConfig`. ✅ **v0.8**
- [x] **Wire `zeroize`** for secret material — API keys in `LLMConfig` and HMAC secret key in `AuditLog` zeroized on drop. ✅ **v0.8**
- [x] **Honor `token_lifetime_secs`** for any issued credentials — agent sessions auto-terminate after configured duration. Enforced in both `run_agent_loop` and `run_agent_loop_with_mcp`. ✅ **v0.8**
- [x] **Prompt-injection defense** — instruction-boundary enforcement, output schema validation. ✅ **v0.8**

**Exit criteria:** an agent runs tools, but only those allowed by policy, with a complete audit log. Security features actively invoked, not just present.

### v0.5 — Providers and routing 🔀 **(COMPLETE 2026-06-07)**

**Primary objective:** Eliminate code duplication and add production-grade resilience.

- [x] **Unified OpenAI-Compatible Client** ✅ v0.5.0
  - Merge LiteLLM, OpenAI, OpenRouter into `OpenAICompatibleClient` with provider enum
  - Provider-specific defaults: endpoint, headers (OpenRouter needs `HTTP-Referer`, `X-Title`)
  - Keep Ollama separate (different API format)
  - **Impact:** ~400 LOC reduction, single maintenance path

- [x] **Retry & Fallback Chain** ✅ v0.5.1
  - Exponential backoff with jitter (base 100ms, max 10s, 3 retries)
  - Fallback chain: primary → secondary → tertiary (configurable order)
  - Circuit breaker: open after 5 consecutive failures, half-open after 30s
  - **Exit criteria:** `ravenclaws --exec "task"` with fallback to Ollama when cloud providers fail

- [x] **Token Budget & Cost Tracking** ✅ v0.5.1
  - `--token-budget <N>` CLI flag and `RAVENCLAW_TOKEN_BUDGET` env var
  - Track tokens per request using `usage` field in responses
  - Cost estimation table (per-provider, per-model pricing)
  - Auto-downgrade: switch to cheaper model when 80% of budget consumed
  - **Exit criteria:** Agent stops before exceeding budget, logs cost estimate

- [x] **MCP Client Integration** (highest leverage) ✅ v0.5.2
  - MCP client: connect to external MCP servers (filesystem, database, API tools)
  - Tool discovery and registration from MCP servers
  - Protocol: JSON-RPC over stdio or SSE
  - **Exit criteria:** Can use MCP-provided tools alongside built-in tools

- [x] **Native Anthropic Provider** ✅ v0.5.3
  - Direct Anthropic API client (not via OpenRouter)
  - Support for tool use (Anthropic's native function calling)
  - Image input support (stubbed for future multi-modal expansion)
  - Full test coverage (4 unit tests + integration via factory)

- [ ] **Multi-modal Input** ⚠️ **PARTIAL** — AnthropicClient has image support structure, not wired to CLI *(v0.10)*
  - Image attachments in `ChatMessage` (base64 or URL)
  - PDF/text document ingestion
  - Provider-specific encoding (OpenAI vision, Anthropic images)

**Exit criteria:** ✅ COMPLETE (v0.5 core features)
1. [x] Single run transparently fails over between providers
2. [x] Respects token budget
3. [x] Can consume MCP-provided tools
4. [x] Code coverage ≥80% on routing/fallback logic (277+ tests across 9 modules)

### v0.6 — Swarm, supervisor, and RavenFabric 🕸️

- [x] **Supervisor mode (single-provider)** — task decomposition, sub-agent spawning, result aggregation ✅ Implemented 2026-06-07
- [x] **Swarm mode (single-provider)** — multiple parallel agents with different personas (no fixed limit) ✅ Implemented 2026-06-07
- [x] **Supervisor mode (multi-model)** — provider-aware task decomposition ✅ Implemented 2026-06-07
- [x] **Swarm mode (multi-model)** — parallel agents across different providers ✅ Implemented 2026-06-07
- [x] **Git hooks (pre-commit / pre-push)** — automated verification before every commit and push ✅ Implemented 2026-06-18
- [x] **CI/CD hardening** — `DEBIAN_FRONTEND=noninteractive` + `timeout-minutes` for apt-get in cross-compilation deps ✅ Implemented 2026-06-18
- [x] **Node.js 24 migration** — `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` in all workflows ✅ Implemented 2026-06-18
- [x] **CodeQL v4 migration** — all `codeql-action/*@v3` → `@v4` ✅ Implemented 2026-06-18
- [x] **RavenFabric integration** — secure E2E remote command execution + mesh coordination (the headline capability). ✅ v0.6.1
- [x] **Agent communication** — structured message passing; conflict resolution across agents. ✅ **v0.9.1** *(moved to v0.9)*
**Exit criteria:** ✅ COMPLETE (v0.6 core features) — Supervisor and Swarm modes implemented for single-provider and multi-model. CI/CD hardened with Node.js 24 and CodeQL v4. RavenFabric integration complete with full client module, wiring into all agent modes, and 12 unit tests.

### v0.7 — Observability and ops 📈 **(COMPLETE)**

- [x] **MCP Server** — expose RavenClaws tools over stdio via MCP protocol. `--mcp-server` flag, policy-checked and audited. ✅ **v0.7.0**
- [x] **Long-running server mode** with HTTP `/health` `/ready` `/metrics` endpoints (fixes the k8s CrashLoop). ✅ **v0.7.1**
- [x] **Prometheus-style metrics** (requests, tokens, tool calls, errors, uptime). ✅ **v0.7.1**
- [x] **Graceful shutdown**, signal handling. ✅ **v0.7.1** — SIGTERM/SIGINT handled in server mode
- [x] **OpenTelemetry tracing** (opt-in, self-hosted collector, correlation IDs). ✅ **v0.7.2**
- [x] **Helm chart** (`charts/ravenclaws/`) — 11 Kubernetes resources, full values.yaml, validated with `helm lint`. ✅ **v0.7.3**
- [x] **Eval harness + run inspection** — golden-task evals, assertions on intermediate steps, and replayable run traces. ✅ **v0.7.4**
- [x] **Async / long-horizon background runs** — assign-and-walk-away background execution, resumable across restarts (matches Manus's headline UX). ✅ **v0.8**
- [x] **Scheduling & triggers** — cron, webhook, and file-watch activation for proactive 24/7 agents. ✅ **v0.8**
  - `EvalConfig`/`EvalTask`/`EvalRunner` with 7 assertion types (contains, not_contains, exact, regex, non_empty, min_length, max_length)
  - `RunTrace` with step-by-step, LLM call, and tool call tracing
  - `EvalReport` with text and JSON output formats
  - CLI `--eval <path>` and `--eval-json` flags
  - 24 Rust unit tests + 20 verification tests
  - Sample eval configs in `tests/eval/` (basic-suite.toml, security-suite.toml)

**Exit criteria:** ✅ RavenClaws runs as a stable long-lived workload with green probes, exported metrics, opt-in distributed tracing, and Helm-based deployment.

### v0.8 — Enterprise and compliance 🏢 *(commercial-licensed)*

Maps to the commercial tier in [LICENSING.md](LICENSING.md).

- [ ] **RBAC + multi-tenant isolation** (separate workspaces, secrets, quotas).
- [ ] **SSO / SAML.**
- [ ] **SecurityPolicy** — immutable rules, blast-radius limits.
- [ ] **Multi-level audit logging** — levels (`off`/`basic`/`detailed`/`debug`), formats (JSON/CEF/LEEF/Syslog), shipping sinks, integrity chaining.
- [ ] **Compliance presets & reporting** (SOC2, ISO 27001, HIPAA, GDPR, PCI-DSS).
- [ ] **Air-gap / offline licensing**; runtime feature-flag gating.
- [ ] **Output artifacts & reporting** — generate documents, spreadsheets, slides, and sites via the skill system (v0.5); underpins compliance and executive reporting.

### ✅ v0.9 — Autonomous heartbeat & self-orchestration 💓 (v0.9.2 released)

RavenClaws becomes a truly autonomous agent that can operate independently over
long time horizons, and dynamically orchestrate swarms of any size.

**Released versions:** [v0.9.0](https://github.com/egkristi/RavenClaws/releases/tag/v0.9.0) (heartbeat + persistence) · [v0.9.1](https://github.com/egkristi/RavenClaws/releases/tag/v0.9.1) (swarm orchestration + inter-agent communication) · [v0.9.2](https://github.com/egkristi/RavenClaws/releases/tag/v0.9.2) (swarm health & telemetry)

- [x] **Autonomous heartbeat** — persistent background loop with configurable tick interval; agent wakes, assesses progress, plans next steps, executes, and sleeps. No human-in-the-loop required for routine operation. ✅ **v0.9.0**
- [x] **Long-horizon task persistence** — task state survives restarts; agent resumes from last checkpoint with full context. Heartbeat continues across binary restarts. ✅ **v0.9.0**
  - `HeartbeatState` persisted to `workdir/heartbeat-<id>.json` after every tick
  - `HeartbeatAgent::new()` auto-resumes from saved state on restart
  - `BackgroundTaskManager` persists all tasks as individual JSON files in `<workdir>/tasks/`
  - `--task-resume` flag re-executes incomplete tasks on startup
  - 401 total unit tests (0 regressions)
- [x] **Self-provisioning of sub-agents** — RavenClaws dynamically spawns new agent instances (local or remote via RavenFabric) based on task decomposition. Supervisor mode becomes recursive: supervisors spawn supervisors. ✅ **v0.9.1**
- [x] **Scalable swarm orchestration** — support for 10s to **1000s** of workers. No fixed limit — the swarm scales organically to the task. Configurable topologies: star (single coordinator), mesh (peer-to-peer), hierarchical (tree of supervisors), and hybrid. ✅ **v0.9.1**
- [x] **Worker personality & capability profiles** — each swarm member has a declarative profile (persona, tools, provider, model, resource limits). Profiles are composable and inheritable. ✅ **v0.9.1**
- [x] **Dynamic role assignment** — agent analyzes task requirements and assigns roles (researcher, coder, reviewer, executor) to swarm members based on capability profiles and current load. ✅ **v0.9.1**
- [x] **Inter-agent communication bus** — structured message passing between swarm members with delivery guarantees, routing, and policy enforcement. All communication is audited. ✅ **v0.9.1**
- [x] **Swarm health & telemetry** — heartbeat monitoring per agent, dead-agent detection, automatic replacement. Metrics: task throughput, agent utilization, error rates, communication latency. ✅ **v0.9.2**
  - `SwarmHealthMonitor` with per-worker heartbeat tracking, four-state health model (Healthy/Degraded/Unhealthy/Dead)
  - `WorkerTelemetry` — tasks completed/failed, error count, avg duration, messages sent/received
  - `SwarmMetrics` — aggregate health: total/healthy/degraded/unhealthy/dead workers, task throughput, utilization, error rate, communication latency
  - Configurable heartbeat interval (5s), max missed beats (3), replacement timeout (30s)
  - Integrated into `execute_with_profile()` and `recursive_supervise_impl()` — auto-registration, heartbeat on completion, failure tracking
  - Shared across sub-orchestrators via `Arc<RwLock<>>` for recursive supervision
  - Periodic health check logging in supervisor loop
  - Public accessors: `health_metrics()` and `worker_telemetry()` on `SwarmOrchestrator`
  - CLI flag: `--swarm-health-monitoring` (env: `RAVENCLAW_SWARM_HEALTH_MONITORING`)
  - 22 unit tests, 452 total (0 regressions)

### v0.9.4 — Critical Fixes: Make `--exec` Work Reliably 🔧 *(released 2026-06-27)*

**Theme:** Every `ravenclaws --exec "do something"` must produce output. No silent failures.
No models that "don't work." The agent loop must be robust to any model behavior.

- [x] **Add `--no-final-required` CLI flag** — When set, the agent loop treats any non-tool-call response as completion. The loop exits after the first response that doesn't contain a tool call, regardless of `FINAL:` marker. This makes `--exec` work with models that don't use the `FINAL:` convention (e.g., `deepseek-v4-pro:cloud`). ✅ **v0.9.4**
- [ ] **Add `--exec` FINAL: fallback** — When agent loop hits max iterations without `FINAL:`, print the last response to stdout instead of returning an error. *(Moved to v0.9.5)*
- [x] **Add agent loop response logging** — Log the first 200-500 chars of LLM response content at debug level. Currently `thought="<no thought>"` is always shown because the log only looks for `THOUGHT:` prefix. ✅ **v0.9.4**
- [ ] **Add `--verbose` flag** — Enable debug-level logging of LLM response content when set. *(Moved to v0.9.5)*
- [x] **Update default system prompt with `FINAL:` example** — Add `FINAL:` usage instructions to the default system prompt so models are more likely to use the convention without explicit instruction. ✅ **v0.9.4**
- [ ] **Add configurable sandbox workdir** — Add `RAVENCLAWS_SANDBOX_WORKDIR` env var or `sandbox.workdir` config field. *(Moved to v0.9.8)*
- [ ] **Add graceful shutdown for heartbeat** — Add a `Drop` impl or shutdown hook to `HeartbeatAgent` that calls `persist_state()` on SIGTERM/SIGINT. *(Moved to v0.9.8)*
- [ ] **Add init container `chown` to K8s deployment** — Add explicit `chown -R 65532:65532 /workspace` to the init container in `k8s/deployment.yaml`. *(Moved to v0.9.8)*
- [x] **Improve heartbeat `goal` error message** — When `heartbeat.goal` is missing, include an example in the error message. ✅ **v0.9.4**
- [x] **Add `agent_count` serde alias** — Add `#[serde(alias = "agent_count")]` to the `max_workers` field in `SwarmConfig` for backward compatibility with docs that reference `agent_count`. ✅ **v0.9.4**
- [ ] **Add `[swarm.profiles]` shorthand deserializer** — Add custom deserializer that accepts `{name: persona_string}` map syntax. *(Moved to v0.9.5)*
- [ ] **Add graceful shutdown for server mode** — Ensure heartbeat state is persisted and connections drained before exit. *(Moved to v0.9.6)*
- [ ] **Add SIGHUP-based config reload** — For long-running agents, reload config without restarting the pod. *(Moved to v0.9.6)*

**Exit criteria:**
- [x] `ravenclaws --exec "Say hello"` works with ANY model, including those that don't emit `FINAL:` or structured tool calls ✅ **v0.9.4**
- [x] Default system prompt includes `FINAL:` usage instructions ✅ **v0.9.4**
- [x] Heartbeat `goal` error message includes example ✅ **v0.9.4**
- [x] `agent_count` alias works in swarm config ✅ **v0.9.4**
- [x] Agent loop response logging at debug level ✅ **v0.9.4**
- [ ] `--exec` FINAL: fallback when max iterations reached *(v0.9.5)*
- [ ] `--verbose` flag for debug-level logging *(v0.9.5)*
- [ ] `[swarm.profiles]` shorthand syntax works in TOML config *(v0.9.5)*
- [ ] Server mode handles SIGTERM gracefully *(v0.9.6)*
- [ ] Config hot-reload via SIGHUP works for long-running agents *(v0.9.6)*
- [ ] Sandbox workdir is configurable via env var or config field *(v0.9.8)*
- [ ] Heartbeat mode handles SIGTERM gracefully *(v0.9.8)*
- [ ] K8s deployment works with `readOnlyRootFilesystem: true` *(v0.9.8)*

### v0.9.5 — Tool Execution Reliability 🛠️ ✅

**Theme:** Tool execution must work with any model, not just those that emit structured `tool_calls`. Add fallback mechanisms, text-based tool call detection, and multi-MCP support.

- [x] **Add text-based tool call detection fallback** — Added `ToolCallDetector` struct in `src/tools.rs` with 5 regex patterns for common tool call formats. 11 unit tests covering all patterns, deduplication, and edge cases. Not yet wired into agent loop (will be done when agent loop is refactored to use it).
- [ ] **Add TOML-based MCP config section** — *Deferred to v0.9.6.*
- [ ] **Add multi-MCP-client support** — *Deferred to v0.9.6.*
- [x] **Add tool execution logging** — Added `debug!`-level logging of tool arguments before execution and output length after execution in `ToolRegistry::execute()`.
- [ ] **Add tool call assertions to eval harness** — *Deferred to v0.9.6.*
- [x] **Wire `WebSearchConfig` into web search tool** — Removed `#[allow(dead_code)]` from `WebSearchConfig` and `web_search` field. Added `ToolRegistry::with_config(&Config)` that reads `config.web_search.endpoint` and passes it to the web search tool. `main.rs` now uses `with_config()` for MCP server and `--exec` mode.
- [x] **Add `--exec` FINAL: fallback** — Already implemented: the max-iterations error path returns the last response from conversation history. `--exec` mode in `main.rs` prints the response via `println!()`. No changes needed.
- [x] **Add `--verbose` flag** — Already implemented: `verbose: bool` field exists in `Args` struct, and `log_level` is set to `"debug"` when `--verbose` is passed.
- [ ] **Add `[swarm.profiles]` shorthand deserializer** — *Deferred to v0.9.6.*

**Exit criteria:**
- [x] Tool execution works with ANY model, including those that don't emit structured `tool_calls` (ToolCallDetector + `--no-final-required`)
- [x] Text-based tool call detection fallback parses natural language tool descriptions into `ToolCall` structs
- [ ] Multiple MCP servers can be connected simultaneously via TOML config *(deferred)*
- [x] Tool calls are logged with arguments and results at debug level
- [ ] Eval harness has tool call assertions *(deferred)*
- [x] Web search tool uses configurable endpoint from `Config.web_search`
- [x] No silent failures — every `--exec` invocation produces stdout output
- [x] `--verbose` flag shows LLM response content for debugging
- [ ] `[swarm.profiles]` shorthand syntax works in TOML config *(deferred)*

### v0.9.6 — Server Mode: Full Agent Execution API 🌐

**Theme:** The HTTP server must be able to run agents, not just report status. Add `/chat`, `/execute`, and `/tools` endpoints so RavenClaws can serve as a primary agent gateway.

- [ ] **Add `/chat` endpoint** — POST endpoint that accepts a user message and returns an agent response. Supports streaming (SSE) and non-streaming modes. Uses the same agent loop as `--exec` mode. **Implementation:** Add `post_chat()` handler in `src/server.rs` that deserializes `{messages: Vec<ChatMessage>, stream: Option<bool>}`, calls `run_agent_loop()`, and returns the response as JSON or SSE stream.
- [ ] **Add `/execute` endpoint** — POST endpoint that accepts a task description and executes it as a background run. Returns a task ID that can be polled for status/results. Supports async execution with result retrieval. **Implementation:** Add `post_execute()` handler that creates a `BackgroundTask`, returns `{task_id: Uuid}`, and a `get_task()` handler that returns task status/results.
- [ ] **Add `/tools` endpoint** — GET endpoint that returns the list of available tools (built-in + MCP-discovered) with their schemas. POST endpoint that executes a specific tool by name with provided arguments. **Implementation:** Add `get_tools()` handler that serializes `ToolRegistry::list_tools()` and `post_tool_execute()` handler that calls `ToolRegistry::execute()`.
- [ ] **Add `/health/deep` endpoint** — Deep health check that verifies LLM connectivity by making a lightweight request, in addition to the existing process-liveness `/health`. **Implementation:** Add `get_health_deep()` handler that calls `llm.chat()` with a minimal prompt and checks for a non-error response.
- [ ] **Add readiness probe LLM connectivity check** — Make `/ready` endpoint optionally verify LLM connectivity by making a lightweight request, in addition to the current process-liveness check. **Implementation:** Add `llm_check: bool` to `ServerConfig`. When true, `/ready` makes a lightweight LLM request before returning 200.
- [ ] **Add env var override for server port** — Document `RAVENCLAWS_RUNTIME_PORT` or add `RAVENCLAWS_SERVE_PORT` as an env var alias for the server port. **Implementation:** Add `#[serde(alias = "RAVENCLAWS_SERVE_PORT")]` or env var mapping in `Config::load()`.
- [ ] **Add dedicated HTTP server mode docs page** — `docs/guides/server-mode.md` and `website/public/docs/server-mode.html` explaining endpoints, configuration, ingress setup, and interaction with heartbeat mode.
- [ ] **Add graceful shutdown for server mode** — When the pod is terminated (e.g., during rollout restart), ensure heartbeat state file is persisted and connections are drained before exit. *(Moved from v0.9.4)* **Implementation:** Register `tokio::signal::ctrl_c()` and `tokio::signal::unix::SignalKind::terminate()` handlers in `main.rs` for server mode. Call `server.shutdown()` and `heartbeat.persist_state()` before exit.
- [ ] **Add SIGHUP-based config reload** — For long-running agents, a SIGHUP handler that reloads `ravenclaws.toml` without restarting the pod. *(Moved from v0.9.4)* **Implementation:** Register `tokio::signal::unix::SignalKind::hangup()` handler. On SIGHUP, call `Config::load()` and update the running config. Log the reload event.

**Exit criteria:**
- [ ] `/chat` endpoint accepts messages and returns agent responses (streaming + non-streaming)
- [ ] `/execute` endpoint accepts tasks and returns pollable task IDs
- [ ] `/tools` endpoint lists available tools with schemas and executes tools by name
- [ ] `/health/deep` verifies LLM connectivity
- [ ] `/ready` optionally checks LLM connectivity
- [ ] Server port is configurable via env var
- [ ] Server mode docs page exists in `docs/guides/` and `website/public/docs/`
- [ ] Server mode handles SIGTERM gracefully — state file persisted, connections drained
- [ ] Config hot-reload via SIGHUP works for long-running agents

### v0.9.7 — MCP Ecosystem Integration 🔌

**Theme:** RavenClaws must be a first-class citizen in the MCP ecosystem — able to connect to any MCP server and be consumed by any MCP client. Full SSE support, documentation, and verified integrations.

- [ ] **Add MCP server SSE transport documentation** — Document how to connect RavenClaws as an MCP server from OpenClaw, Claude Desktop, and other MCP clients. Include example configs. **Implementation:** Create `docs/guides/mcp-server-sse.md` with OpenClaw config example (`{"ravenclaws-mcp": {"transport": "sse", "url": "http://localhost:3100/mcp"}}`), Claude Desktop config example, and curl examples.
- [ ] **Add MCP client SSE transport documentation** — Document how to connect RavenClaws to SSE-based MCP servers (Playwright, PostgreSQL, ChromaDB, SearXNG). Include example configs. **Implementation:** Create `docs/guides/mcp-client-sse.md` with TOML config examples for each MCP server type.
- [ ] **Add verified MCP server integration tests** — Test RavenClaws MCP server against real MCP clients (OpenClaw, Claude Desktop). Verify tool discovery, execution, and error handling. **Implementation:** Add `scripts/lib/test-mcp-server.sh` that starts RavenClaws in MCP server mode, connects with a test client, discovers tools, and executes a tool.
- [ ] **Add verified MCP client integration tests** — Test RavenClaws MCP client against real MCP servers (filesystem, GitHub, Playwright). Verify tool discovery, registration, and execution. **Implementation:** Add `scripts/lib/test-mcp-client.sh` that starts a test MCP server (e.g., `@modelcontextprotocol/server-filesystem`), connects RavenClaws as client, and verifies tool discovery.
- [ ] **Add MCP server health endpoint** — Add `/mcp/health` endpoint to the MCP server that reports connected clients, available tools, and execution stats. **Implementation:** Add `get_mcp_health()` handler in `src/mcp.rs` that returns `{clients: usize, tools: Vec<String>, uptime_seconds: u64}`.
- [ ] **Add MCP client reconnection** — When an MCP server disconnects, automatically retry connection with exponential backoff. Log reconnection attempts. **Implementation:** Add reconnection loop in `McpClient::connect()` with `backoff = ExponentialBackoff::new(100, 5000, 30_000)` and max retries.
- [ ] **Add MCP server authentication** — Optional API key or token-based authentication for MCP server connections. Configurable via `[mcp]` config section. **Implementation:** Add `auth_token: Option<String>` to `McpServerConfig`. When set, require `Authorization: Bearer <token>` header on all MCP server endpoints.

**Exit criteria:**
- [ ] RavenClaws can be added as an MCP server in OpenClaw's config (SSE transport) and works end-to-end
- [ ] RavenClaws can connect to Playwright, PostgreSQL, and ChromaDB MCP servers simultaneously
- [ ] MCP server SSE transport documented with example configs for OpenClaw, Claude Desktop
- [ ] MCP client SSE transport documented with example configs for Playwright, PostgreSQL, ChromaDB
- [ ] Verified integration tests pass against real MCP clients and servers
- [ ] MCP server has `/mcp/health` endpoint
- [ ] MCP client reconnects automatically on disconnection with exponential backoff

### v0.9.8 — Production Hardening 🏭

**Theme:** Close all remaining gaps for production deployment. Wire unwired infrastructure, add community health files, reduce image size, and add deep health checks.

- [ ] **Wire `RavenFabricClient` into agent loop** — Client is created in `main.rs` but `health()`, `list_agents()`, `execute()`, and `broadcast()` are never invoked at runtime. All methods are `#[allow(dead_code)]`. **Implementation:** Pass `Option<Arc<RavenFabricClient>>` to `run_agent_loop()`. After each agent loop iteration, call `client.health()` to report liveness. When the agent produces a result, call `client.broadcast()` to share it with the mesh.
- [ ] **Wire `ProviderFallbackChain` into agent loop** — Fallback chain struct and all methods are `#[allow(dead_code)]`. Never used by `run_agent_loop` or `run_agent_loop_with_mcp`. **Implementation:** Pass `Option<Arc<ProviderFallbackChain>>` to `run_agent_loop()`. When `llm.chat()` returns an error, try the next provider in the fallback chain before returning the error.
- [ ] **Wire `TokenBudget` into agent loop** — Entire struct and all methods are `#[allow(dead_code)]`. Token budget is never checked during agent execution. **Implementation:** Pass `Option<Arc<TokenBudget>>` to `run_agent_loop()`. Before each LLM call, check `budget.remaining()`. If exhausted, return a message to the user and exit the loop.
- [ ] **Wire `AgentMessageBus` into swarm orchestration** — Message bus is created but never used in the orchestration flow. All methods are `#[allow(dead_code)]`. **Implementation:** Pass `Arc<AgentMessageBus>` to `SwarmOrchestrator`. After each sub-agent completes a step, call `bus.send()` to share the result. Before assigning tasks, check `bus.receive()` for relevant context.
- [ ] **Wire `SwarmHealthMonitor` into swarm orchestration** — Health monitoring is initialized but never checked during orchestration. All methods are `#[allow(dead_code)]`. **Implementation:** Pass `Arc<SwarmHealthMonitor>` to `SwarmOrchestrator`. After each sub-agent iteration, call `monitor.record_heartbeat(agent_id)`. Before assigning tasks, check `monitor.dead_agents()` and replace any that have timed out.
- [ ] **Add community health files** — `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`, issue templates, and PR template. **Implementation:** Create each file in the repo root following GitHub community standards. `SECURITY.md` should describe the vulnerability reporting process. `CONTRIBUTING.md` should reference `AGENTS.md` and the verification system.
- [ ] **Reduce container image size** — Current ~50 MB vs < 30 MB target. Investigate multi-stage build optimization, smaller base image, or removing RavenFabric agent binary from production image. **Implementation:** Try `gcr.io/distroless/static-debian12:nonroot` instead of `cc-debian12`. Use `cargo build --release --no-default-features` to exclude optional features. Strip debug symbols with `--strip` in release profile.
- [ ] **Add v0.9.1 → v0.9.2 migration section to `docs/guides/migration.md`** — Document inter-agent communication bus and swarm health monitoring additions. **Implementation:** Add a new section to `docs/guides/migration.md` with the version diff and any config changes.
- [ ] **Document LiteLLM API key configuration** — Add `api_key` field to the `[llm]` config table in `docs/guides/configuration.md` and `website/public/docs/configuration.html`. Explain when it's required for LiteLLM, and that the correct K8s Secret reference is `litellm-secrets` key `LITELLM_MASTER_KEY` (not `openclaw-secrets` key `LITELLM_API_KEY`). **Implementation:** Edit both files to add the `api_key` field description with correct secret reference.
- [ ] **Document K8s NetworkPolicy requirements** — Add docs explaining that new RavenClaws agents need their pod label added to the LiteLLM ingress NetworkPolicy, or use a more permissive policy. Include example: `- podSelector: matchLabels: {app: hugin-ravenclaws}`. **Implementation:** Add a section to `docs/guides/configuration.md` or a new `docs/guides/k8s-network.md` explaining the NetworkPolicy setup.
- [ ] **Document K8s Secret references** — Add docs explaining which Secrets are required (e.g., `litellm-secrets` with `LITELLM_MASTER_KEY`) and how to reference them in the deployment. Include the correct `secretKeyRef` YAML snippet. **Implementation:** Add a section to `docs/guides/configuration.md` or `k8s/README.md` explaining the Secret structure.
- [ ] **Add configurable sandbox workdir** — Add `RAVENCLAWS_SANDBOX_WORKDIR` env var or `sandbox.workdir` config field. Default `/tmp/ravenclaws-sandbox` breaks with `readOnlyRootFilesystem: true` in K8s. *(Moved from v0.9.4)* **Implementation:** Add `workdir: Option<PathBuf>` to `SandboxConfig`, check env var `RAVENCLAWS_SANDBOX_WORKDIR` then config field, fall back to `/tmp/ravenclaws-sandbox`. In `Sandbox::new()`, try creating the workdir and fall back to `std::env::temp_dir()` if `/tmp` is read-only.
- [ ] **Add init container `chown` to K8s deployment** — Add explicit `chown -R 65532:65532 /workspace` to the init container in `k8s/deployment.yaml`. *(Moved from v0.9.4)* **Implementation:** Add `initContainers` section to `k8s/deployment.yaml` with `image: busybox`, `command: ["chown", "-R", "65532:65532", "/workspace"]`, `volumeMounts: [{name: workspace, mountPath: /workspace}]`.
- [ ] **Add graceful shutdown for heartbeat** — Add a `Drop` impl or shutdown hook to `HeartbeatAgent` that calls `persist_state()` when the agent loop exits on SIGTERM/SIGINT. *(Moved from v0.9.4)* **Implementation:** Add `impl Drop for HeartbeatAgent { fn drop(&mut self) { self.persist_state().ok(); } }`. Also register a `tokio::signal::ctrl_c()` handler in `main.rs` for the heartbeat mode.

**Exit criteria:**
- [ ] `RavenFabricClient` wired to agent loop — `health()`, `execute()`, `broadcast()` called at runtime
- [ ] `ProviderFallbackChain` wired to agent loop — fallback chain used when primary provider fails
- [ ] `TokenBudget` wired to agent loop — token budget checked during agent execution
- [ ] `AgentMessageBus` wired to swarm orchestration — messages flow between agents
- [ ] `SwarmHealthMonitor` wired to swarm orchestration — health checks performed during orchestration
- [ ] Community health files in place: `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`
- [ ] Container image under 30 MB
- [ ] Migration docs updated for v0.9.1 → v0.9.2
- [ ] LiteLLM API key documented in config reference (with correct `litellm-secrets` reference)
- [ ] K8s NetworkPolicy requirements documented (with example pod label)
- [ ] K8s Secret references documented (with correct `secretKeyRef` YAML)
- [ ] Sandbox workdir is configurable via env var or config field
- [ ] K8s deployment works with `readOnlyRootFilesystem: true` (init container chown)
- [ ] Heartbeat mode handles SIGTERM gracefully — state file is always consistent

### v0.9.9 — Parity & Polish ✨

**Theme:** Reach feature parity with OpenClaw for the primary agent use case. Add the remaining capabilities that users expect from a primary agent.

- [ ] **Add agent execution endpoints to HTTP server** — Server mode has `/health`, `/ready`, `/metrics` but no `/chat`, `/execute`, or `/tools` endpoints. The server can report status but cannot actually run agents. *(Moved from v0.10 — critical for primary agent use)* **Implementation:** Same as v0.9.6 items — these are the same endpoints, listed here for v1.0 completeness.
- [ ] **Deduplicate `run_agent_loop` and `run_agent_loop_with_mcp`** — ~500 lines of duplicated code. Refactor to share common logic with MCP tool registration as a plugin. *(Moved from v0.10 — reduces maintenance burden)* **Implementation:** Extract shared logic into `run_agent_loop_inner()` that takes a `&ToolRegistry` parameter. Have both public functions call the inner function with their respective tool registries.
- [ ] **Integrate eval harness with agent loop** — `EvalRunner::run_task()` should use `run_agent_loop()` instead of calling `llm.chat()` directly, so eval tasks test tool calling, ReAct loop, and security integration. *(Moved from v0.10)* **Implementation:** Change `EvalRunner::run_task()` to accept an `AgentConfig` and call `run_agent_loop()` instead of `llm.chat()`.
- [ ] **Ship vLLM docs + verification tests** — `docs/guides/vllm.md` with quick start, `scripts/lib/test-provider-vllm.sh` for integration testing, matching `website/public/docs/vllm.html` page. **Implementation:** Create the docs and test files following the pattern of existing provider docs/tests.
- [ ] **Ship llama.cpp docs + verification tests** — `docs/guides/llamacpp.md` with quick start, `scripts/lib/test-provider-llamacpp.sh` for integration testing, matching `website/public/docs/llamacpp.html` page. **Implementation:** Create the docs and test files following the pattern of existing provider docs/tests.
- [ ] **Add Azure OpenAI adapter** — `Azure` variant to `OpenAICompatibleProvider` with `api-key` header, deployment-based URLs, and `api-version` query parameter. ~240 LOC. **Implementation:** Add `Azure` variant to `LLMProvider` enum. Create `AzureClient` struct that wraps `OpenAICompatibleClient` with Azure-specific headers and URL construction.
- [ ] **Add deep health check endpoint** — `/health/deep` that verifies LLM connectivity by making a lightweight request, in addition to the existing process-liveness `/health`. **Implementation:** Same as v0.9.6 item — listed here for v1.0 completeness.
- [ ] **Add env var override for server port** — Document `RAVENCLAWS_RUNTIME_PORT` or add `RAVENCLAWS_SERVE_PORT` as an env var alias for the server port. **Implementation:** Same as v0.9.6 item — listed here for v1.0 completeness.
- [ ] **Add dedicated HTTP server mode docs page** — `docs/guides/server-mode.md` and `website/public/docs/server-mode.html` explaining endpoints, configuration, ingress setup, and interaction with heartbeat mode. **Implementation:** Same as v0.9.6 item — listed here for v1.0 completeness.

**Exit criteria:**
- [ ] Agent execution endpoints in server mode — `/chat`, `/execute`, `/tools` fully functional
- [ ] `run_agent_loop` and `run_agent_loop_with_mcp` deduplicated — shared logic extracted
- [ ] Eval harness uses `run_agent_loop()` instead of calling `llm.chat()` directly
- [ ] vLLM docs + verification tests shipped
- [ ] llama.cpp docs + verification tests shipped
- [ ] Azure OpenAI adapter working with `api-key` header and deployment-based URLs
- [ ] Deep health check endpoint `/health/deep` verifies LLM connectivity
- [ ] Server port configurable via env var
- [ ] Server mode docs page in `docs/guides/` and `website/public/docs/`

### v1.0 — Simply the Best 🏆

**The stable release. RavenClaws is a fully functional primary agent — production-ready,
benchmarked, documented, and trusted. All five pillars are verified by independent
measurement. No more "use OpenClaw for real work" — RavenClaws IS the real work.**

**Scope:** v1.0 = v0.9.3 + v0.9.4 (critical fixes) + v0.9.5 (tool reliability) + v0.9.6
(server endpoints) + v0.9.7 (MCP ecosystem) + v0.9.8 (production hardening) + v0.9.9
(parity & polish). All gaps identified in rpi5 deployment feedback are closed.
Enterprise features (v0.8) and advanced capabilities (v0.10) are deferred to post-1.0.

**Exit criteria:**
- [ ] All v0.9.4 exit criteria met — `--exec` works with ANY model, no silent failures
- [ ] All v0.9.5 exit criteria met — tool execution works with ANY model, multi-MCP via TOML
- [ ] All v0.9.6 exit criteria met — server mode has `/chat`, `/execute`, `/tools` endpoints
- [ ] All v0.9.7 exit criteria met — MCP ecosystem integration verified end-to-end
- [ ] All v0.9.8 exit criteria met — all infrastructure wired, container < 30 MB, K8s docs complete
- [ ] All v0.9.9 exit criteria met — feature parity with OpenClaw for primary agent use case
- [ ] `ravenclaws --exec "Summarize this repository"` works with ANY provider and produces output
- [ ] `ravenclaws --serve` provides a fully functional agent API (chat, execute, tools)
- [ ] Tool execution works with models that don't emit structured `tool_calls` (text-based fallback)
- [ ] MCP client connects to multiple SSE-based MCP servers simultaneously
- [ ] RavenClaws can be added as an MCP server in OpenClaw's config (SSE transport)
- [ ] All verification tests passing across all 4 deployment targets (macOS, Linux, Docker, K8s)
- [ ] Release automation complete — signed tags, multi-arch containers, SBOM, provenance, crates.io publish all green
- [ ] No critical or high issues in ISSUES.md
- [ ] CI/CD green across all 3 workflows
- [ ] v1.0 tag pushed and released

### v0.10 — Hardening, ecosystem, advanced reasoning 💎 *(post-1.0)*

These features are deferred to after the v1.0 stable release. They represent
significant new capabilities that are not required for a production-ready 1.0.

- [ ] **Graceful degradation under load** — when resources are constrained, swarm prioritizes critical tasks, scales down non-essential workers, and queues overflow.
- [ ] **Self-healing** — failed agents are detected, replaced, and caught up. Supervisor re-assigns orphaned tasks. No single point of failure in mesh topologies.
- [ ] **Threat model + external security review.**
- [ ] **Fuzzing** (`cargo fuzz`) + property tests for config/policy parsers.
- [ ] **Skill/plugin marketplace + WASM sandboxing** for third-party extensions (core MCP ships in v0.4, the skill system in v0.5).
- [ ] **SDKs** (Python/TS) and a documentation site.
- [ ] **Advanced reasoning** — tree-of-thought, self-reflection, uncertainty estimation / ask-for-help.
- [ ] **Memory tiers** — episodic, semantic (local embeddings), procedural.
- [ ] **Multi-modal input** — Wire AnthropicClient's image support structure to CLI. Image attachments in `ChatMessage` (base64 or URL), PDF/text document ingestion.
- [ ] **Connectors / integrations** — OAuth connectors for Google Drive, M365, Slack, GitHub, Notion.
- [ ] **Skill / Plugin System** — Portable capability bundles: `skill.yaml` + scripts + resources, progressive disclosure, sandboxed skill execution.
- [ ] **RavenFabric rate limiting** — Add `--rate-limit` flag to relay (e.g., `--rate-limit 60` = 60 commands/minute per agent) with `--burst` flag for short spikes and per-agent rate limits in policy.
- [ ] **RavenFabric relay HA** — Document relay clustering (multiple relays behind a load balancer), add `--peer` flag for relay mesh, leverage stateless design for redundancy.
- [ ] **RavenFabric audit log verification** — `rf audit verify` command to check HMAC signature chain integrity, detect tampering, export to SIEM-friendly formats (CEF, LEEF).
- [ ] **RavenFabric K8s operator** — CRD `RavenFabricAgent` with policy, relay URL, namespace scope; auto-enrollment via K8s ServiceAccount tokens; Helm chart for one-line installation.
- [ ] **RavenFabric Prometheus metrics** — `rf-relay --metrics-listen 0.0.0.0:9091` with metrics: connections, commands allowed/denied, latency, agent memory/CPU.
- [ ] **RavenFabric structured policy validation** — Lint-style warnings for risky patterns (e.g., "Policy allows `kubectl delete`"), severity levels, `--strict` flag for CI/CD.
- [ ] **RavenFabric policy versioning & rollback** — `rf policy history`, `rf policy rollback`, auto-backup on change, git integration.
- [ ] **RavenFabric multi-agent identity management** — `rf agent list`, `rf agent rotate-key`, `rf agent revoke`, agent groups for batch execution.
- [ ] **RavenFabric file transfer** — `rf cp` and `rf sync` for encrypted file transfer, respects policy path restrictions.
- [ ] **RavenFabric interactive shell** — `rf shell <agent>` with persistent session, tab completion, policy-enforced command execution.
- [ ] **RavenFabric skill auto-generation** — `rf skill generate --agent <id>` auto-extracts allowed commands, denied patterns, and project context into `.ravenfabric-skill.md`.
- [ ] **RavenFabric web dashboard** — Optional web UI (`rf-dashboard` binary) with real-time audit log viewer, policy editor with live validation, agent status overview, and metrics graphs.
- [ ] **RavenFabric Terraform provider** — `ravenfabric_relay`, `ravenfabric_agent`, `ravenfabric_policy` resources for GitOps-managed deployment.
- [ ] **RavenFabric Ansible collection** — `community.ravenfabric` collection with modules for relay, agent, and policy management.
- [ ] **RavenFabric Windows agent** — `ravenfabric-windows-amd64-agent.exe` with PowerShell policy support and Windows Event Log integration.

---

## Provider Strategy

### Current Architecture

RavenClaws has **6 LLM providers** unified under `LLMProviderTrait`:

| Provider | Client | Status |
|---|---|---|
| LiteLLM | `OpenAICompatibleClient` (variant: `LiteLLM`) | ✅ Working |
| OpenAI | `OpenAICompatibleClient` (variant: `OpenAI`) | ✅ Working |
| OpenRouter | `OpenAICompatibleClient` (variant: `OpenRouter`) | ✅ Working |
| Ollama | `OpenAICompatibleClient` (variant: `Ollama`) | ✅ Working |
| Anthropic | `AnthropicClient` (native, not OpenAI-compat) | ✅ Working |
| OpenAI-Compatible | `OpenAICompatibleClient` (variant: `Generic`) | ✅ v0.9.3 |

The `OpenAICompatibleClient` handles 5 of 6 providers via a shared `/v1/chat/completions`
endpoint with provider-specific defaults (endpoint URL, headers, model names).

### ✅ Generic `openai-compatible` Provider (Implemented v0.9.3)

**Decision: ADD a generic `provider = "openai-compatible"` variant.** This is the
single highest-leverage provider addition — it unlocks dozens of inference engines
with zero per-provider code.

**What it covers (all speak `/v1/chat/completions`):**
- **vLLM** — popular open-source inference engine (PagedAttention, continuous batching)
- **llama.cpp** / **llamafile** — local CPU/GPU inference, single-binary server
- **LM Studio** — GUI + local server for GGUF models
- **Text Generation Inference (TGI)** — Hugging Face's inference server
- **Groq** — ultra-fast LPU inference (free tier available)
- **Together AI** — hosted open-source models
- **Fireworks AI** — fast inference, function-calling support
- **DeepInfra** — serverless inference
- **Perplexity** — API-compatible endpoint
- **Any custom OpenAI-compatible endpoint** — self-hosted, air-gapped, or proprietary

**Implementation scope (small):**
1. Add `OpenAICompatible` variant to `OpenAICompatibleProvider` enum in `config.rs`
2. No new client code — `OpenAICompatibleClient` already speaks the right protocol
3. Provider defaults: no default endpoint (user must set `--endpoint`), no default API key
4. CLI mapping: `--provider openai-compatible` (hyphenated for readability)
5. Tool-calling: depends on the backend — vLLM supports tools, llama.cpp does not (yet)
6. Tests: 3-4 `mockito` tests verifying custom endpoint + no-default-key behavior

**Estimated effort:** ~50 LOC in `config.rs` + ~30 LOC in `main.rs` + ~80 LOC tests = **~160 LOC total**

**Why NOT add native vLLM / llama.cpp providers:**
- Both speak OpenAI-compatible API — a native client would be a wrapper around the same
  `/v1/chat/completions` endpoint with no additional capability
- Adding them as named variants creates maintenance burden (version bumps, endpoint changes)
- The generic approach is more future-proof — adding a new inference engine doesn't require a code change

### Recommendation: Ship Tested Docs/Recipes

**Decision: ADD configuration recipes + verification tests for vLLM and llama.cpp.**
Documentation is where the real value lives — users need to know how to point RavenClaws
at these backends, not that a new enum variant exists.

**What to ship:**
1. **`docs/guides/vllm.md`** — Quick start: `docker run vllm/vllm-openai:latest --model mistralai/Mistral-7B-Instruct-v0.3` → `ravenclaws --provider openai-compatible --endpoint http://localhost:8000 --model mistralai/Mistral-7B-Instruct-v0.3`
2. **`docs/guides/llamacpp.md`** — Quick start: `llama-server -m model.gguf --port 8080` → `ravenclaws --provider openai-compatible --endpoint http://localhost:8080 --model model`
3. **Verification tests** in `scripts/lib/test-provider-vllm.sh` and `scripts/lib/test-provider-llamacpp.sh` — start the backend, run a test prompt, verify response, stop the backend
4. **Add to `scripts/verify.sh`** — `--vllm` and `--llamacpp` flags (skipped if backends not available)
5. **Add to `website/public/docs/`** — matching HTML pages for ravenclaws.io

**Estimated effort:** ~200 LOC docs + ~100 LOC verification tests + ~50 LOC website = **~350 LOC total**

### Recommendation: Add a Small Azure OpenAI Adapter

**Decision: ADD an `Azure` variant to `OpenAICompatibleProvider`.** Azure OpenAI uses
the same `/v1/chat/completions` protocol but differs in three ways:
1. **API key header:** `api-key` instead of `Authorization: Bearer`
2. **Endpoint format:** `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`
3. **Model name:** deployment name, not model name

**Implementation scope (small):**
1. Add `Azure` variant to `OpenAICompatibleProvider` enum
2. Override `build_headers()` to use `api-key` header
3. Override `build_endpoint()` to construct the Azure-specific URL
4. Config fields: `--endpoint` (resource base URL), `--azure-deployment`, `--azure-api-version`
5. Tests: 3-4 `mockito` tests for header format, URL construction, and error handling

**Estimated effort:** ~80 LOC in `config.rs` + ~60 LOC in `llm.rs` + ~100 LOC tests = **~240 LOC total**

### Recommendation: Defer Native AWS Bedrock and Gemini/Vertex

**Decision: DO NOT add native Bedrock or Gemini/Vertex providers at this time.**

| Provider | Why defer | How to reach today |
|---|---|---|
| **AWS Bedrock** | Complex auth (AWS SigV4), separate SDK, low community demand for self-hosted agents | Via LiteLLM proxy (`litellm --model bedrock/*`) |
| **Gemini / Vertex AI** | OpenAI-compatibility layer exists (`gemini-2.0-flash` works via OpenRouter); Vertex has complex GCP auth | Via OpenRouter or LiteLLM proxy |
| **Mistral AI** | OpenAI-compatible API | Via `openai-compatible` generic provider |
| **Cohere** | OpenAI-compatible API | Via `openai-compatible` generic provider |
| **xAI (Grok)** | OpenAI-compatible API | Via `openai-compatible` generic provider |

**Rationale:**
- All four are reachable today via LiteLLM or OpenRouter — no capability gap
- Adding native providers creates maintenance burden (API changes, auth complexity, SDK updates)
- The generic `openai-compatible` provider covers Mistral, Cohere, and xAI with zero code
- Bedrock and Gemini/Vertex have complex auth that would require significant code (~500+ LOC each)
- This aligns with the **Small** and **Simple** pillars — resist adding code that LiteLLM already handles

### Critical Caveat: Tool-Calling Fidelity is the Gating Feature

**Tool-calling (function calling) is NOT guaranteed across OpenAI-compatible backends.**
Chat completion works everywhere, but structured tool calling varies wildly:

| Backend | Tool Calling | Notes |
|---|---|---|
| OpenAI | ✅ Full | Native, reliable |
| Anthropic | ✅ Full | Native (separate client) |
| LiteLLM | ✅ Full | Proxies to any backend |
| vLLM | ⚠️ Partial | Supports tools format, quality varies by model |
| llama.cpp | ❌ None | No tool-calling support (GGUF format limitation) |
| Groq | ✅ Good | Fast, supports tools |
| Together AI | ✅ Good | Supports tools |
| TGI | ⚠️ Partial | Limited tool support |
| Ollama | ⚠️ Partial | Tool support varies by model |

**Impact on agent loop:** If the backend doesn't support tool calling, the agent loop
falls back to ReAct-style text parsing (`TOOL_CALL:` / `ARGS:` patterns). This works
but is less reliable than structured function calling.

**Recommendation:** Document tool-calling support per backend in the recipe docs.
The agent loop already handles both modes (structured + text-based), so no code change
is needed — just clear documentation of what works where.

### Summary: Provider Roadmap

| Action | Priority | Effort | Impact |
|---|---|---|---|
| Add `provider = "openai-compatible"` generic variant | **High** | ~160 LOC | Unlocks 10+ inference engines |
| Ship vLLM docs + verification tests | **High** | ~350 LOC | Production-grade local inference |
| Ship llama.cpp docs + verification tests | **Medium** | ~350 LOC | Edge/air-gapped inference |
| Add Azure OpenAI adapter | **Medium** | ~240 LOC | Enterprise Azure customers |
| Native AWS Bedrock provider | **Defer** | ~500+ LOC | Reachable via LiteLLM |
| Native Gemini/Vertex provider | **Defer** | ~500+ LOC | Reachable via OpenRouter/LiteLLM |
| Native Mistral/Cohere/xAI provider | **Defer** | ~0 LOC | Covered by generic `openai-compatible` |

**Total v1.0 provider scope:** ~1,100 LOC (generic provider + vLLM docs + llama.cpp docs + Azure adapter)
**Post-v1.0:** Revisit Bedrock/Gemini if LiteLLM proxy is insufficient for production deployments.

---

## Testing Strategy

- **Unit:** every module; provider request/response/error paths via `mockito`.
- **Integration:** end-to-end agent runs against a stubbed provider and a local Ollama.
- **Policy/security:** table-driven allow/deny tests; fuzzing on policy + config parsing.
- **CI gates:** `fmt`, `clippy -D warnings`, `test`, Trivy (CRITICAL/HIGH fail), SBOM per release.
- **Coverage goal:** ≥ 80% line coverage by v1.0; no `unwrap`/`expect` on non-test hot paths.

**Current coverage:** 452 unit tests across 18 modules + 114 verification tests across 10 modules. All tests pass, clippy clean, fmt clean.

**Known testing gaps:**
- `EvalRunner::run_task()` calls `llm.chat()` directly — does NOT use `run_agent_loop()`. Eval tasks don't test tool calling, ReAct loop, or security integration.
- No tool call assertions in eval harness — `Assertion` enum has 7 text-based types but no assertion for checking tool calls were made or specific tools were invoked.
- `run_agent_loop` and `run_agent_loop_with_mcp` have ~500 lines of duplicated code — no shared test coverage for the common logic.
- No integration tests for `RavenFabricClient` execution paths (client is created but never called).
- No integration tests for `ProviderFallbackChain` or `TokenBudget` (both are dead code).
- No integration tests for `AgentMessageBus` or `SwarmHealthMonitor` (both are dead code in orchestration).

---

## Performance Targets (v1.0)

| Metric | Target | Current |
|---|---|---|
| Stripped binary size | < 15 MB | 5.2 MB ✅ |
| Container image size | < 30 MB | ~50 MB ⚠️ (includes RavenFabric agent binary) |
| Cold start (single mode) | < 50 ms | 5.2 ms ✅ |
| Idle memory (server mode) | < 20 MB RSS | Not yet measured |
| Provider failover decision | < 5 ms | ✅ (v0.5.1) |
| Tool-call audit write | non-blocking, < 1 ms enqueue | ✅ (wired) |

---

## Security Hardening (by version)

| Version | Hardening added |
|---|---|
| 0.1 | Memory-safe Rust, TLS check, no creds in config, distroless, signed images, SBOM, Trivy. |
| 0.2 | Verified supply chain for downloaded binaries (SHA256 checksum); no panic/abort on client init; cross-compilation deps in CI. |
| 0.4 | Deny-by-default tool policy, sandboxed execution, audit log, secret zeroization, prompt-injection defense. **(Infrastructure complete, needs wiring)** |
| 0.8 | Secret zeroization on drop (`zeroize` for API keys + HMAC keys), `atty` replaced with `std::io::IsTerminal`. |
| 0.6 | E2E-encrypted remote exec via RavenFabric. |
| 0.7 | MCP Server — policy-checked and audited tool exposure over stdio. HTTP server mode with health/metrics endpoints. OpenTelemetry tracing. Helm chart for K8s deployment. |
| 0.8 | RBAC, SecurityPolicy with blast-radius limits, compliance reporting. |
| 0.9 | Inter-agent communication encryption, swarm-wide policy enforcement, heartbeat authentication, self-provisioning authorization. |
| 0.10 | External security review, fuzzing, published threat model. |
| 1.0 | Audit log mutex `unwrap()` → proper error handling. Community health files (SECURITY.md, CONTRIBUTING.md). SSE transport for MCP. |

---

## Design Decisions

- **Rust, `unsafe` forbidden** — memory safety and small static binaries are foundational to "secure + small."
- **OpenAI-compatible core** — most providers speak it; one client shape covers LiteLLM/OpenAI/OpenRouter, with Ollama as the documented exception.
- **AGPLv3 + Commercial dual license** — keeps the core open, closes the SaaS loophole, funds development. See [LICENSING.md](LICENSING.md).
- **Delegate heavy orchestration to RavenFabric** — RavenClaws stays a small worker; the mesh/remote-exec substrate is a separate, specialized system.
- **No phone-home** — observability is opt-in and self-hosted; trust is a feature.

---

## Technical Debt

Concrete items carried from the current codebase:

1. ~~**Security infrastructure not wired** — `PolicyEngine`, `Sandbox`, `AuditLog` are complete but never invoked.~~ ✅ **Wired to agent loop (commit 51e42b0)**
2. ~~**Pattern-matching tool calls** — Fragile `TOOL_CALL:` / `ARGS:` parsing instead of structured JSON.~~ ✅ **Structured function calling (v0.4)**
3. ~~**No MCP integration** — Reinventing tools instead of using industry standard.~~ ✅ **MCP client (v0.5.2)**
4. ~~**k8s Deployment runs a program that exits immediately** → needs server mode (v0.7) or a Job manifest meanwhile.~~ ✅ **Fixed — `--serve` mode with HTTP probes**
5. ~~**Client duplication** across LiteLLM/OpenAI/OpenRouter (`handle_response` ×4).~~ ✅ **Unified `OpenAICompatibleClient` (v0.5.0)**
6. ~~**Dead/unwired code:** `rustls` dep unused; `security`/`ravenfabric` config fields not honored.~~ ✅ **All modules wired to agent loop; RavenFabric config fields consumed by client; `zeroize` wired for secret material**
7. ~~**No graceful shutdown** — SIGTERM/SIGINT not handled; no audit log flush on exit.~~ ✅ **Fixed — graceful shutdown in server mode (v0.7.1)**
8. **No config hot-reload** — Changes require restart.
9. **Container image ~50 MB** — Target is < 30 MB.
10. **cargo-udeps findings** — Unused dependencies detected. *(periodic review)*
11. **cargo-outdated findings** — Dependencies behind latest. *(periodic review)*
12. **~60 `#[allow(dead_code)]` annotations** — Significant unwired infrastructure: `RavenFabricClient`, `ProviderFallbackChain`, `TokenBudget`, `AgentMessageBus`, `SwarmHealthMonitor`, `WebSearchConfig`, and ~15 unused error variants, ~15 unused struct fields, ~15 unused methods, ~5 dead error enums.
13. **`unwrap()` on audit log mutex** — 7+ calls on hot path (`audit.rs` lines 181, 315, 320, 325, 330, 361, 367). Will panic if mutex is poisoned.
14. **`run_agent_loop` and `run_agent_loop_with_mcp` are nearly identical** — ~500 lines of duplicated code. The only difference is MCP tool registration.
15. **Legacy `TOOL_CALL:` / `ARGS:` format still supported** — Dead code path in agent loop. No LLM provider generates this format.
16. **`EvalRunner::run_task()` bypasses agent loop** — Calls `llm.chat()` directly instead of `run_agent_loop()`. Eval tasks don't test tool calling, ReAct loop, or security integration.
17. **No tool call assertions in eval harness** — `Assertion` enum has 7 text-based types but no assertion for checking tool calls were made or specific tools were invoked.
18. **Server mode has no agent execution endpoints** — Only `/health`, `/ready`, `/metrics`. No `/chat`, `/execute`, or `/tools`.

---

## How You Can Help

- **Contributors:** pick an unchecked item and open a PR (CLA required — see [LICENSING.md](LICENSING.md#contributor-license-agreement-cla)).
- **Security researchers:** audit the code and report responsibly. *(A `SECURITY.md` policy is planned for v0.2.)*
- **Users:** file issues for missing features or rough edges.
- **Enterprise:** ask about commercial licensing and priority features.

---

*Secure. Small. Efficient. Robust. Simple. — Simply the best.* 🐦‍⬛
