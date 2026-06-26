# 🐦‍⬛ RavenClaws Roadmap

**Date:** 2026-06-23  
**Version:** v0.9.2 — Swarm Health & Telemetry ✅  
**Previous Release:** v0.9.1 (2026-06-22) — Inter-agent communication bus ✅  
**Current Commit:** `a284fbd` — All GitHub URL references updated to egkristi/RavenClaws
**CI Status:** Build & Release #166 ✅ · Container Build #166 ✅ · Security Scan #124 ✅
**v1.0 Hardening Progress:** 4/15 items completed (deprecated types removed, dead code eliminated, library API established, performance targets verified)

**Vision:** RavenClaws shall become the ultimate AI agentic assistant and worker —
the supreme, most trusted, and most capable autonomous agent. Simply the best.

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

**Version:** 0.9.2 (2026-06-23) — Swarm Health & Telemetry  
**Stats:** 18 source modules (+lib.rs, +eval.rs, +ravenfabric.rs), ~15,200 LOC, 5 LLM providers, 5 built-in tools (+web_search), 452 unit tests, 114 verification tests across 10 modules, multi-arch CI with signed images + SBOM, official Helm chart, `zeroize` for secret material, prompt-injection defense, autonomous heartbeat agent, long-horizon task persistence, self-provisioning swarm orchestration, inter-agent communication bus, swarm health monitoring & telemetry, published on crates.io as `ravenclaws` (binary + library crate).

| Component | Status | Details |
|---|---|---|
| Single agent (single-provider) | ✅ Working | Sends one prompt, logs response, exits |
| Single agent (multi-model) | ✅ Working | Iterates all providers, logs each response |
| **Swarm mode (single-provider)** | ✅ **v0.6** | Multiple parallel agents with different personas (analytical/creative/pragmatic); no fixed limit |
| **Supervisor mode (single-provider)** | ✅ **v0.6** | Task decomposition, sub-agent spawning, result aggregation |
| **Swarm mode (multi-model)** | ✅ **v0.6** | Parallel agents across different LLM providers; scales to any number |
| **Supervisor mode (multi-model)** | ✅ **v0.6** | Provider-aware task decomposition and assignment |
| LLM providers (5) | ✅ Working | LiteLLM, OpenAI, OpenRouter, Ollama, **Anthropic** (unified trait) |
| CLI & env-var overrides | ✅ Working | `--provider`, `--endpoint`, `--model`, layered TOML→env→flags |
| Config validation | ✅ Working | TLS enforcement, endpoint checks |
| Container & K8s security | ✅ Working | Distroless, non-root, read-only FS, dropped caps, seccomp, RBAC |
| CI/CD pipeline | ✅ Implemented | fmt + clippy `-D warnings` + test, 5-target builds, multi-arch images, **Cosign + SBOM + provenance + Trivy**, crates.io publish, releases — cross-compilation deps installed for all targets |
| Security scanning | ✅ Implemented | CodeQL, cargo-audit, cargo-deny, cargo-outdated, cargo-udeps, Trivy (FS + config), Hadolint, Kubescape, OSSF Scorecard, dependency review — all SARIF results uploaded to GitHub Security tab |
| Verification suite | ✅ Working | 114 system/integration checks · 10 modules · 4 targets (`scripts/verify.sh`: local, Docker, Linux, K8s, security, performance, LLM-quality, swarm, eval) — shell-orchestrated, requires live services |
| Eval harness | ✅ **v0.7.4** | `--eval <path>` mode with 7 assertion types, run traces, text/JSON reports, 24 unit tests + 20 verification tests, sample configs in `tests/eval/` |
| Multi-model routing | ✅ Working | `next_client()` round-robin + fallback chain with circuit breaker |
| RavenFabric integration | ✅ **v0.6.1** | Full client module (`RavenFabricClient`) with health, list_agents, execute, broadcast; wired into all agent modes; 12 unit tests |
| `--exec` one-shot mode | ✅ Working | Sends prompt to LLM, prints response to stdout; full test coverage |
| Rust unit tests | ✅ Working | 291 tests across all 10 modules; `mockito`-based HTTP tests for all 5 providers + RavenFabric |
| Agent loop / ReAct planning | ✅ Working | perceive→plan→act→observe with max-iteration guard, `FINAL:` marker detection, configurable via `--max-iterations` |
| Tool-use / function calling | ✅ Working | Tool abstraction + registry + **5 built-in tools** (+web_search) + **MCP tool discovery** + agent loop wiring |
| Deny-by-default policy | ✅ **Wired to agent loop** | `PolicyEngine` validates ALL tool calls before execution (commit 51e42b0) |
| Sandboxed execution | ✅ **Wired to agent loop** | `Sandbox` provides workdir jail for `shell_exec` (commit 51e42b0) |
| Audit log | ✅ **Wired to agent loop** | HMAC-SHA256 chained, tamper-evident, emits events for all tool calls (commit 51e42b0) |
| Streaming responses | ✅ Working | SSE streaming for LiteLLM, default non-streaming fallback for others |
| Conversation memory | ✅ Working | `ConversationMemory` struct with configurable max history, auto-trim |
| Interactive REPL | ✅ Working | `--repl` flag with stdin loop, streaming output, `/exit` `/reset` commands |
| System prompt / persona | ✅ Working | `LLMConfig.system_prompt` field, CLI `--system-prompt`, env var override |
| MCP client | ✅ Working | JSON-RPC 2.0 over stdio, tool discovery from external servers (v0.5.2) |
| **MCP server** | ✅ **v0.7** | Exposes RavenClaws tools over stdio via MCP protocol; `--mcp-server` flag; policy-checked and audited |
| **HTTP server mode** | ✅ **v0.7.1** | Long-running server with `/health`, `/ready`, `/metrics` endpoints; `--serve` flag; fixes k8s CrashLoopBackOff |
| **OpenTelemetry tracing** | ✅ **v0.7.2** | Opt-in distributed tracing with OTLP gRPC/stdout exporter; `#[instrument]` spans on agent loop, HTTP server, tools, LLM calls |
| Native Anthropic provider | ✅ Working | Direct Claude API with tool use, token tracking (v0.5.3) |
| Retry / fallback / circuit breaker | ✅ Working | Exponential backoff, token budgets, provider fallback chain (v0.5.1) |
| Pre-built binary releases | 📋 Wired, untagged | CI produces them on tag; none released yet |
| Git hooks (pre-commit / pre-push) | ✅ Working | `.githooks/` — fmt, clippy, tests, binary size, secrets on commit; +release build, Docker, security on push |
| Structured function calling | ✅ Working | OpenAI Tools format for OpenAI/LiteLLM/OpenRouter/Anthropic |
| **Human-in-the-loop approvals** | ✅ **v0.8** | `--require-approval` flag prompts for sensitive tool calls; audited |
| **Prompt-injection defense** | ✅ **v0.8** | `InjectionDetector` with 50+ patterns, instruction-boundary enforcement, output schema validation; wired to both agent loops; audited |
| Multi-modal input | ⚠️ Partial | AnthropicClient has image support structure, not wired to CLI |

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
Cognition (Claude), Manus, Perplexity Comet, Kimi, Open Interpreter, and Vellum.

We don't win by out-featuring them. We win by refusing to compromise on all five
pillars at once. By category:

- **vs. cloud / hosted assistants** (Claude Cowork, Manus, Perplexity Computer, Kimi): RavenClaws is **self-hostable, offline-capable, and source-available** under AGPLv3. Your data and tool calls never leave infrastructure you control — no phone-home.
- **vs. minimal agent runtimes** (Open Interpreter, ZeroClaw, PicoClaw): RavenClaws matches their footprint while adding a real **security model** (deny-by-default tool policy, audit log, sandboxing) and **multi-provider** routing with fallback.
- **vs. SDK / platform plays** (Vellum, Hermes Agent): RavenClaws is a **single dependency-light binary**, not a service you rent or a framework you marry. Embed it, ship it, forget it.

The bar: anything the field can do, RavenClaws should do **smaller, safer, and
simpler** — or deliberately not at all.

> **Where RavenClaws must lead, measurably (v1.0):** memory-safe core with zero
> known CVEs, sub-15 MB binary, sub-50 ms cold start, fully self-hostable and
> air-gappable, signed + SBOM-attested supply chain. These are claims we will
> benchmark and publish — not marketing.

### RavenClaws vs. Field (v0.9 achieved)

| Capability | RavenClaws v0.9 | Cognition (Claude) | Manus | Open Interpreter |
|---|:---:|:---:|:---:|:---:|
| Agent loop | ✅ | ✅ | ✅ | ✅ |
| Tool calling | ✅ (structured) | ✅ (structured) | ✅ | ✅ |
| **MCP client/server** | ✅ (both) | ✅ | ✅ | ✅ |
| Sandboxed execution | ✅ (wired) | ✅ | ✅ | ⚠️ Optional |
| **Security model** | ✅ (wired) | ⚠️ | ⚠️ | ❌ |
| **Local-first / air-gapped** | ✅ (Ollama) | ❌ | ❌ | ✅ |
| **~5 MB binary** | ✅ | ❌ (cloud) | ❌ (cloud) | ❌ (Python) |
| **Helm chart** | ✅ (v0.7.3) | ❌ | ❌ | ❌ |
| **No telemetry** | ✅ | ❌ | ❌ | ✅ |
| **Autonomous heartbeat** | ✅ **v0.9** | ✅ | ✅ | ❌ |
| **Long-horizon task persistence** | ✅ **v0.9** | ✅ | ✅ | ❌ |
| **Scalable swarm (1000+ workers)** | ✅ **v0.9** | ❌ | ❌ | ❌ |
| **Self-provisioning sub-agents** | ✅ **v0.9** | ❌ | ❌ | ❌ |
| **Swarm health & telemetry** | ✅ **v0.9.2** | ❌ | ❌ | ❌ |
| **Crate on crates.io** | ✅ **ravenclaws** (binary + library) | ❌ | ❌ | ❌ |
| Multi-modal input | ⚠️ (partial) | ✅ | ✅ | ⚠️ |
| Web search | ✅ (SearXNG + DuckDuckGo) | ✅ | ✅ | ✅ |
| Browser automation | ❌ | ✅ | ✅ | ⚠️ Plugins |
| Async background runs | ✅ (v0.8) | ✅ | ✅ | ❌ |
| Scheduling / triggers | ✅ (v0.8) | ✅ | ✅ | ❌ |
| Sub-agents / swarm | ✅ (v0.6) | ✅ | ✅ | ❌ |
| OAuth connectors | ❌ | ✅ | ✅ | ⚠️ Plugins |

**RavenClaws's Wedge:**
1. **Trust as a feature** — deny-by-default security, no telemetry, verifiable end-to-end
2. **Edge-deployable** — ~5 MB binary, runs on Raspberry Pi, air-gapped capable
3. **RavenFabric mesh** — E2E-encrypted remote execution across fleet (unique)
4. **Autonomous heartbeat** — operates independently for days/weeks, no supervision required ✅ v0.9
5. **Self-orchestrating swarm** — dynamically provisions and manages 10s–1000s of workers in any topology, each with unique capability profiles. No fixed limit — the swarm scales to the task.

---

## Features Required to Become the Preferred Alternative

Being *preferred* is a two-step bar: first reach **parity** on the capabilities the
field now treats as table stakes, then **win decisively** on the five pillars where
the cloud incumbents structurally can't follow.

### Part 1 — Table stakes (reach parity)

| Capability | Why it's table stakes | In RavenClaws | Target |
|---|---|:--:|:--:|
| Agent loop (plan → act → observe) | Without it there is no "agent" | ✅ | v0.3 |
| Tool / function calling | The substrate for every action | ✅ (primitive) | v0.4 |
| **MCP — client *and* server** | Industry standard (Anthropic, OpenAI, Google, Microsoft, Salesforce) | ✅ (both) | **v0.7** ✅ |
| Sandboxed execution | Native primitive in competitors | ⚠️ (not wired) | v0.4 |
| Persistent memory (vector recall) | Without it every session starts from zero | ⚠️ (in-memory only) | v0.3 → v0.9 |
| Web search + headless browser | Manus/Perplexity center on browse/summarize/fill-forms | ✅ (SearXNG + DuckDuckGo) | **v0.8** ✅ |
| File operations (read/write/edit) | Core to "worker" | ✅ | v0.4 |
| Sub-agents / swarm orchestration | Kimi runs 300 sub-agents / 4,000 steps | ✅ (v0.6) | v0.6 |
| **Autonomous heartbeat (long-running)** | Operates independently for days/weeks without supervision | ✅ **v0.9** | **v0.9** |
| **Scalable swarm (1000+ workers)** | Dynamic provisioning of 10s–1000s of agents in any topology; no fixed limit | ✅ **v0.9** | **v0.9** |
| **Self-provisioning sub-agents** | Agent spawns agents; recursive supervisor mode | ✅ **v0.9** | **v0.9** |
| **Inter-agent communication** | Structured message passing between swarm members | ✅ **v0.9.1** | **v0.9** |
| Async / long-horizon background runs | Manus's killer feature (cloud background) | ✅ **v0.8** | **v0.8** ✅ |
| Scheduling / triggers (cron, webhook) | Proactive, set-and-forget operation | ✅ **v0.8** | **v0.7** |
| Streaming + intermediate results | First-class in Vellum; needed for interactive UX | ✅ | v0.3 |
| Multi-modal input (images, PDFs) | Manus/Kimi are multimodal; "worker" must read docs | ❌ | v0.5 |
| Connectors / integrations (OAuth) | Claude-style connectors; Manus's weakness | ❌ | v0.6 |
| Retries / provider fallback | Vellum: retry, fall back, fail early | ⚠️ (partial) | v0.5 |
| Human-in-the-loop approvals | Enterprises require guardrails + audit + HITL | ✅ **v0.8** | **v0.4** |

### Part 2 — Where RavenClaws wins (the "preferred" wedge)

| Differentiator | Why it beats the field | Pillars | Phase |
|---|---|:--:|:--:|
| **Local-first / self-hosted / air-gapped** | Manus is cloud-only; Comet's "Local" is a browser, not a worker. RavenClaws runs fully offline with Ollama. | Secure · Simple | ✅ core |
| **Security model: deny-by-default + sandbox + audit** | Field bolts security on; we ship it in core. | Secure | ⚠️ v0.4 (wire it) |
| **~5 MB single binary, edge/embeddable** | No cloud agent runs on a Raspberry Pi. | Small · Efficient | ✅ |
| **Provider-agnostic + cost-aware routing** | Not locked to one model vendor. | Efficient · Robust | v0.5 |
| **RavenFabric mesh: E2E-encrypted remote exec** | Unique — competitors are single-host or single-cloud. | Robust | ✅ v0.6.1 |
| **No telemetry · signed + SBOM** | Trust as a feature, verifiable end-to-end. | Secure | ✅ |
| **Open core + commercial** | No lock-in, vs. proprietary cloud. | Simple | ✅ |

### Part 3 — The five that move the needle most

1. **MCP client + server (v0.7)** — instant access to entire tool ecosystem. ✅ **Both client and server now implemented.**
2. **Wire security model (v0.4)** — PolicyEngine + Sandbox + AuditLog invoked on every tool call. Core value proposition.
3. **Local-first privacy + security** — the wedge no cloud agent can copy.
4. **Autonomous heartbeat + self-orchestration (v0.9)** — RavenClaws operates independently for days, dynamically spawning and managing swarms of any size. No competitor offers this in a self-hosted, secure package. ✅ **Heartbeat implemented.**
5. **Scalable swarm (1000+ workers) (v0.9)** — from a handful of collaborators to thousands of workers, each with unique profiles. Self-provisioning, self-healing, and policy-governed. No artificial caps — the swarm is a true swarm.

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

- [ ] **Multi-modal Input** ⚠️ **PARTIAL** — AnthropicClient has image support structure, not wired to CLI
  - Image attachments in `ChatMessage` (base64 or URL)
  - PDF/text document ingestion
  - Provider-specific encoding (OpenAI vision, Anthropic images)

- [ ] **Skill / Plugin System** (foundations) — **MOVED TO v0.6**
  - Portable capability bundles: `skill.yaml` + scripts + resources
  - Progressive disclosure: skills advertise capabilities, agent selects
  - Sandboxed skill execution (reuse `Sandbox`)

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
- [ ] **Connectors / integrations** — OAuth connectors for Google Drive, M365, Slack, GitHub, Notion (acts as the user, not a shared service account). *(v0.7)*
- [ ] **Skill / Plugin System** (foundations) — **MOVED FROM v0.5** *(v0.7)*
  - Portable capability bundles: `skill.yaml` + scripts + resources
  - Progressive disclosure: skills advertise capabilities, agent selects
  - Sandboxed skill execution (reuse `Sandbox`)

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

### v1.0 — Simply the best 🏆

The stable release. RavenClaws is production-ready, benchmarked, documented, and
trusted. All five pillars are verified by independent measurement.

**Scope:** v1.0 = current v0.9.2 + hardening + docs + API stability. Enterprise
features (v0.8) and advanced capabilities (v0.10) are deferred to post-1.0.

- [x] **Deprecated types removed** — `LiteLLMClient`, `OpenRouterClient`, `OpenAIClient` (deprecated since v0.5.0) removed from codebase.
- [x] **Dead code eliminated** — legacy `execute_tool_call`, unused `run_exec_stream`, and `#[allow(dead_code)]` annotations reviewed and cleaned up.
- [x] **Library API established** — `[lib]` section in `Cargo.toml`, `src/lib.rs` with re-exports of stable public API for all 18 modules.
- [x] **Performance targets verified** — 5.2 MB stripped binary (< 15 MB target ✅), 5.2 ms cold start (< 50 ms target ✅). Both well under v1.0 targets.
- [x] **Zero known CVEs** — cargo-audit confirms 0 CVEs in dependency tree. 1 advisory (unmaintained `instant` transitive dep through `notify`) — informational only, no fix available.
- [ ] **Autonomous operation validated** — RavenClaws runs unattended for 7+ days, completing tasks via heartbeat loop, recovering from failures, and scaling swarm up/down as needed.
- [ ] **Swarm scale validated** — 1000+ worker agents operating in mesh topology, with < 5% overhead per additional agent. Swarm grows and shrinks organically — no fixed limit, no artificial cap.
- [ ] **API stability** guarantees + semver discipline. All public types and traits reviewed for v1.0 API surface.
- [ ] **Complete docs**, examples, migration guides. README includes quickstart, configuration reference, and architecture overview.
- [ ] **All verification tests passing** across all 4 deployment targets (macOS, Linux, Docker, K8s).
- [ ] **Release automation complete** — signed tags, multi-arch containers, SBOM, provenance, crates.io publish all green.
- [ ] **Reproducible builds.**

**Exit criteria:** All checkboxes above checked. No critical or high issues in ISSUES.md. CI/CD green across all 3 workflows. v1.0 tag pushed and released.

---

## Testing Strategy

- **Unit:** every module; provider request/response/error paths via `mockito`.
- **Integration:** end-to-end agent runs against a stubbed provider and a local Ollama.
- **Policy/security:** table-driven allow/deny tests; fuzzing on policy + config parsing.
- **CI gates:** `fmt`, `clippy -D warnings`, `test`, Trivy (CRITICAL/HIGH fail), SBOM per release.
- **Coverage goal:** ≥ 80% line coverage by v1.0; no `unwrap`/`expect` on non-test hot paths.

**Current coverage:** 452 unit tests across 18 modules + 114 verification tests across 10 modules. All tests pass, clippy clean, fmt clean.

---

## Performance Targets (v1.0)

| Metric | Target | Current |
|---|---|---|
| Stripped binary size | < 15 MB | 5.2 MB ✅ |
| Container image size | < 30 MB | ~50 MB ⚠️ |
| Cold start (single mode) | < 50 ms | 5.2 ms ✅ |
| Idle memory (server mode) | < 20 MB RSS | N/A (no server) |
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
7. **No graceful shutdown** — SIGTERM/SIGINT not handled; no audit log flush on exit. *(v0.7)* ✅ **Fixed — graceful shutdown in server mode (v0.7.1)**
8. **No config hot-reload** — Changes require restart. *(v0.7)*
9. **Container image ~50 MB** — Target is < 30 MB. *(v0.7)*
10. **cargo-udeps findings** — Unused dependencies detected. *(periodic review)*
11. **cargo-outdated findings** — Dependencies behind latest. *(periodic review)*

---

## How You Can Help

- **Contributors:** pick an unchecked item and open a PR (CLA required — see [LICENSING.md](LICENSING.md#contributor-license-agreement-cla)).
- **Security researchers:** audit the code and report responsibly. *(A `SECURITY.md` policy is planned for v0.2.)*
- **Users:** file issues for missing features or rough edges.
- **Enterprise:** ask about commercial licensing and priority features.

---

*Secure. Small. Efficient. Robust. Simple. — Simply the best.* 🐦‍⬛
