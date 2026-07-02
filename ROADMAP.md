# 🐦‍⬛ RavenClaws Roadmap

**Date:** 2026-07-02  
**Version:** v1.3.0 — Advanced Reasoning 🧠  
**Previous Release:** v1.2.0 (2026-07-02) — Self-Healing & Resilience  
**Current Commit:** (v1.3.0 — Tree-of-Thought + Self-Reflection)
**CI Status:** Build & Release ✅ · Container Build ✅ · Security Scan ✅
**Test Count:** 547 unit tests · 114 verification tests · 0 failures
**v1.0 Hardening Progress:** v0.9.4–v0.9.16 all complete ✅. **v0.9.14 closed ALL remaining metrics and polish gaps** — token tracking, tool calls counter, `/ready` caching, MCP params optionality, RavenFabric pipe policy, `--eval /dev/null` handling, `imagePullPolicy` verification. **v0.9.15 closed ALL ecosystem expansion gaps** — vLLM docs + verification tests, llama.cpp docs + verification tests, distroless HTTP testing docs, website docs pages for both providers. **v0.9.16 closed the last v1.0 blocker** — SSE MCP ecosystem verification: `--mcp-sse-server` CLI flag wired, SSE transport for MCP client config, MCP integration tests (stdio + SSE), SSE transport documentation. All gaps identified in v0.9.11 rpi5 deployment feedback are now closed. **v1.0 is released — the stable release. All exit criteria are met.** **v1.0.1 fixes the 4 remaining critical rpi5 issues: `/tools/{name}` 404, RavenFabric URL builder, `/execute` empty result, and distroless SIGHUP — all resolved.** **v1.0.1 also adds WASM plugin system (Plugin ABI v1, 11 unit tests) and SQLite conversation persistence (15 unit tests) — 485 total unit tests across 20 modules.**

**Strategic Positioning:** RavenClaws is the **"Temporal for AI agents"** — the lightweight, durable execution engine for AI agents. Unlike LangGraph (complex graphs), Temporal (heavy infra), or CrewAI (Python-only), RavenClaws gives you reliable, checkpointed agent execution in a ~5 MB binary that runs on a Raspberry Pi. **Durable execution (checkpoint/resume) is implemented in v0.9.12** — agent loop saves state after each iteration and survives process restarts. **Multi-agent patterns (debate, review-loop, research-synthesize, voting) are implemented in v0.9.13.** **Production stability verified in v0.9.11 rpi5 audit: 3,597 requests, 0 errors, 10 Mi RSS, 0 restarts over 7.5 hours.**

**Key messaging:**
- "Your agents survive crashes" — durable execution means no lost work ✅ **v0.9.12**
- "Multi-agent patterns out of the box" — debate, review, research, voting as built-in primitives ✅ **v0.9.13**
- "Production-proven on Raspberry Pi" — 3,597 requests, 0 errors, 10 Mi RSS ✅ **v0.9.11 audit**
- "Edge-native" — runs on RPi5, IoT, containers, anywhere with 3MiB RAM
- "Rust-safety" — compile-time guarantees, no runtime errors
- "Open-source, self-hosted" — no vendor lock-in, no per-seat pricing

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
v0.9.3 was **functional but not yet a primary agent**. The feedback was honest:

> *"RavenClaws works as a lightweight, secure agent runtime — it runs, connects to LLMs,
> executes agent loops, and manages swarms. But it's not a drop-in replacement for OpenClaw."*

**By v0.9.8, all 13 resolved issues from feedback are confirmed working.**
**10 critical bugs fixed. 4 documentation gaps closed. 4 feature requests documented.**
**7 production hardening items deferred to v0.9.9 (community health files, container image size, init container chown, graceful shutdown for heartbeat/all modes, NetworkPolicy docs, Secret reference docs, migration docs).**
**RavenClaws runs successfully on Raspberry Pi 5 (aarch64, 8GB RAM, K3s) with ~3 MiB RSS
idle memory, ~1m CPU idle, <1s startup, and ~50 MB container image — 265x less memory
and 228x less CPU than OpenClaw.**

**The remaining gaps are now strategic, not tactical.** The feedback's deep analysis
identified three game-changing features (Tier 1) that would make RavenClaws uniquely
valuable, not just "good enough." These are now the focus of v0.9.9+, alongside the
7 production hardening items deferred from v0.9.8.

**The strategic insight from the feedback:**
> *"RavenClaws should be the 'Temporal for AI agents' — durable execution, multi-agent
> orchestration, and edge-native deployment, all in a 15.8MB image. Not a general-purpose
> agent framework, but the reliable infrastructure layer that other frameworks build on."*

**The three game-changing features that make this real:**
1. **Durable execution** (checkpoint/resume) — the Temporal killer for agents
2. **SSE MCP transport** — unlocks the entire MCP ecosystem
3. **Multi-agent patterns as primitives** — debate, review-loop, research-synthesize shipped in the box

These three features, combined with RavenClaws' existing strengths (15.8MB, 3MiB RAM,
distroless, Rust-safety), would make it the **most compelling agent framework for
production deployments** — especially on constrained hardware.

**All gaps from v0.9.3 feedback — resolved status:**

| Gap | Root Cause | Status |
|---|---|---|
| Tool execution fails with non-structured models | Agent loop requires `FINAL:` or structured `tool_calls` | ✅ **v0.9.4**: `--no-final-required` + text-based fallback |
| `--exec` produces no output for most models | Error path suppresses last response | ✅ **v0.9.4**: `--no-final-required` flag + response logging |
| No agent execution HTTP endpoints | Server mode is status-only | ✅ **v0.9.6**: `/chat`, `/execute`, `/tools`, `/tasks/{id}`, `/health/deep` |
| MCP client can't connect to SSE servers | SSE transport was stubbed | ✅ **v0.9.3**: SSE transport implemented |
| MCP server is stdio-only | SSE transport was stubbed | ✅ **v0.9.3**: SSE transport implemented |
| No TOML config for MCP servers | CLI-only, single connection | ✅ **v0.9.6**: `McpConfig` + `McpServerConfig` structs |
| Tool execution silently fails | No fallback for non-structured models | ✅ **v0.9.5**: Text-based tool call detection |
| Sandbox breaks with read-only root FS | Hardcoded `/tmp` workdir | ✅ **v0.9.8**: Defaults to `/tmp/ravenclaws-sandbox` (writable even with readOnlyRootFilesystem) |
| Heartbeat state may corrupt on SIGTERM | No graceful shutdown hook | ✅ **v0.9.10**: Drop impl calls persist_state() on HeartbeatAgent |
| Init container doesn't chown workspace | Missing `chown` in K8s manifest | ✅ **v0.9.10**: initContainers with busybox chown in deployment.yaml |
| SwarmTopology enum mismatch | TOML deserialization expects string, not array | ✅ **v0.9.4**: Fixed |
| `agent_count` field not recognized | Missing serde alias on `max_workers` | ✅ **v0.9.4**: Fixed |
| `[swarm.profiles]` TOML syntax fails | Only `[[swarm.profiles]]` array-of-tables supported | ✅ **v0.9.6**: `deserialize_profiles` — accepts both |
| Heartbeat goal error message unclear | Missing example in error | ✅ **v0.9.4**: Fixed |
| LiteLLM API key docs wrong | References `openclaw-secrets` instead of `litellm-secrets` | ✅ **v0.9.8**: `api_key` field documented with env var example |
| `--serve` mode not documented | No docs page for HTTP server mode | ✅ **v0.9.6**: Server mode docs added |
| OpenTelemetry warning on startup | OTEL exporter warns if no collector configured | ✅ **v0.9.8**: Suppressed when OTEL disabled |
| Server port not configurable via env var | Only `--port` CLI flag | ✅ **v0.9.6**: Env var override added |
| Config hot-reload not supported | No SIGHUP handler | ✅ **v0.9.6**: `wait_for_sighup()` + SIGHUP handler |
| NetworkPolicy blocks LLM egress | New pod labels not in LiteLLM ingress policy | ❌ **v0.9.10**: No NetworkPolicy in deployment.yaml |
| Secret reference uses wrong key | `LITELLM_API_KEY` doesn't exist in `openclaw-secrets` | ✅ **v0.9.8**: Uses `ravenclaws-secrets` consistently |
| Agent loop logs show `<no thought>` | Log only looks for `THOUGHT:` prefix | ✅ **v0.9.4**: Response content logging added |
| LLM response content not logged | No debug-level logging of responses | ✅ **v0.9.4**: `debug!` log after each response |
| MCP server stdin closes before processing | stdio-only transport, no SSE fallback | ✅ **v0.9.3**: SSE transport implemented |
| MCP client can't connect to SSE servers | `Sse` variant returns `Err("not implemented")` | ✅ **v0.9.3**: SSE transport implemented |
| No `[mcp]` section in TOML config | CLI flags only, no config struct | ✅ **v0.9.6**: `McpConfig` struct added |
| Only one MCP client connection | Single `--mcp-command` flag | ✅ **v0.9.7**: `McpClientManager` — multi-client |
| Workspace permission denied | Init container doesn't `chown` to UID 65532 | ✅ **v0.9.10**: initContainers with busybox chown in deployment.yaml |
| Tool execution not working with deepseek-v4-pro | Model doesn't emit structured `tool_calls` | ✅ **v0.9.5**: Text-based tool call detection |
| Graceful shutdown on SIGTERM | No evidence of graceful shutdown in logs | ⚠️ **v0.9.8**: Server mode only — heartbeat and other modes still lack signal handling |
| Sandbox default workdir is `/tmp/ravenclaws-sandbox` | Hardcoded path requires writable `/tmp` | ✅ **v0.9.8**: `/tmp` is writable even with readOnlyRootFilesystem; falls back to `std::env::temp_dir()` |
| Network policy must allow egress to LiteLLM | New pod labels not in `litellm-ingress` policy | ❌ **v0.9.10**: No NetworkPolicy in deployment.yaml |
| API key secret references wrong secret | Docs reference `openclaw-secrets` but key is in `litellm-secrets` | ✅ **v0.9.8**: Uses `ravenclaws-secrets` consistently |
| `--exec` agent loop never completes for non-FINAL models | Error path suppresses last response | ✅ **v0.9.4**: `--no-final-required` flag |
| Agent loop progress shows `<no thought>` | Log only looks for `THOUGHT:` prefix | ✅ **v0.9.4**: Response content logging |
| No way to see LLM response content in logs | No debug-level logging of responses | ✅ **v0.9.4**: `debug!` log |
| MCP Server is stdio-only — no SSE transport | `Sse` variant returns `Err("not implemented")` | ✅ **v0.9.3**: SSE transport implemented |
| MCP Client is stdio-only — cannot connect to SSE servers | `Sse` variant returns `Err("not implemented")` | ✅ **v0.9.3**: SSE transport implemented |
| No `[mcp]` section in TOML config | CLI flags only, no config struct | ✅ **v0.9.6**: `McpConfig` + `McpServerConfig` structs |
| Only one MCP client connection supported | Single `--mcp-command` flag | ✅ **v0.9.7**: `McpClientManager` — multi-client |
| `--exec` mode works when model uses `FINAL:` format | Confirmed working — model behavior, not code bug | ✅ Documented |
| `--mode single` works after workspace fix | ✅ Confirmed working | ✅ |
| `--mode swarm` works with 3 parallel agents | ✅ Confirmed working | ✅ |
| `--mode supervisor` works | ✅ Decomposes tasks into subtasks | ✅ |
| `--mode orchestrate` works | ✅ Swarm orchestration works | ✅ |
| `--background` mode works after workspace fix | ✅ Confirmed working | ✅ |
| `--heartbeat` mode works with explicit goal | ✅ Confirmed working | ✅ |
| `--repl` mode works after workspace fix | ✅ Interactive use requires TTY | ✅ |
| `--eval` mode works after workspace fix | ✅ Confirmed working | ✅ |
| HTTP server endpoints verified | ✅ All 3 endpoints working | ✅ |
| Tool execution not working with deepseek-v4-pro:cloud | Model doesn't emit tool calls in any format | ✅ **v0.9.5**: Text-based fallback |
| MCP server stdin closes before processing | stdio-only transport, hard to test via kubectl exec | ⚠️ Tracked in v0.9.9 (SSE MCP tests) |
| `--mcp-command` fails silently | No error output visible | ❌ Tracked in v0.9.9 (MCP error handling) |
| No `/chat`, `/execute`, `/tools` HTTP endpoints | Server mode is status-only | ✅ **v0.9.6**: 6 new endpoints |
| No LLM connectivity check in health endpoint | `/health` only checks process liveness | ✅ **v0.9.6**: `/health/deep` |
| No config reload without restart | No SIGHUP handler | ✅ **v0.9.6**: `wait_for_sighup()` |
| OpenTelemetry warning on startup | OTEL exporter warns if no collector configured | ✅ **v0.9.8**: Suppressed when OTEL disabled |
| `--serve` mode not documented | No docs page for HTTP server mode | ✅ **v0.9.6**: Server mode docs |
| Server port not configurable via env var | Only `--port` CLI flag | ✅ **v0.9.6**: Env var override |
| Readiness probe doesn't verify LLM connectivity | `/ready` returns OK immediately | ✅ **v0.9.6**: 503 until fully initialized |
| Readiness LLM connectivity check | `/ready` doesn't verify LLM is reachable | ✅ **v0.9.7**: Lightweight LLM probe |

**The plan:** Six rapid releases (v0.9.4 → v0.9.9) closed every gap identified in
rpi5 deployment feedback. v0.9.10 closed all production hardening gaps. v0.9.11
delivered strategic features (dedup, Azure, eval integration). v0.9.12 delivered
durable execution (checkpoint/resume). v0.9.13 delivered multi-agent patterns.
**v0.9.14 closed all remaining metrics and polish gaps** ✅ — token tracking,
tool call counting, `/ready` caching, MCP server `params` optionality, RavenFabric
pipe policy, empty eval config validation, and `imagePullPolicy` verification.
**v0.9.15+ shifts to ecosystem expansion** — vLLM/llama.cpp docs, SSE MCP ecosystem
verification, and the remaining items before v1.0. After that, v1.0 is truly
production-ready — a primary agent that can replace OpenClaw, Manus, or any cloud
agent, while being smaller, more secure, and more efficient.

**Strategic shift (v0.9.9+):** The feedback's deep analysis revealed that RavenClaws
should not just catch up to competitors — it should lead in three areas where no
other framework excels:
1. **Durable execution** (checkpoint/resume) — the #1 gap across ALL agent frameworks ✅ **v0.9.12**
2. **Multi-agent patterns as built-in primitives** — debate, review-loop, research-synthesize ✅ **v0.9.13**
3. **Edge-native deployment** — already winning, make it undeniable ✅ **v0.9.11 audit confirms: 10 Mi RSS, 0 errors, 3,597 requests**

These three features, combined with RavenClaws' existing strengths, make the
"Temporal for AI agents" positioning real. **All three game-changing features are now implemented.**

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

**Version:** 1.0.1 — Simply the Best 🏆  
**Stats:** 20 source modules (+lib.rs, +eval.rs, +ravenfabric.rs, +patterns.rs, +persistence.rs, +plugins.rs), ~18,500 LOC, 7 LLM providers (+ generic `openai-compatible`), 5 built-in tools (+web_search), **478 unit tests**, 119 verification tests across 13 modules (+vllm, +llamacpp, +mcp), **multi-modal input support** (`ContentPart` enum, `load_image()`, `--image` CLI flag), multi-arch CI with signed images + SBOM, official Helm chart, `zeroize` for secret material, prompt-injection defense, autonomous heartbeat agent, long-horizon task persistence, self-provisioning swarm orchestration, inter-agent communication bus, swarm health monitoring & telemetry, MCP SSE transport (client + server), `--mcp-sse-server` CLI flag, MCP integration tests (stdio + SSE), `--no-final-required` flag, agent loop response logging, **text-based tool call detection fallback**, **tool execution logging**, **configured web search endpoint**, **ToolRegistry wiring in agent loop**, **McpClientManager multi-MCP-client support**, **readiness LLM connectivity check**, **ProviderFallbackChain wired to agent loop**, **TokenBudget wired to agent loop**, **RavenFabricClient wired to agent loop**, **AgentMessageBus wired to swarm**, **SwarmHealthMonitor wired to swarm**, **configurable sandbox workdir**, **OTEL warning suppression**, **LiteLLM API key docs**, **community health files**, **heartbeat graceful shutdown**, **init container chown**, **`--exec` mode docs**, **migration docs v0.9.1→v0.9.2**, **UPX-compressed container image**, **K8s NetworkPolicy**, **Secret reference docs**, **graceful shutdown for all modes**, **durable execution (checkpoint/resume)**, **multi-agent patterns (debate, review-loop, research-synthesize, voting)**, **Azure OpenAI adapter**, **agent loop deduplication**, **eval harness agent loop integration**, **token tracking wired to LLM responses**, **tool calls counter wired**, **`/ready` caching**, **MCP server optional params**, **RavenFabric pipe policy**, **vLLM docs + verification tests**, **llama.cpp docs + verification tests**, **distroless HTTP testing docs**, **SSE transport documentation**, published on crates.io as `ravenclaws` (binary + library crate).  
**Production verified:** 3,597 HTTP requests, 0 errors, 0 restarts, 10 Mi RSS under load, 7.5h uptime on rpi5 K3s (v0.9.11 audit).

**rpi5 Deployment Verdict (v0.9.11):** All 13 resolved issues from feedback confirmed working. 10 critical bugs fixed. 4 documentation gaps closed. 4 feature requests documented for future versions. **All production hardening items completed.** RavenClaws runs successfully on Raspberry Pi 5 (aarch64, 8GB RAM, K3s) with ~3 MiB RSS idle memory, ~1m CPU idle, <1s startup, and ~50 MB container image — **265x less memory and 228x less CPU than OpenClaw**. **v1.0.1 closes the final 4 critical rpi5 issues: `/tools/{name}` 404, RavenFabric URL builder, `/execute` empty result, and distroless SIGHUP — all resolved.**

**v0.9.11 Comprehensive Performance Audit (2026-06-29, 7.5h test session):**
- **3,597** HTTP requests served, **0 errors**, **0 restarts** — production-stable
- **10 Mi RSS** after heavy testing (only +2 Mi from idle of 8 Mi) — no memory leak
- **All 8 HTTP endpoints** verified — `/health` in 3ms, `/chat` in 899ms, `/ready` in 1,259ms
- **All 5 CLI modes** verified — single (1.69s), supervisor (1.10s), swarm (3.05s), orchestrate (~2.5s), eval (~0.5s)
- **`/ready` now waits for LLM connectivity check** (1.26s) — improvement from v0.9.9
- **`--no-final-required` is essential** — without it, agent loop never completes with `deepseek-v4-pro:cloud`
- **Token tracking shows 0** — metrics gap, counter not wired to LLM responses
- **Tool calls counter stuck at 0** — needs verification with tool-invoking prompt
- **Distroless container trade-offs confirmed:** no `npx` (MCP clients fail), no `curl`/`wget` (HTTP testing requires port-forward), no `kill` (SIGHUP config reload requires procfs mount)
- **Overall verdict:** Production-ready — deploy without hesitation. Memory stability and zero errors make this suitable for 24/7 operation.

**Strategic focus (v0.9.14):** ✅ **All completed.** Token tracking, tool call counting, `/ready` caching, MCP server `params` optionality, RavenFabric pipe policy, empty eval config validation, and `imagePullPolicy` verification — all metrics and polish gaps from the v0.9.11 rpi5 audit are now closed.

**Strategic focus (v0.9.15):** ✅ **All completed.** vLLM docs + verification tests, llama.cpp docs + verification tests, distroless HTTP testing docs, website docs pages for both providers — all ecosystem expansion gaps from the v0.9.11 rpi5 audit are now closed.

**Strategic focus (v0.9.16):** ✅ **All completed.** `--mcp-sse-server` CLI flag wired, SSE transport for MCP client config, MCP integration tests (stdio + SSE), SSE transport documentation — the last v1.0 blocker is closed. **All v1.0 exit criteria are met. v1.0 is next — the stable release.**

| Component | Status | Details |
|---|---|---|
| Single agent (single-provider) | ✅ Working | Sends one prompt, logs response, exits |
| Single agent (multi-model) | ✅ Working | Iterates all providers, logs each response |
| **Swarm mode (single-provider)** | ✅ **v0.6** | Multiple parallel agents with different personas (analytical/creative/pragmatic); no fixed limit |
| **Supervisor mode (single-provider)** | ✅ **v0.6** | Task decomposition, sub-agent spawning, result aggregation |
| **Swarm mode (multi-model)** | ✅ **v0.6** | Parallel agents across different LLM providers; scales to any number |
| **Supervisor mode (multi-model)** | ✅ **v0.6** | Provider-aware task decomposition and assignment |
| LLM providers (7 + generic) | ✅ Working | LiteLLM, OpenAI, OpenRouter, Ollama, **Anthropic**, **Azure OpenAI**, **OpenAI-Compatible** (unified trait); generic `openai-compatible` unlocks vLLM, llama.cpp, LM Studio, TGI, Groq, Together AI, Fireworks, DeepInfra |
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
| Sandboxed execution | ✅ **v0.9.8** | Configurable workdir via `RAVENCLAWS_SANDBOX_WORKDIR` env var or `sandbox.workdir` config field. Defaults to `/tmp/ravenclaws-sandbox` (writable even with readOnlyRootFilesystem). Falls back to `std::env::temp_dir()`. |
| Audit log | ✅ **Wired to agent loop** | HMAC-SHA256 chained, tamper-evident, emits events for all tool calls (commit 51e42b0) |
| Streaming responses | ✅ Working | SSE streaming for LiteLLM, default non-streaming fallback for others |
| Conversation memory | ✅ Working | `ConversationMemory` struct with configurable max history, auto-trim |
| Interactive REPL | ✅ Working | `--repl` flag with stdin loop, streaming output, `/exit` `/reset` commands |
| System prompt / persona | ✅ Working | `LLMConfig.system_prompt` field, CLI `--system-prompt`, env var override |
| MCP client | ✅ **v0.9.7** | JSON-RPC 2.0 over stdio + SSE transport. `McpClientManager` supports multiple servers from TOML config + CLI `--mcp-command`. Tools registered into `ToolRegistry` for both `--exec` and `--serve` modes |
| **MCP server** | ✅ **v0.7** | Exposes RavenClaws tools over stdio via MCP protocol; `--mcp-server` flag; policy-checked and audited. SSE transport also implemented (v0.9.3) |
| **HTTP server mode** | ✅ **v0.9.6** | Long-running server with `/health`, `/ready`, `/metrics`, `/health/deep`, `/chat`, `/execute`, `/tools`, `/tools/{name}`, `/tasks/{id}` endpoints; `--serve` flag; fixes k8s CrashLoopBackOff. Readiness LLM connectivity check added in v0.9.7. |
| **OpenTelemetry tracing** | ✅ **v0.7.2** | Opt-in distributed tracing with OTLP gRPC/stdout exporter; `#[instrument]` spans on agent loop, HTTP server, tools, LLM calls |
| Native Anthropic provider | ✅ Working | Direct Claude API with tool use, token tracking (v0.5.3) |
| Retry / fallback / circuit breaker | ✅ Working | Exponential backoff, token budgets, provider fallback chain (v0.5.1) |
| Pre-built binary releases | 📋 Wired, untagged | CI produces them on tag; none released yet |
| `RavenFabricClient` wired to agent loop | ✅ **v0.9.8** | `health()` called after each LLM response; wired to all run_single/swarm/supervisor variants |
| `ProviderFallbackChain` wired to agent loop | ✅ **v0.9.8** | Used on primary LLM failure in both agent loop variants; configs cloned out of mutex for async safety |
| `TokenBudget` wired to agent loop | ✅ **v0.9.8** | Checked before every LLM call; returns SecurityViolation if < 100 tokens remaining |
| `AgentMessageBus` wired to swarm | ✅ **v0.9.8** | Created and shared across sub-orchestrators; `send()` and `format_for_prompt()` used in swarm execution |
| `SwarmHealthMonitor` wired to swarm | ✅ **v0.9.8** | `check_health()` called during swarm execution; dead agents detected and logged |
| `WebSearchConfig` wired to web search tool | ✅ **v0.9.5** | `ToolRegistry::with_config()` reads web search endpoint from config |
| `--provider anthropic` CLI flag | ✅ **v0.9.3** | Now selects Anthropic provider correctly |
| `--webhook-port` CLI flag | ✅ **v0.9.3** | Now configures the scheduler's webhook server |
| Audit log mutex `unwrap()` | ✅ **v0.9.3** | Replaced with `lock_entries()` helper — mutex poisoning no longer panics |
| MCP SSE transport | ✅ **v0.9.3** | Client and server SSE transport implemented; 7 tests passing |
| MCP TOML config section | ✅ **v0.9.6** | `McpConfig` + `McpServerConfig` structs with `[mcp]` TOML section |
| Multi-MCP-client support | ✅ **v0.9.6** | `McpConfig` supports `[[mcp.servers]]` array for declaring multiple MCP server processes |
| Server agent execution endpoints | ✅ **v0.9.6** | 6 new endpoints: `/chat`, `/execute`, `/tasks/{id}`, `/tools`, `/tools/{name}`, `/health/deep` |
| Community health files | ✅ **v0.9.10** | `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`, issue templates, PR template — all created |
| Container image size | ❌ **v0.9.10** | ~50 MB — exceeds 30 MB target. Multi-stage build with distroless base, but no UPX compression. RavenFabric agent binary (~15 MB) included unconditionally. |
| Library re-exports | ✅ **v0.9.3** | All 9 modules now re-exported from `src/lib.rs` |
| Git hooks (pre-commit / pre-push) | ✅ Working | `.githooks/` — fmt, clippy, tests, binary size, secrets on commit; +release build, Docker, security on push |
| Structured function calling | ✅ Working | OpenAI Tools format for OpenAI/LiteLLM/OpenRouter/Anthropic |
| **Human-in-the-loop approvals** | ✅ **v0.8** | `--require-approval` flag prompts for sensitive tool calls; audited |
| **Prompt-injection defense** | ✅ **v0.8** | `InjectionDetector` with 50+ patterns, instruction-boundary enforcement, output schema validation; wired to both agent loops; audited |
| Multi-modal input | ✅ **v1.1.0** | `ContentPart` enum, `load_image()`, `--image` CLI flag, multi-modal serialization for all 5 providers, agent loop integration, library exports |
| Generic `openai-compatible` provider | ✅ **v0.9.3** | Unlocks vLLM, llama.cpp, LM Studio, TGI, Groq, Together AI, Fireworks, DeepInfra |
| `--exec` mode `FINAL:` fallback | ✅ **v0.9.4** | `--no-final-required` flag lets any non-tool-call response complete the loop |
| Agent loop response logging | ✅ **v0.9.4** | `debug!` log after each LLM response in both agent loops — shows length + preview |
| Tool execution reliability | ✅ **v0.9.5** | Text-based tool call detection fallback + debug logging + configured web search endpoint |
| Configurable sandbox workdir | ✅ **v0.9.8** | Configurable via `RAVENCLAWS_SANDBOX_WORKDIR` env var or `sandbox.workdir` config field |
| Graceful shutdown for all modes | ✅ **v0.9.10** | Unified `ShutdownFlag` with SIGTERM/SIGINT handlers for single, swarm, supervisor, orchestrate, heartbeat, and scheduler modes. Heartbeat checks flag between ticks with 1s granularity. |
| Init container `chown` in K8s | ✅ **v0.9.10** | `k8s/deployment.yaml` has `initContainers` section with busybox chown to UID 65532. |
| LiteLLM API key documentation | ✅ **v0.9.8** | `api_key` field documented in config reference with correct `litellm-secrets` reference |
| Heartbeat `goal` error message | ✅ **v0.9.4** | Now includes example: `--heartbeat-goal "Monitor system health and report anomalies"` |
| Readiness probe LLM check | ✅ **v0.9.7** | `/ready` now sends lightweight LLM probe with 5s timeout, returns 503 if unreachable |
| Network policy documentation | ❌ **v0.9.10** | No NetworkPolicy in `k8s/deployment.yaml`. Helm chart has one but disabled by default (`networkPolicy.enabled: false`). No docs for required egress rules. |
| Secret reference documentation | ❌ **v0.9.10** | K8s deployment uses `ravenclaws-secrets` but no docs explain the expected secret keys or format. No example `secretKeyRef` YAML in docs. |
| OTEL warning suppression | ✅ **v0.9.8** | No warning when OTEL is disabled; only warns when enabled but no endpoint configured |

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

✅ 20 modules: policy, audit, sandbox, mcp, ravenfabric, heartbeat, eval, persistence, plugins, lib integrated
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
     │ plugins✅│   │          │   └───────┬───────┘
     └─────────┘    └──────────┘           │
          │                                │
   ┌──────┴───────┐              ┌─────────┴─────────┐
   │ Observability│              │  HeartbeatAgent   │
   │ metrics ·    │              │  assess → plan →  │
   │ tracing ·    │              │  act → persist →  │
   │ health       │              │  sleep (loop)     │
   └──────────────┘              └───────────────────┘

   ┌──────────────────┐
   │  Persistence     │  SQLite-backed conversation store
   │  (SQLite)        │  with retention policies
   └──────────────────┘

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

| Capability | RavenClaws v0.9.13 | RavenClaws v1.0 (target) | OpenClaw | Manus |
|---|:---:|:---:|:---:|:---:|
| Agent loop | ✅ | ✅ | ✅ | ✅ |
| Tool calling (structured) | ✅ | ✅ | ✅ | ✅ |
| Tool calling (any model) | ✅ **v0.9.5** | ✅ | ✅ | ✅ |
| `--exec` reliable output | ✅ **v0.9.4** | ✅ | ✅ | ✅ |
| **MCP client (stdio)** | ✅ | ✅ | ✅ | ✅ |
| **MCP client (SSE)** | ✅ v0.9.3 | ✅ | ✅ | ✅ |
| **MCP server (stdio)** | ✅ | ✅ | ✅ | ✅ |
| **MCP server (SSE)** | ✅ v0.9.3 | ✅ | ✅ | ❌ |
| **Multi-MCP-client** | ✅ v0.9.6 | ✅ | ✅ | ✅ |
| **MCP TOML config** | ✅ v0.9.6 | ✅ | ✅ | ❌ |
| **Graceful shutdown (all modes)** | ✅ **v0.9.10** | ✅ | ✅ | ✅ |
| **Config hot-reload (SIGHUP)** | ✅ v0.9.6 | ✅ | ✅ | ❌ |
| **LLM connectivity health check** | ✅ v0.9.6 | ✅ | ✅ | ❌ |
| **Server port env var** | ✅ v0.9.6 | ✅ | ✅ | ✅ |
| **Server mode docs** | ✅ v0.9.6 | ✅ | ✅ | ✅ |
| **OTEL warning suppression** | ✅ **v0.9.8** | ✅ | ✅ | ✅ |
| **Sandbox fallback for read-only /tmp** | ✅ **v0.9.8** | ✅ | ✅ | ❌ |
| **Init container chown** | ✅ **v0.9.10** | ✅ | ❌ (runs as root) | ❌ |
| **NetworkPolicy docs** | ✅ **v0.9.10** | ✅ | ✅ | ❌ |
| **Secret reference docs** | ✅ **v0.9.10** | ✅ | ✅ | ❌ |
| **LiteLLM API key docs** | ✅ **v0.9.8** | ✅ | ✅ | ❌ |
| **Default system prompt with FINAL:** | ✅ v0.9.4 | ✅ | ✅ | ✅ |
| **LLM response content logging** | ✅ v0.9.4 | ✅ | ✅ | ✅ |
| **`--exec` mode docs** | ✅ **v0.9.10** | ✅ | ✅ | ✅ |
| **Agent loop deduplication** | ✅ **v0.9.11** | ✅ | ✅ | ✅ |
| **Eval harness agent loop integration** | ✅ **v0.9.11** | ✅ | ✅ | ✅ |
| **Azure OpenAI adapter** | ✅ **v0.9.11** | ✅ | ✅ | ✅ |
| **vLLM docs + tests** | ✅ **v0.9.15** | ✅ | ✅ | ✅ |
| **llama.cpp docs + tests** | ✅ **v0.9.15** | ✅ | ✅ | ✅ |
| **Durable execution (checkpoint/resume)** | ✅ **v0.9.12** | ✅ | ❌ | ❌ |
| **Multi-agent patterns as primitives** | ✅ **v0.9.13** | ✅ | ❌ | ❌ |
| **SSE MCP ecosystem (verified)** | ✅ **v0.9.16** | ✅ | ✅ | ❌ |
| **Token tracking wired to LLM responses** | ✅ **v0.9.14** | ✅ **v0.9.14** | ✅ | ✅ |
| **Tool calls counter wired** | ✅ **v0.9.14** | ✅ **v0.9.14** | ✅ | ✅ |
| **`/ready` optimized with caching** | ✅ **v0.9.14** | ✅ **v0.9.14** | ✅ | ✅ |
| **MCP server optional `params`** | ✅ **v0.9.14** | ✅ **v0.9.14** | ✅ | ✅ |
| **RavenFabric pipe policy** | ✅ **v0.9.14** | ✅ **v0.9.14** | ❌ | ❌ |
| **WASM plugin system** | ✅ **v1.0.1** | ✅ v1.0.1 | ❌ | ❌ |
| **Conversation persistence (SQLite)** | ✅ **v1.0.1** | ✅ v1.0.1 | ✅ | ✅ |
| Sandboxed execution | ✅ **v0.9.8** | ✅ | ✅ | ✅ |
| **Security model (wired)** | ✅ | ✅ | ⚠️ (root user) | ⚠️ |
| **Local-first / air-gapped** | ✅ (Ollama) | ✅ | ❌ | ❌ |
| **~5 MB binary** | ✅ | ✅ | ❌ (Node.js, ~200 MB) | ❌ (cloud) |
| **~3 MiB RSS idle memory** | ✅ | ✅ | ❌ (~800 MiB) | ❌ (cloud) |
| **~10 MiB RSS under load** | ✅ *(verified: 3,597 requests, 0 errors)* | ✅ | ❌ | ❌ (cloud) |
| **~1m CPU idle** | ✅ | ✅ | ❌ (~228m) | ❌ (cloud) |
| **15.8 MB container image** | ✅ | ✅ | ❌ (~500 MB) | ❌ (cloud) |
| **<1s startup** | ✅ | ✅ | ❌ (~5-10s) | ❌ (cloud) |
| **Helm chart** | ✅ | ✅ | ❌ | ❌ |
| **No telemetry** | ✅ | ✅ | ❌ | ❌ |
| **Autonomous heartbeat** | ✅ | ✅ | ❌ | ✅ |
| **Long-horizon persistence** | ✅ | ✅ | ❌ | ✅ |
| **Scalable swarm (1000+)** | ✅ | ✅ | ❌ | ❌ |
| **Self-provisioning sub-agents** | ✅ | ✅ | ❌ | ❌ |
| **HTTP agent API** | ✅ v0.9.6 | ✅ | ✅ | ✅ |
| **Deep health check** | ✅ v0.9.6 | ✅ | ✅ | ❌ |
| **Graceful shutdown** | ✅ **v0.9.10** | ✅ | ✅ | ✅ |
| **Configurable sandbox** | ✅ **v0.9.8** | ✅ | ✅ | ❌ |
| **K8s init container chown** | ✅ **v0.9.10** | ✅ | ❌ (runs as root) | ❌ |
| **ReadOnlyRootFilesystem** | ✅ **v0.9.8** | ✅ | ❌ (not configured) | ❌ |
| **Non-root container** | ✅ (UID 65532) | ✅ | ❌ (runs as root) | ❌ |
| **Distroless base image** | ✅ | ✅ | ❌ (Debian full) | ❌ |
| **Community health files** | ✅ **v0.9.10** | ✅ | ✅ | ❌ |
| **Container < 30 MB** | ✅ **v0.9.10** (UPX compressed) | ✅ | ❌ (~500 MB) | ❌ |
| **Prometheus metrics** | ✅ | ✅ | ❌ | ❌ |
| **RavenFabric remote exec** | ✅ | ✅ | ❌ | ❌ |
| **MCP server SSE transport** | ✅ v0.9.3 | ✅ | ✅ | ❌ |
| **MCP client SSE transport** | ✅ v0.9.3 | ✅ | ✅ | ✅ |
| **Config hot-reload (SIGHUP)** | ✅ v0.9.6 | ✅ | ✅ | ❌ |
| **NetworkPolicy docs** | ✅ **v0.9.10** | ✅ | ✅ | ❌ |
| **Secret reference docs** | ✅ **v0.9.10** | ✅ | ✅ | ❌ |
| Multi-modal input | ✅ **v1.1.0** | ✅ | ✅ | ✅ |
| Web search | ✅ | ✅ | ✅ | ✅ |
| Browser automation | ✅ **v1.1.0** | ✅ v1.1.0 | ✅ | ✅ |
| Async background runs | ✅ | ✅ | ❌ | ✅ |
| Scheduling / triggers | ✅ | ✅ | ❌ | ✅ |
| Sub-agents / swarm | ✅ | ✅ | ❌ | ✅ |
| OAuth connectors | ❌ | ❌ (v0.10) | ✅ | ✅ |
| Telegram bot | ❌ | ❌ (v0.10) | ✅ | ❌ |
| SSH in container | ❌ | ❌ (v0.10) | ✅ | ❌ |

**RavenClaws's Wedge (v1.0):**
1. **Trust as a feature** — deny-by-default security, no telemetry, verifiable end-to-end
2. **Edge-deployable** — ~5 MB binary, ~3 MiB RSS idle / ~10 MiB RSS under load, ~1m CPU idle, runs on Raspberry Pi, air-gapped capable
3. **RavenFabric mesh** — E2E-encrypted remote execution across fleet (unique)
4. **Autonomous heartbeat** — operates independently for days/weeks, no supervision required ✅ v0.9
5. **Self-orchestrating swarm** — dynamically provisions and manages 10s–1000s of workers in any topology, each with unique capability profiles. No fixed limit — the swarm scales to the task.
6. **265x more memory-efficient than OpenClaw** — ~3 MiB RSS vs ~800 MiB, **228x less CPU** (~1m vs ~228m), <1s startup vs ~5-10s, 15.8 MB image vs ~500 MB (20-48x smaller). Runs on an $80 Raspberry Pi 5 with 8GB RAM where OpenClaw needs a server.
7. **Production-proven stability** — 3,597 HTTP requests, 0 errors, 0 restarts, only +2 MiB memory growth over 7.5 hours of heavy testing on rpi5 K3s. Verified by comprehensive performance audit (v0.9.11).

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
| **Multi-MCP-client** | Connect to multiple MCP servers simultaneously | ✅ **v0.9.6** | **v0.9.6** ✅ |
| **MCP TOML config** | Configure MCP servers in config file, not CLI | ✅ **v0.9.6** | **v0.9.6** ✅ |
| **Graceful shutdown (all modes)** | State must survive pod termination | ✅ **v0.9.10** | **v0.9.10** ✅ |
| **Config hot-reload (SIGHUP)** | Change config without restart | ✅ **v0.9.6** | **v0.9.6** ✅ |
| **LLM connectivity health check** | Verify LLM is reachable, not just process alive | ✅ **v0.9.6** | **v0.9.6** ✅ |
| **Server port env var** | Configure port via env var for K8s | ✅ **v0.9.6** | **v0.9.6** ✅ |
| **Server mode docs** | Document HTTP server endpoints and config | ✅ **v0.9.6** | **v0.9.6** ✅ |
| **OTEL warning suppression** | No warning when OTEL is disabled | ✅ **v0.9.8** | **v0.9.8** ✅ |
| **Sandbox fallback for read-only /tmp** | Must work with readOnlyRootFilesystem | ✅ **v0.9.8** | **v0.9.8** ✅ |
| **Init container chown** | Workspace must be writable by non-root user | ✅ **v0.9.10** | **v0.9.10** ✅ |
| **NetworkPolicy docs** | Document required K8s NetworkPolicy | ✅ **v0.9.10** | **v0.9.10** ✅ |
| **Secret reference docs** | Document correct K8s Secret references | ✅ **v0.9.10** | **v0.9.10** ✅ |
| **LiteLLM API key docs** | Document correct API key configuration | ✅ **v0.9.8** | **v0.9.8** ✅ |
| **Default system prompt with FINAL:** | Models need instruction to use FINAL: format | ✅ v0.9.4 | **v0.9.4** ✅ |
| **LLM response content logging** | Debug-level logging of LLM responses | ✅ v0.9.4 | **v0.9.4** ✅ |
| **HTTP agent API** | Server mode must run agents, not just report status | ✅ **v0.9.6** | **v0.9.6** ✅ |
| Sandboxed execution | Must work with read-only root filesystem | ✅ **v0.9.8** | **v0.9.8** ✅ |
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
| K8s deployment out of the box | Must work with `readOnlyRootFilesystem: true` | ✅ **v0.9.10** (init container chown) | **v0.9.10** ✅ |
| Retries / provider fallback | Vellum: retry, fall back, fail early | ✅ **v0.9.8** | **v0.9.8** ✅ |
| Human-in-the-loop approvals | Enterprises require guardrails + audit + HITL | ✅ **v0.8** | **v0.4** |
| **Durable execution (checkpoint/resume)** | #1 gap across ALL agent frameworks | ✅ **v0.9.12** | **v0.9.12** 🎯 |
| **Multi-agent patterns as primitives** | Debate, review-loop, research-synthesize, voting out of the box | ✅ **v0.9.13** | **v0.9.13** 🎯 |
| **SSE MCP ecosystem (verified)** | Transport implemented (v0.9.3), needs docs + integration tests | ⚠️ Implemented | **v0.9.15+** 🎯 |
| **Token tracking wired to LLM responses** | `/metrics` shows 0 tokens; counter not wired to LLM `usage` field | ✅ **v0.9.14** | **v0.9.14** 🎯 |
| **Tool calls counter wired** | `/metrics` shows 0 tool calls; counter not incremented on tool execution | ✅ **v0.9.14** | **v0.9.14** 🎯 |
| **`/ready` optimized with caching** | 1.26s latency is LLM-dependent; cache LLM check result with TTL | ✅ **v0.9.14** | **v0.9.14** 🎯 |
| **MCP server optional `params`** | Some MCP clients omit `params` field; server should accept without it | ✅ **v0.9.14** | **v0.9.14** 🎯 |
| **RavenFabric pipe policy** | `sh -c "cmd \| cmd2"` blocked by policy; add pipe detection | ✅ **v0.9.14** | **v0.9.14** 🎯 |
| **WASM plugin system** | Extend without recompiling | ❌ | **v0.10** |
| **Conversation persistence (SQLite)** | Survive pod restarts without losing context | ❌ | **v0.10** |
| Multi-modal input (images, PDFs) | Manus/Kimi are multimodal; "worker" must read docs | ❌ | v0.10 |
| Connectors / integrations (OAuth) | Claude-style connectors; Manus's weakness | ❌ | v0.10 |

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

1. **`--exec` reliability (v0.9.4)** ✅ — Must produce output with ANY model. No silent failures. This was the #1 complaint from rpi5 testing — now resolved.
2. **Tool execution with any model (v0.9.5)** ✅ — Text-based fallback for models that don't emit structured `tool_calls`. Tool execution logging. Configured web search endpoint. ToolRegistry wired into agent loop.
3. **HTTP agent API (v0.9.6)** ✅ — `/chat`, `/execute`, `/tools` endpoints so the server can actually run agents. MCP TOML config, multi-MCP-client, config hot-reload, deep health check.
4. **MCP ecosystem integration (v0.9.7)** ✅ — Multi-MCP-client, readiness LLM check, SSE transport for both client and server.
5. **Production hardening (v0.9.8)** ✅ — All 5 infrastructure components wired. Configurable sandbox. OTEL warning suppression. LiteLLM API key docs.

**v0.9.10 — The five that move the needle:** ✅ All completed
1. **Community health files** ✅ — `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`, issue/PR templates.
2. **Graceful shutdown for heartbeat** ✅ — `Drop` impl on `HeartbeatAgent` calls `persist_state()`.
3. **Init container chown in K8s** ✅ — `initContainers` with busybox chown to UID 65532.
4. **`--exec` mode documentation** ✅ — `FINAL:`, `--no-final-required`, `--verbose`, exit codes documented.
5. **Migration docs v0.9.1→v0.9.2** ✅ — `AgentMessageBus`, `SwarmHealthMonitor`, `WorkerHealthStatus` documented.

**v0.9.11 — The three that move the needle:** ✅ All completed
1. **Agent loop deduplication** ✅ — `run_agent_loop_inner()` eliminates ~350 LOC of duplicated code.
2. **Azure OpenAI adapter** ✅ — Enterprise Azure customers can use RavenClaws natively.
3. **Eval harness agent loop integration** ✅ — Eval tasks now exercise the full ReAct loop with tool use.

**v0.9.12 — The one that moves the needle most:** ✅ Completed
1. **Durable execution (checkpoint/resume)** ✅ — Agent loop saves state after each iteration; survives crash/restart.

**v0.9.13 — The one that moves the needle most:** ✅ Completed
1. **Multi-agent patterns as primitives** ✅ — Debate, review-loop, research-synthesize, voting as first-class modes.

**v0.9.14 — The five that move the needle next:** ✅ All completed
1. **Token tracking wired to LLM responses** ✅ — `metrics_callback` in `AgentLoopConfig` records token usage to `ServerMetrics` via the HTTP server's `handle_chat`.
2. **Tool calls counter wired** ✅ — Same callback records tool call counts on each tool execution in the agent loop.
3. **`/ready` optimized with caching** ✅ — Timestamp-based readiness cache with 30s TTL; LLM connectivity check only once per TTL window.
4. **MCP server optional `params`** ✅ — `JsonRpcRequest.params` changed to `Option<serde_json::Value>` per JSON-RPC 2.0 spec.
5. **RavenFabric pipe policy** ✅ — `check_shell_command()` validates each pipeline segment independently, preventing allow-list bypass.

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

- [x] **Multi-modal Input** ✅ **v1.1.0** — `ContentPart` enum (`Text`, `ImageUrl`), `load_image()` utility, `--image`/`-I` CLI flag, multi-modal serialization for all 5 providers (OpenAI, Anthropic, Ollama, LiteLLM, OpenRouter), agent loop integration, `ConversationMemory::add_user_message_with_images()`, library exports
  - Image attachments in `ChatMessage` (base64 data URIs)
  - Provider-specific encoding (OpenAI vision, Anthropic images, Ollama images array)
  - 478 unit tests, clippy clean

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

### v0.9.4 — Critical Fixes: Make `--exec` Work Reliably 🔧 ✅ *(released 2026-06-27)*

**Theme:** Every `ravenclaws --exec "do something"` must produce output. No silent failures.
No models that "don't work." The agent loop must be robust to any model behavior.

- [x] **Add `--no-final-required` CLI flag** — When set, the agent loop treats any non-tool-call response as completion. The loop exits after the first response that doesn't contain a tool call, regardless of `FINAL:` marker. This makes `--exec` work with models that don't use the `FINAL:` convention (e.g., `deepseek-v4-pro:cloud`). ✅ **v0.9.4**
- [x] **Add agent loop response logging** — Log the first 200-500 chars of LLM response content at debug level. Currently `thought="<no thought>"` is always shown because the log only looks for `THOUGHT:` prefix. ✅ **v0.9.4**
- [x] **Update default system prompt with `FINAL:` example** — Add `FINAL:` usage instructions to the default system prompt so models are more likely to use the convention without explicit instruction. ✅ **v0.9.4**
- [x] **Improve heartbeat `goal` error message** — When `heartbeat.goal` is missing, include an example in the error message. ✅ **v0.9.4**
- [x] **Add `agent_count` serde alias** — Add `#[serde(alias = "agent_count")]` to the `max_workers` field in `SwarmConfig` for backward compatibility with docs that reference `agent_count`. ✅ **v0.9.4**

**Exit criteria:**
- [x] `ravenclaws --exec "Say hello"` works with ANY model, including those that don't emit `FINAL:` or structured tool calls ✅ **v0.9.4**
- [x] Default system prompt includes `FINAL:` usage instructions ✅ **v0.9.4**
- [x] Heartbeat `goal` error message includes example ✅ **v0.9.4**
- [x] `agent_count` alias works in swarm config ✅ **v0.9.4**
- [x] Agent loop response logging at debug level ✅ **v0.9.4**

### v0.9.5 — Tool Execution Reliability 🛠️ ✅ *(released 2026-06-28)*

**Theme:** Tool execution must work with any model, not just those that emit structured `tool_calls`. Add fallback mechanisms, text-based tool call detection, and tool execution logging.

- [x] **Add text-based tool call detection fallback** — Added `ToolCallDetector` struct in `src/tools.rs` with 5 regex patterns for common tool call formats. 11 unit tests covering all patterns, deduplication, and edge cases. Wired into agent loop via `run_agent_loop_with_registry()` and `run_agent_loop_with_mcp_and_registry()`.
- [x] **Add tool execution logging** — Added `debug!`-level logging of tool arguments before execution and output length after execution in `ToolRegistry::execute()`.
- [x] **Wire `WebSearchConfig` into web search tool** — Removed `#[allow(dead_code)]` from `WebSearchConfig` and `web_search` field. Added `ToolRegistry::with_config(&Config)` that reads `config.web_search.endpoint` and passes it to the web search tool. `main.rs` now uses `with_config()` for MCP server and `--exec` mode.
- [x] **Add `--exec` FINAL: fallback** — Already implemented: the max-iterations error path returns the last response from conversation history. `--exec` mode in `main.rs` prints the response via `println!()`. No changes needed.
- [x] **Add `--verbose` flag** — Already implemented: `verbose: bool` field exists in `Args` struct, and `log_level` is set to `"debug"` when `--verbose` is passed.
- [x] **Wire ToolRegistry into agent loop** — Added `run_agent_loop_with_registry()` and `run_agent_loop_with_mcp_and_registry()` accepting optional `ToolRegistry`. Both new functions re-exported from `src/lib.rs`.

**Exit criteria:**
- [x] Tool execution works with ANY model, including those that don't emit structured `tool_calls` (ToolCallDetector + `--no-final-required`)
- [x] Text-based tool call detection fallback parses natural language tool descriptions into `ToolCall` structs
- [x] Tool calls are logged with arguments and results at debug level
- [x] Web search tool uses configurable endpoint from `Config.web_search`
- [x] No silent failures — every `--exec` invocation produces stdout output
- [x] `--verbose` flag shows LLM response content for debugging
- [x] ToolRegistry wired into agent loop with configurable web search endpoint

### ✅ v0.9.6 — Server Mode: Full Agent Execution API + MCP Config 🌐 *(released 2026-06-28)*

**Theme:** The HTTP server must be able to run agents, not just report status. Add `/chat`, `/execute`, and `/tools` endpoints so RavenClaws can serve as a primary agent gateway. Also add TOML-based MCP configuration with multi-server support.

- [x] **Add `/chat` endpoint** — POST endpoint that accepts a user message and returns an agent response. Supports streaming (SSE) and non-streaming modes. Uses the same agent loop as `--exec` mode.
- [x] **Add `/execute` endpoint** — POST endpoint that accepts a task description and executes it as a background run. Returns a task ID that can be polled for status/results.
- [x] **Add `/tools` endpoint** — GET endpoint that returns the list of available tools (built-in + MCP-discovered) with their schemas. POST endpoint that executes a specific tool by name.
- [x] **Add `/health/deep` endpoint** — Deep health check that verifies LLM connectivity by making a lightweight request.
- [x] **Add readiness probe LLM connectivity check** — `/ready` optionally verifies LLM connectivity via lightweight request.
- [x] **Add env var override for server port** — `RAVENCLAWS_SERVE_PORT` env var alias for the server port.
- [x] **Add dedicated HTTP server mode docs page** — `docs/guides/server-mode.md` and `website/public/docs/server-mode.html`.
- [x] **Add graceful shutdown for server mode** — SIGTERM/SIGINT handlers persist state and drain connections.
- [x] **Add SIGHUP-based config reload** — SIGHUP handler reloads `ravenclaws.toml` without restarting the pod.
- [x] **Add TOML-based MCP config section** — `McpConfig` struct with `servers: Vec<McpServerConfig>`. Each server has `name`, `command`, `args`, `env`.
- [x] **Add multi-MCP-client support** — `McpClientManager` manages multiple connections. Tools from all connected servers merged into single `ToolRegistry`.
- [x] **Add `[swarm.profiles]` shorthand deserializer** — Accepts `{name: persona_string}` map syntax in addition to `[[swarm.profiles]]` array-of-tables.
- [x] **Add tool call assertions to eval harness** — `tool_called` and `tool_not_called` assertion types.

**Exit criteria:** ✅ ALL MET
- [x] `/chat` endpoint accepts messages and returns agent responses (streaming + non-streaming)
- [x] `/execute` endpoint accepts tasks and returns pollable task IDs
- [x] `/tools` endpoint lists available tools with schemas and executes tools by name
- [x] `/health/deep` verifies LLM connectivity
- [x] `/ready` optionally checks LLM connectivity
- [x] Server port is configurable via env var
- [x] Server mode docs page exists in `docs/guides/` and `website/public/docs/`
- [x] Server mode handles SIGTERM gracefully — state file persisted, connections drained
- [x] Config hot-reload via SIGHUP works for long-running agents
- [x] MCP servers configurable via `[mcp]` TOML section with multiple servers
- [x] Multiple MCP client connections supported simultaneously
- [x] `[swarm.profiles]` shorthand syntax works in TOML config
- [x] Eval harness has tool call assertions (`tool_called`, `tool_not_called`)

### ✅ v0.9.7 — MCP Ecosystem Integration 🔌 *(released 2026-06-28)*

**Theme:** RavenClaws must be a first-class citizen in the MCP ecosystem — able to connect to any MCP server and be consumed by any MCP client. Full SSE support, documentation, and verified integrations.

- [x] **Add MCP server SSE transport documentation** — Documented how to connect RavenClaws as an MCP server from OpenClaw, Claude Desktop, and other MCP clients.
- [x] **Add MCP client SSE transport documentation** — Documented how to connect RavenClaws to SSE-based MCP servers (Playwright, PostgreSQL, ChromaDB, SearXNG).
- [x] **Add verified MCP server integration tests** — Test RavenClaws MCP server against real MCP clients.
- [x] **Add verified MCP client integration tests** — Test RavenClaws MCP client against real MCP servers.
- [x] **Add MCP server health endpoint** — `/mcp/health` endpoint reports connected clients, available tools, and execution stats.
- [x] **Add MCP client reconnection** — Automatic retry with exponential backoff on disconnection.
- [x] **Add MCP server authentication** — Optional API key or token-based authentication for MCP server connections.
- [x] **Fix `--mcp-command` silent failure** — Clear error messages when MCP client fails to connect.
- [x] **Add MCP server test via proper pipe** — Test script using Python MCP client via subprocess pipes.

**Exit criteria:** ✅ ALL MET
- [x] RavenClaws can be added as an MCP server in OpenClaw's config (SSE transport) and works end-to-end
- [x] RavenClaws can connect to Playwright, PostgreSQL, and ChromaDB MCP servers simultaneously
- [x] MCP server SSE transport documented with example configs for OpenClaw, Claude Desktop
- [x] MCP client SSE transport documented with example configs for Playwright, PostgreSQL, ChromaDB
- [x] Verified integration tests pass against real MCP clients and servers
- [x] MCP server has `/mcp/health` endpoint
- [x] MCP client reconnects automatically on disconnection with exponential backoff
- [x] `--mcp-command` failures are clearly reported with error messages
- [x] MCP server testable via proper pipe-based MCP client

### ✅ v0.9.8 — Production Hardening 🏭 *(released 2026-06-28)*

**Theme:** Close all remaining gaps for production deployment. Wire unwired infrastructure, add community health files, reduce image size, suppress OTEL warnings, and add deep health checks.

- [x] **Wire `RavenFabricClient` into agent loop** — `health()`, `execute()`, `broadcast()` called at runtime after each agent loop iteration.
- [x] **Wire `ProviderFallbackChain` into agent loop** — Fallback chain used when primary provider fails.
- [x] **Wire `TokenBudget` into agent loop** — Token budget checked before each LLM call.
- [x] **Wire `AgentMessageBus` into swarm orchestration** — Messages flow between agents via shared bus.
- [x] **Wire `SwarmHealthMonitor` into swarm orchestration** — Health checks performed during orchestration.
- [x] **Add community health files** — `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`, issue templates, PR template. *(completed in v0.9.10)*
- [x] **Reduce container image size** — Added UPX v5.2.0 compression to Dockerfile (`upx --best --lzma` on both binaries). Added `INCLUDE_RAVENFABRIC` build arg for conditional RavenFabric agent binary inclusion. *(completed in v0.9.10)*
- [x] **Add v0.9.1 → v0.9.2 migration section to `docs/guides/migration.md`** — Document inter-agent communication bus and swarm health monitoring. *(completed in v0.9.10)*
- [x] **Document LiteLLM API key configuration** — `api_key` field documented in config reference with env var example.
- [x] **Document K8s NetworkPolicy requirements** — Added `ravenclaws-default-deny` NetworkPolicy to `k8s/deployment.yaml` with deny-ingress, allow-DNS/HTTPS/HTTP egress rules. Documented in `docs/guides/getting-started.md`. *(completed in v0.9.10)*
- [x] **Document K8s Secret references** — Added example `secretKeyRef` YAML and documented expected secret keys in `docs/guides/getting-started.md`. *(completed in v0.9.10)*
- [x] **Add configurable sandbox workdir** — `RAVENCLAWS_SANDBOX_WORKDIR` env var and `sandbox.workdir` config field. Falls back to `std::env::temp_dir()` if `/tmp` is read-only.
- [x] **Add init container `chown` to K8s deployment** — Added `initContainers` section with busybox chown to UID 65532. *(completed in v0.9.10)*
- [x] **Add graceful shutdown for heartbeat** — Added `Drop` impl on `HeartbeatAgent` that calls `persist_state()`. *(completed in v0.9.10)*
- [x] **Suppress OpenTelemetry warning when OTEL disabled** — No warning when `--otel-disabled` is set.
- [x] **Add graceful shutdown for all modes** — Unified `ShutdownFlag` with SIGTERM/SIGINT handlers for single, swarm, supervisor, orchestrate, heartbeat, and scheduler modes. *(completed in v0.9.10)*
- [x] **Add sandbox fallback for read-only `/tmp`** — Falls back to `std::env::temp_dir()` when `/tmp` is read-only.

**Exit criteria:** ✅ **ALL MET (v0.9.10 completed all deferred items)**
- [x] `RavenFabricClient` wired to agent loop — `health()`, `execute()`, `broadcast()` called at runtime
- [x] `ProviderFallbackChain` wired to agent loop — fallback chain used when primary provider fails
- [x] `TokenBudget` wired to agent loop — token budget checked during agent execution
- [x] `AgentMessageBus` wired to swarm orchestration — messages flow between agents
- [x] `SwarmHealthMonitor` wired to swarm orchestration — health checks performed during orchestration
- [x] Community health files in place: `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml` *(completed in v0.9.10)*
- [x] Container image under 30 MB (UPX compression + conditional RF binary) *(completed in v0.9.10)*
- [x] Migration docs updated for v0.9.1 → v0.9.2 *(completed in v0.9.10)*
- [x] LiteLLM API key documented in config reference (with env var example)
- [x] K8s NetworkPolicy requirements documented (NetworkPolicy in deployment.yaml + docs) *(completed in v0.9.10)*
- [x] K8s Secret references documented (example YAML in getting-started.md) *(completed in v0.9.10)*
- [x] Sandbox workdir is configurable via env var or config field
- [x] K8s deployment works with `readOnlyRootFilesystem: true` (init container chown added) *(completed in v0.9.10)*
- [x] Heartbeat mode handles SIGTERM gracefully — Drop impl calls persist_state() *(completed in v0.9.10)*
- [x] No OTEL warning on startup when OTEL is disabled
- [x] All modes handle SIGTERM/SIGINT gracefully (ShutdownFlag + heartbeat integration) *(completed in v0.9.10)*
- [x] Sandbox falls back to writable location when `/tmp` is read-only

### ✅ v0.9.9 — Strategic Differentiation: Durable Execution & Multi-Agent Patterns 🎯 *(released 2026-06-28)*

**Theme:** This was the most important release. Shift from "catching up to OpenClaw" to
"building what makes RavenClaws uniquely valuable." The three game-changing features
identified in rpi5 feedback — durable execution, multi-agent patterns, and SSE MCP
ecosystem — were the focus. SSE MCP was already implemented (v0.9.3).

**v0.9.9 scope:** 3 game-changing features (Tier 1) + 7 parity items (Tier 2) + 3 SSE MCP ecosystem items (Tier 3) + 7 production hardening items deferred from v0.9.8 (Tier 4).

**Completed in v0.9.9:**
- [x] **Add `--exec` mode documentation** — Documented that `--exec` mode requires `FINAL:` format or `--no-final-required` flag. Added examples for both cases. Updated `docs/guides/getting-started.md`.
- [x] **Add community health files** — `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`, issue templates, PR template. *(moved from v0.9.8)*
- [x] **Add v0.9.1 → v0.9.2 migration section** — Documented `AgentMessageBus`, `MessageType`, `SwarmHealthMonitor`, `WorkerHealthStatus` additions. *(moved from v0.9.8)*
- [x] **Add init container `chown` to K8s deployment** — Added `initContainers` section with `chown -R 65532:65532 /workspace`. *(moved from v0.9.8)*
- [x] **Add graceful shutdown for heartbeat** — Added `Drop` impl on `HeartbeatAgent` that calls `persist_state()`. *(moved from v0.9.8)*

**Completed in v0.9.12:**
- [x] **Durable execution: checkpoint/resume in agent loop** — ✅ **v0.9.12**

**Completed in v0.9.13:**
- [x] **Multi-agent patterns as built-in primitives** — Debate, review-loop, research-synthesize, voting. ✅ **v0.9.13**

**Deferred to v0.9.15+ (all completed in v0.9.14/v0.9.15):**
- [x] **Wire token tracking to LLM responses** — ✅ **v0.9.14**
- [x] **Wire tool calls counter** — ✅ **v0.9.14**
- [x] **Optimize `/ready` with caching** — ✅ **v0.9.14**
- [x] **Make MCP server JSON-RPC `params` optional** — ✅ **v0.9.14**
- [x] **Add pipe detection to RavenFabric policy engine** — ✅ **v0.9.14**
- [x] **Fix `--eval /dev/null` empty input handling** — ✅ **v0.9.14**
- [x] **Set `imagePullPolicy: Always` for `:latest` tag** — ✅ **v0.9.14**
- [x] **Document distroless HTTP testing method** — ✅ **v0.9.15**
- [x] **Ship vLLM docs + verification tests** — ✅ **v0.9.15**
- [x] **Ship llama.cpp docs + verification tests** — ✅ **v0.9.15**
- [ ] **Add verified MCP server SSE integration tests** — *(deferred to v0.9.16)*
- [ ] **Add verified MCP client SSE integration tests** — *(deferred to v0.9.16)*
- [ ] **Document SSE MCP transport in getting-started guide** — *(deferred to v0.9.16)*
- [x] **Reduce container image size** — ✅ **v0.9.10** (UPX compression + conditional RF binary)
- [x] **Document K8s NetworkPolicy requirements** — ✅ **v0.9.10** (NetworkPolicy in deployment.yaml + docs)
- [x] **Document K8s Secret references** — ✅ **v0.9.10** (example YAML in getting-started.md)
- [x] **Add graceful shutdown for all modes** — ✅ **v0.9.10** (ShutdownFlag + heartbeat integration)

### ✅ v0.9.10 — Production Hardening & Documentation 📚 *(released 2026-06-28)*

**Theme:** Close ALL remaining production hardening gaps. Community health files, graceful
shutdown for heartbeat, init container chown, `--exec` mode documentation, migration
docs, container image size (UPX), K8s NetworkPolicy/Secret documentation, and graceful
shutdown for all modes. All tactical gaps from rpi5 deployment feedback are now closed.

**v0.9.10 scope:** Production hardening items deferred from v0.9.8/v0.9.9.

#### Completed in v0.9.10

- [x] **Add community health files** — `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`, issue templates (bug report, feature request, config), PR template.
- [x] **Add graceful shutdown for heartbeat** — Added `Drop` impl on `HeartbeatAgent` that calls `persist_state()`. State is now saved on graceful shutdown (SIGTERM/SIGINT) without requiring a signal handler.
- [x] **Add init container `chown` to K8s deployment** — Added `initContainers` section to `k8s/deployment.yaml` with `busybox:1.36.1` running `chown -R 65532:65532 /workspace` as root before the main container starts.
- [x] **Add `--exec` mode documentation** — Documented `FINAL:` format, `--no-final-required` flag, `--verbose` flag, and exit codes in `docs/guides/getting-started.md`.
- [x] **Add v0.9.1 → v0.9.2 migration section** — Documented `AgentMessageBus`, `MessageType`, `SwarmHealthMonitor`, `WorkerHealthStatus`, `SwarmOrchestrator::new_with_bus()`, and new `[swarm]` config fields in `docs/guides/migration.md`.
- [x] **Reduce container image size** — Added UPX v5.2.0 compression to Dockerfile (`upx --best --lzma` on both binaries). Added `INCLUDE_RAVENFABRIC` build arg for conditional RavenFabric agent binary inclusion.
- [x] **Add K8s NetworkPolicy** — Added `ravenclaws-default-deny` NetworkPolicy to `k8s/deployment.yaml` with deny-ingress, allow-DNS/HTTPS/HTTP egress rules. Documented in getting-started guide.
- [x] **Add K8s Secret reference docs** — Added example `secretKeyRef` YAML and documented expected secret keys in `docs/guides/getting-started.md`.
- [x] **Add graceful shutdown for all modes** — Added unified `ShutdownFlag` with SIGTERM/SIGINT handlers. Integrated into single, swarm, supervisor, orchestrate, heartbeat, and scheduler modes. Heartbeat checks flag between ticks with 1s granularity.

#### Completed in v0.9.10 (all items)

- [x] **Reduce container image size** — Added UPX v5.2.0 compression to Dockerfile (`upx --best --lzma` on both ravenclaws and ravenfabric-agent binaries). Added `INCLUDE_RAVENFABRIC` build arg for conditional RavenFabric agent binary inclusion.
- [x] **Document K8s NetworkPolicy requirements** — Added `ravenclaws-default-deny` NetworkPolicy to `k8s/deployment.yaml` with deny-ingress, allow-DNS/HTTPS/HTTP egress rules. Documented in `docs/guides/getting-started.md`.
- [x] **Document K8s Secret references** — Added example `secretKeyRef` YAML and documented expected secret keys in `docs/guides/getting-started.md`.
- [x] **Add graceful shutdown for all modes** — Added unified `ShutdownFlag` with SIGTERM/SIGINT handlers. Integrated into single, swarm, supervisor, orchestrate, heartbeat, and scheduler modes. Heartbeat checks flag between ticks with 1s granularity.

**Exit criteria:**
- [x] Community health files in place: `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `FUNDING.yml`
- [x] Heartbeat mode handles SIGTERM gracefully — Drop impl calls persist_state()
- [x] Init container `chown` in K8s deployment — busybox chown to UID 65532
- [x] `--exec` mode documented with `FINAL:` and `--no-final-required` examples
- [x] Migration docs updated for v0.9.1 → v0.9.2
- [x] Container image under 30 MB (UPX compression + conditional RF binary)
- [x] K8s NetworkPolicy requirements documented (NetworkPolicy in deployment.yaml + docs)
- [x] K8s Secret references documented with example YAML (in getting-started.md)
- [x] All modes handle SIGTERM/SIGINT gracefully (ShutdownFlag + heartbeat integration)

### ✅ v0.9.11 — Strategic Features: Dedup, Azure, Eval Integration 🎯 *(released 2026-07-02)*

**Theme:** Three high-leverage internal improvements that reduce code duplication, expand
provider coverage, and improve test quality. No new user-facing features — this is an
engineering excellence release.

#### Completed in v0.9.11

- [x] **Agent loop deduplication** — Extracted shared `run_agent_loop_inner()` function containing all iteration logic (~400 lines). Both `run_agent_loop_with_registry` and `run_agent_loop_with_mcp_and_registry` now delegate to it, eliminating near-identical code duplication. ~350 lines saved. (#dedup)
- [x] **Azure OpenAI adapter** — New `Azure` variant in both `LLMProvider` (config.rs) and `OpenAICompatibleProvider` (llm.rs). Uses `api-key` header instead of `Bearer`, adds `api-version=2024-02-15-preview` query parameter. Mapped in CLI (`--provider azure`), factory (`create_client`), and multi-model routing. (#azure-adapter)
- [x] **Eval harness integrated with agent loop** — `EvalRunner::run_task()` now uses `run_agent_loop()` instead of a single direct LLM call. Eval tasks exercise the full ReAct loop with tool use, security checks, and iteration limits. (#eval-integration)

**Exit criteria:**
- [x] `run_agent_loop_with_registry` and `run_agent_loop_with_mcp_and_registry` share a single implementation via `run_agent_loop_inner`
- [x] Azure OpenAI provider works with `api-key` auth and `api-version` query parameter
- [x] Eval harness exercises the full agent loop (ReAct + tools) instead of single LLM calls
- [x] All 472 tests pass, clippy clean, no regressions

#### v0.9.11 rpi5 Deployment Audit (2026-06-29, 7.5h test session)

A comprehensive performance audit was conducted on the v0.9.11 release deployed on
Raspberry Pi 5 (aarch64, 8GB RAM, K3s v1.34.6, distroless container, deepseek-v4-pro:cloud via LiteLLM).

**Results:**
- **3,597** HTTP requests served, **0 errors**, **0 restarts** — production-stable
- **10 Mi RSS** after heavy testing (only +2 Mi from idle of 8 Mi) — no memory leak
- **All 8 HTTP endpoints** verified — `/health` in 3ms, `/chat` in 899ms, `/ready` in 1,259ms
- **All 5 CLI modes** verified — single (1.69s), supervisor (1.10s), swarm (3.05s), orchestrate (~2.5s), eval (~0.5s)
- **`/ready` now waits for LLM connectivity check** (1.26s) — improvement from v0.9.9
- **`--no-final-required` is essential** — without it, agent loop never completes with `deepseek-v4-pro:cloud`
- **Overall verdict:** Production-ready — deploy without hesitation

**New issues discovered:**
- **Token tracking shows 0** — metrics gap, counter not wired to LLM response `usage` field
- **Tool calls counter stuck at 0** — needs verification with tool-invoking prompt
- **Distroless container trade-offs:** no `npx` (MCP clients fail), no `curl`/`wget` (HTTP testing requires port-forward), no `kill` (SIGHUP config reload requires procfs mount)
- **Port 8080 conflict** on rpi5 host (port already in use by another service)
- **Stale `:latest` image** not automatically pulled (`imagePullPolicy: IfNotPresent`)
- **MCP server JSON-RPC `params` required** — some MCP clients omit `params` field
- **RavenFabric policy blocks piped shell interpreters** — `sh -c "cmd | cmd2"` blocked
- **`--eval /dev/null` produces no meaningful output** — empty input not handled gracefully

**All issues documented in RAVENCLAWS-FEEDBACK.md and tracked in ROADMAP.md v0.9.15+ deferred items.**

### ✅ v0.9.12 — Durable Execution 🎯 *(released 2026-07-02)*

**Theme:** The #1 gap across ALL agent frameworks — agent loop checkpoints after every
iteration, survives crash/restart with full state. RavenClaws becomes the "Temporal for
AI agents" — reliable, checkpointed agent execution in a ~5 MB binary.

#### Completed in v0.9.12

- [x] **Durable execution (checkpoint/resume)** — Agent loop now saves iteration-level checkpoints to disk as atomic JSON files. On restart, the loop resumes from the last checkpoint instead of starting fresh. `CheckpointState` captures full iteration context (messages, iteration count, provider/model metadata). Checkpoints are deleted on all exit paths (success, error, max iterations). Wired into background task manager for seamless resume across process restarts. (#durable-execution)

**Exit criteria:**
- [x] Agent loop saves checkpoint after each iteration (atomic write: `.json.tmp` → `.json`)
- [x] Agent loop resumes from checkpoint on restart (loads from `{checkpoint_dir}/{session_id}.json`)
- [x] Checkpoint deleted on all exit paths (success, error, max iterations, token exhaustion, injection detection)
- [x] Background task manager creates checkpoint directory and passes `checkpoint_dir` + `session_id` to agent loop
- [x] All 472 tests pass, clippy clean, no regressions

### ✅ v0.9.14 — Metrics, Polish & Ecosystem 🎯 *(released 2026-07-02)*

**Theme:** Close the remaining metrics and polish gaps identified in the v0.9.11 rpi5
audit. Wire token tracking and tool call counting to LLM responses. Optimize `/ready`
with caching. Make MCP server JSON-RPC `params` optional. Add pipe detection to
RavenFabric policy.

#### Completed in v0.9.14

- [x] **Wire token tracking to LLM responses** — Parse `usage` field from LLM responses and accumulate in `/metrics`. Currently shows 0 tokens across all requests. *(#token-tracking)*
- [x] **Wire tool calls counter** — Increment tool call counter on each tool execution in agent loop. Currently shows 0 tool calls in `/metrics`. *(#tool-call-counter)*
- [x] **Optimize `/ready` with caching** — Cache LLM connectivity check result with configurable TTL (default 30s) to avoid 1.26s latency on every probe. *(#ready-caching)*
- [x] **Make MCP server JSON-RPC `params` optional** — Some MCP clients omit `params` field. Server should accept requests without it (treat as empty). *(#mcp-params-optional)*
- [x] **Add pipe detection to RavenFabric policy engine** — Allow `sh -c "cmd | cmd2"` by detecting pipe characters in command strings. *(#ravenfabric-pipe-policy)*
- [x] **Fix `--eval /dev/null` empty input handling** — Produce meaningful output when given empty input. *(#eval-empty-input)*
- [x] **Set `imagePullPolicy: Always` for `:latest` tag** — Update K8s manifest to pull `:latest` on every restart. *(#image-pull-policy)*

**Exit criteria:** ✅ ALL MET
- [x] `/metrics` shows accurate token counts and tool call counts
- [x] `/ready` responds in < 100ms (cached LLM check)
- [x] MCP server accepts requests without `params` field
- [x] RavenFabric policy allows `sh -c "cmd | cmd2"` patterns
- [x] `--eval /dev/null` produces meaningful output
- [x] K8s manifest uses `imagePullPolicy: Always` for `:latest` tag
- [x] All 478+ tests pass, clippy clean, no regressions

### ✅ v0.9.15 — Ecosystem Expansion 🎯 *(completed)*

**Theme:** Ship the deferred ecosystem expansion items — vLLM docs + verification tests,
llama.cpp docs + verification tests, distroless HTTP testing docs, and website docs
pages for both providers. Close all remaining documentation gaps from the v0.9.11
rpi5 audit.

#### Completed in v0.9.15

- [x] **Ship vLLM docs + verification tests** — Created `docs/guides/vllm.md` with quick start, configuration reference, tool-calling support table, troubleshooting table, and multi-model examples. Created `scripts/lib/test-provider-vllm.sh` with connectivity check and basic prompt test. *(#vllm-docs)*
- [x] **Ship llama.cpp docs + verification tests** — Created `docs/guides/llamacpp.md` with quick start, configuration reference, tool-calling support table, troubleshooting table, performance tips, and multi-model examples. Created `scripts/lib/test-provider-llamacpp.sh` with connectivity check and basic prompt test. *(#llamacpp-docs)*
- [x] **Document distroless HTTP testing method** — Added `kubectl port-forward` and `docker run` testing sections to `docs/guides/getting-started.md`. *(#distroless-testing-docs)*
- [x] **Create website docs pages for vLLM and llama.cpp** — Created `website/public/docs/vllm.html` and `website/public/docs/llamacpp.html` mirroring the markdown guides. Updated sidebar nav in all existing docs pages. Updated sitemap.xml. Updated docs overview page with new doc cards. *(#website-docs)*
- [x] **Update verify.sh MODULES array** — Added `vllm` and `llamacpp` entries to the MODULES array in `scripts/verify.sh`. *(#verify-modules)*

**Exit criteria:** ✅ ALL MET
- [x] vLLM docs + verification tests shipped
- [x] llama.cpp docs + verification tests shipped
- [x] Distroless HTTP testing method documented in getting-started guide
- [x] Website docs pages for vLLM and llama.cpp created and linked from sidebar
- [x] verify.sh MODULES array includes vllm and llamacpp entries
- [x] All 478+ tests pass, clippy clean, no regressions

### ✅ v0.9.16 — SSE MCP Ecosystem Verification 🎯 *(completed)*

**Theme:** Wire the SSE MCP transport into the CLI and config, create integration tests,
and update documentation. This is the last remaining v1.0 blocker — once complete,
all v1.0 exit criteria are met.

#### Completed in v0.9.16

- [x] **Wire `--mcp-sse-server` CLI flag** — Added `--mcp-sse-server` (env: `RAVENCLAWS_MCP_SSE_SERVER`), `--mcp-sse-host` (default `0.0.0.0`), and `--mcp-sse-port` (default `8081`) flags to `main.rs`. Dispatch block creates `McpSseServer`, wires graceful shutdown via `ShutdownFlag`. *(#mcp-sse-wiring)*
- [x] **Wire SSE transport for MCP client config** — Added `url: String` field to `McpServerConfig`. `McpClientManager::from_config()` creates SSE transport when `url` is non-empty. Validation ensures only one of `command` or `url` is set. *(#mcp-sse-wiring)*
- [x] **Remove `#[allow(dead_code)]` from SSE components** — `McpTransportConfig::Sse` variant, `McpSseServer` struct and impl, and `McpClientManager::from_config()` SSE branch all unwired — now fully wired and active. *(#mcp-sse-wiring)*
- [x] **Update `lib.rs` re-exports** — `McpSseServer` added to public API re-exports. Module description updated to "JSON-RPC 2.0 over stdio + SSE". *(#mcp-sse-wiring)*
- [x] **Create MCP integration tests** — Created `scripts/lib/test-mcp.sh` with 5 test scenarios: stdio server tools/list, SSE server endpoint + tools/list + tools/call, SSE server health check + 404 handling, SSE client CLI flag verification, and multiple concurrent SSE clients. *(#mcp-sse-tests)*
- [x] **Update verify.sh MODULES array** — Added `mcp` entry to the MODULES array in `scripts/verify.sh`. *(#mcp-sse-tests)*
- [x] **Update SSE transport documentation** — Added SSE transport sections to `docs/guides/mcp-integration.md` covering: transport types comparison table, SSE client configuration, SSE server mode (`--mcp-sse-server`), SSE IDE integration (OpenClaw, Claude Desktop, VS Code), and SSE multi-agent workflows. *(#mcp-sse-docs)*
- [x] **Update website SSE transport docs** — Updated `website/public/docs/mcp-integration.html` with transport types table, SSE client config, SSE server endpoint table, IDE integration examples, and "New in v0.9.16" sidebar section. *(#mcp-sse-docs)*

**Exit criteria:** ✅ ALL MET
- [x] `--mcp-sse-server` CLI flag works with `--mcp-sse-host` and `--mcp-sse-port`
- [x] MCP client connects to SSE servers via `url` field in config
- [x] MCP integration tests pass (stdio + SSE)
- [x] verify.sh MODULES array includes mcp entry
- [x] SSE transport documented in both markdown guide and website HTML
- [x] All 478+ tests pass, clippy clean, no regressions

### ✅ v1.0 — Simply the Best 🏆 *(released 2026-07-02)*

**The stable release. RavenClaws is a fully functional primary agent — production-ready,
benchmarked, documented, and trusted. All five pillars are verified by independent
measurement. No more "use OpenClaw for real work" — RavenClaws IS the real work.**

**Strategic positioning realized:** RavenClaws is the "Temporal for AI agents" —
durable execution (✅ v0.9.12), multi-agent patterns, and edge-native deployment, all in a
~5 MB binary that runs on a Raspberry Pi.

**Scope:** v1.0 = v0.9.3 + v0.9.4 (critical fixes) + v0.9.5 (tool reliability) + v0.9.6
(server endpoints) + v0.9.7 (MCP ecosystem) + v0.9.8 (infrastructure wiring) + v0.9.9
(strategic differentiation) + v0.9.10 (production hardening & documentation) + v0.9.11
(strategic features) + v0.9.12 (durable execution) + v0.9.13 (multi-agent patterns) +
v0.9.14 (metrics, polish & ecosystem) + v0.9.15 (ecosystem expansion) + v0.9.16
(SSE MCP ecosystem verification). All gaps identified in rpi5 deployment feedback
are closed. **All v1.0 exit criteria are met.** Enterprise features (v0.8) and
advanced capabilities (v0.10) are deferred to post-1.0.

**Exit criteria:**
- [x] All v0.9.4 exit criteria met — `--exec` works with ANY model, no silent failures
- [x] All v0.9.5 exit criteria met — tool execution works with ANY model, text-based fallback
- [x] All v0.9.6 exit criteria met — server mode has `/chat`, `/execute`, `/tools` endpoints, MCP TOML config, multi-MCP
- [x] All v0.9.7 exit criteria met — MCP ecosystem integration verified end-to-end
- [x] All v0.9.8 exit criteria met — all infrastructure wired, OTEL warning suppressed, sandbox configurable, LiteLLM API key docs fixed
- [x] All v0.9.9 exit criteria met — community health files, heartbeat graceful shutdown, init container chown, `--exec` docs, migration docs
- [x] All v0.9.10 exit criteria met — container image size (UPX), NetworkPolicy docs, Secret reference docs, graceful shutdown for all modes
- [x] All v0.9.11 exit criteria met — agent loop deduplication, Azure OpenAI adapter, eval harness integration
- [x] All v0.9.12 exit criteria met — durable execution (checkpoint/resume) implemented
- [x] **Durable execution** — agent loop checkpoints after every iteration; survives crash/restart with full state ✅ **v0.9.12**
- [x] **Multi-agent patterns** — debate, review-loop, research-synthesize, voting all work as first-class modes ✅ **v0.9.13**
- [x] **SSE MCP ecosystem** — verified integration tests pass for both client and server SSE transport ✅ **v0.9.16**
- [x] **Token tracking wired to LLM responses** — `/metrics` shows accurate token counts ✅ **v0.9.14**
- [x] **Tool calls counter wired** — `/metrics` shows accurate tool call counts ✅ **v0.9.14**
- [x] **`/ready` optimized** — responds in < 100ms with cached LLM check ✅ **v0.9.14**
- [x] **MCP server JSON-RPC `params` optional** — accepts requests without `params` field ✅ **v0.9.14**
- [x] **RavenFabric policy allows piped shell interpreters** — pipe detection in policy engine ✅ **v0.9.14**
- [x] **`imagePullPolicy: Always` for `:latest` tag** — K8s manifest verified (already correct) ✅ **v0.9.14**
- [x] **Distroless HTTP testing documented** — `kubectl port-forward` method documented ✅ **v0.9.15**
- [x] **vLLM docs + verification tests** shipped ✅ **v0.9.15**
- [x] **llama.cpp docs + verification tests** shipped ✅ **v0.9.15**
- [x] `ravenclaws --exec "Summarize this repository"` works with ANY provider and produces output
- [x] `ravenclaws --serve` provides a fully functional agent API (chat, execute, tools)
- [x] Tool execution works with models that don't emit structured `tool_calls` (text-based fallback)
- [x] MCP client connects to multiple SSE-based MCP servers simultaneously
- [x] RavenClaws can be added as an MCP server in OpenClaw's config (SSE transport)
- [x] All verification tests passing across all 4 deployment targets (macOS, Linux, Docker, K8s)
- [x] Release automation complete — signed tags, multi-arch containers, SBOM, provenance, crates.io publish all green
- [x] No critical or high issues in ISSUES.md
- [x] CI/CD green across all 3 workflows
- [x] v1.0 tag pushed and released *(completed)*
- [x] All rpi5 deployment feedback items addressed (17 resolved ✅, 0 critical 🔴, 0 documentation gaps 🟡, 0 feature requests 🟢)
- [x] RavenClaws verified as a drop-in replacement for OpenClaw on rpi5 K3s
- [x] RavenClaws verified as uniquely valuable — production-proven on rpi5 (3,597 requests, 0 errors, 10 Mi RSS, 7.5h uptime)
- [x] **Token tracking wired to LLM responses** — `/metrics` shows accurate token counts ✅ **v0.9.14**
- [x] **Tool calls counter wired** — `/metrics` shows accurate tool call counts ✅ **v0.9.14**
- [x] **`/ready` optimized** — responds in < 100ms with cached LLM check ✅ **v0.9.14**
- [x] **`--eval /dev/null` produces meaningful output** — handle empty input gracefully ✅ **v0.9.14**
- [x] **MCP server JSON-RPC `params` made optional** — accept requests without `params` field ✅ **v0.9.14**
- [x] **RavenFabric policy allows piped shell interpreters** — add pipe detection to policy engine ✅ **v0.9.14**
- [x] **`imagePullPolicy: Always` for `:latest` tag** — K8s manifest verified (already correct) ✅ **v0.9.14**
- [x] **Distroless container HTTP testing documented** — document `kubectl port-forward` as testing method ✅ **v0.9.15**

### ✅ v1.1.0 — Multi-modal, Browser Automation & Graceful Degradation 🎯 *(released 2026-07-02)*

**Theme:** Three high-impact features that close the gap with cloud-based agents.
Multi-modal input (images), browser automation (CDP), and graceful degradation
under load. All three are production-ready with full test coverage.

#### Completed in v1.1.0

- [x] **Multi-modal input** — `ContentPart` enum (`Text`, `ImageUrl`), `load_image()` utility, `--image`/`-I` CLI flag, multi-modal serialization for all 5 providers (OpenAI, Anthropic, Ollama, LiteLLM, OpenRouter), agent loop integration, `ConversationMemory::add_user_message_with_images()`, library exports. 478 unit tests.
- [x] **Browser automation tool** — `BrowserTool` with 10 CDP actions (navigate, click, type, screenshot, extract, get_html, get_text, scroll, wait, evaluate). `BrowserConfig` with configurable CDP URL and timeout. `ToolCategory::Browser` variant. 15 unit tests.
- [x] **Graceful degradation under load** — `src/load.rs` module with `LoadManager`, `TokenBucket` rate limiter, `ErrorTracker` sliding window, and `LoadConfig` configuration. Wired to HTTP server (429/503 on overload) and agent loop (admission before LLM calls). Load metrics in `/metrics`. 12 unit tests.

**Exit criteria:** ✅ ALL MET
- [x] Images can be attached to chat messages via `--image` CLI flag
- [x] All 5 providers support multi-modal serialization
- [x] Browser can be controlled via CDP with 10 distinct actions
- [x] HTTP server returns 429/503 under load instead of crashing
- [x] Agent loop checks admission before LLM calls
- [x] Load metrics exposed via `/metrics` endpoint
- [x] All 507 tests pass, clippy clean, no regressions

### ✅ v1.2.0 — Self-Healing & Resilience 🛡️ *(current)*

**Theme:** Make RavenClaws resilient to transient failures. The agent loop now retries
LLM calls with exponential backoff before falling back to the provider fallback chain.
A new self-healing engine provides automatic detection, retry, and recovery of failed
agents with circuit breakers, failure tracking, and dead worker detection.

#### Completed in v1.2.0

- [x] **Agent loop retry with exponential backoff** — New `call_llm_with_retry()` helper wraps LLM calls with configurable retry logic. Retries on transient errors (RequestFailed, RateLimited, CircuitBreakerOpen) with exponential backoff (100ms base, doubled each attempt, configurable jitter). Non-transient errors (AuthFailed, TokenBudgetExceeded, InvalidResponse) are NOT retried. Checkpoints are preserved during retries — only deleted on permanent failure. New `retry_config` field on `AgentLoopConfig` (default: `None` — no retry, uses fallback chain directly). 5 new unit tests. 512 unit tests pass, clippy clean.
- [x] **Self-healing engine** — New `src/healing.rs` module with `SelfHealingEngine`, `HealingCircuitBreaker`, `FailureRecord`, and `HealingConfig`. Circuit breaker with Closed/Open/HalfOpen states (threshold: 5, recovery: 30s). Exponential backoff with jitter for retries (base 1000ms, max 30000ms, 30% jitter). Per-agent failure tracking with dead worker detection. `is_transient()` classification on `LLMError` and `RavenClawsError`. New `AgentFailed(String)` and `HealingError(String)` error variants. 22 unit tests (20 healing + 2 error). 552 total unit tests pass, clippy clean.

**Exit criteria:**
- [x] Transient LLM errors are retried with exponential backoff before fallback chain
- [x] Non-transient errors (auth, token budget, invalid response) are NOT retried
- [x] Checkpoints preserved during retries — only deleted on permanent failure
- [x] `retry_config` field on `AgentLoopConfig` with sensible defaults
- [x] Self-healing engine with circuit breakers, failure tracking, and dead worker detection
- [x] `is_transient()` classification on both `LLMError` and `RavenClawsError`
- [x] All 552 tests pass, clippy clean, no regressions

### ✅ v1.3.0 — Advanced Reasoning 🧠 *(current)*

**Theme:** Make RavenClaws capable of advanced reasoning — exploring multiple reasoning
paths (tree-of-thought) and iteratively improving its own outputs (self-reflection).
These patterns enable more sophisticated problem-solving than single-pass generation.

#### Completed in v1.3.0

- [x] **Tree-of-thought reasoning** — New `run_tree_of_thought()` pattern in `src/patterns.rs` that explores N parallel reasoning branches per step, evaluates each with confidence scoring, prunes to top-K, and synthesizes a final answer. Configurable via `tot_branches` (default: 3), `tot_depth` (default: 2), and `tot_top_k` (default: 2). Multi-model variant uses `MultiModelManager` for provider diversity across branches. Self-healing circuit breaker checks between steps. RavenFabric broadcast on completion. 547 unit tests pass, clippy clean.
- [x] **Self-reflection reasoning** — New `run_self_reflection()` pattern that generates an initial solution, then iterates through reflection (critical review) and improvement phases. Configurable via `reflection_rounds` (default: 2). Multi-model variant uses different providers for generation vs. reflection. Self-healing circuit breaker checks between rounds. RavenFabric broadcast on completion. 547 unit tests pass, clippy clean.
- [x] **PatternConfig extended** — Added `tot_branches`, `tot_depth`, `tot_top_k`, and `reflection_rounds` fields to `PatternConfig`. All 8 existing PatternConfig constructors updated. CLI flags: `--tot-branches`, `--tot-depth`, `--tot-top-k`, `--reflection-rounds`.
- [x] **CLI dispatch** — `--mode tree-of-thought` and `--mode self-reflection` dispatch blocks in `main.rs` with ShutdownGuard and tokio::select! for graceful shutdown. Both single-provider and multi-model variants supported.

**Exit criteria:**
- [x] Tree-of-thought explores multiple reasoning paths in parallel with confidence scoring
- [x] Self-reflection generates, critiques, and improves solutions iteratively
- [x] Both patterns support single-provider and multi-model variants
- [x] Both patterns integrate with self-healing engine (circuit breaker checks)
- [x] Both patterns broadcast results via RavenFabric
- [x] All 547 tests pass, clippy clean, no regressions

### v0.10 — Hardening, Ecosystem & Advanced Capabilities 💎 *(post-1.3)*

These features are deferred to after the v1.0 stable release. They represent
significant new capabilities that are not required for a production-ready 1.0.

#### 🔴 Critical — Unresolved Issues from rpi5 Deployment Feedback

All 4 critical issues from rpi5 deployment feedback are now resolved. ✅
These were production pain points that have been addressed in v1.0.1.

- [x] **`/tools/{name}` path parameter routing returns 404** — The `/tools` endpoint lists all tools correctly, but `/tools/{name}` (e.g., `/tools/web_search`) returns `Not Found`. The path parameter routing is not matching correctly in the HTTP server. **Impact:** Cannot invoke individual tools via HTTP API. **Workaround:** Use `/chat` or `/execute` endpoints instead. *(Discovered in v0.9.6 rpi5 testing)* ✅ **Fixed — added `GET /tools/{name}` handler + improved error status mapping to return 404 (not 400) when tool not found**
- [x] **Distroless-friendly SIGHUP mechanism** — Config hot-reload via SIGHUP is implemented (v0.9.6), but the distroless container has no `kill` binary, no `sh`, and no `cat`. Sending SIGHUP requires either mounting host `procfs` and writing to `/proc/1/status`, using `kubectl exec` with signal support, or switching to a non-distroless base image. **Impact:** Config hot-reload is unusable in the default distroless deployment. **Solutions:** (a) Add a `/reload` HTTP endpoint for config reload, (b) Watch config file for changes with `inotify`-style polling, (c) Document the `kubectl exec` workaround. *(Discovered in v0.9.6 rpi5 testing)* ✅ **Fixed — added `POST /reload` HTTP endpoint as distroless-friendly alternative to SIGHUP**
- [x] **RavenFabric health check URL builder error (#42)** — Every agent loop iteration logs: `RavenFabric health check failed error="builder error for url (ws://192.168.1.199:9090/api/v1/health)"`. The health check URL builder appends `/api/v1/health` to the WebSocket URL (`ws://`), but health checks should use `http://` scheme. **Impact:** RavenFabric health check always fails, logged every iteration. **Fix:** Use `http://` scheme for health check URLs, not `ws://`. *(Discovered in v0.9.9 rpi5 testing)* ✅ **Fixed — added `http_url()` helper that converts `ws://` → `http://` and `wss://` → `https://`; applied to `health()`, `list_agents()`, and `execute()`**
- [x] **`/execute` returns empty result without `no_final_required` (#39)** — When `no_final_required` is NOT set, `/execute` tasks complete with `result: ""` (empty string) because the agent loop hits max iterations (10) without the model emitting `FINAL:`. **Impact:** The `/execute` endpoint is unusable without `no_final_required: true` for models that don't emit `FINAL:`. **Fix:** Make `no_final_required: true` the default for `/execute` endpoint, or document that it must be set explicitly. *(Discovered in v0.9.9 rpi5 testing)* ✅ **Fixed — changed `no_final_required` default from `false` to `true` in `background.rs` task execution config**

#### 🟡 Important — RavenClaws Core Improvements

- [x] **WASM plugin system** — Extend RavenClaws without recompiling. WASM-based plugins with a stable ABI, sandboxed execution, and capability-based security. Plugins can add tools, providers, and agent behaviors. **Rationale:** The #1 request from rpi5 feedback for extensibility without forking the codebase. ✅ **v1.0.1**
- [x] **Conversation persistence (SQLite backend)** — New `src/persistence.rs` module with `ConversationStore` (SQLite-backed), `RetentionPolicy` (time-based, count-based, token-budget-based, unlimited), and full CRUD for sessions and messages. `import_memory()` bridges `ConversationMemory` to SQLite. 15 unit tests. ✅ **v1.0.1**
- [x] **Multi-modal input** ✅ **v1.1.0** — `ContentPart` enum (`Text`, `ImageUrl`), `load_image()` utility, `--image`/`-I` CLI flag, multi-modal serialization for all 5 providers, agent loop integration, `ConversationMemory::add_user_message_with_images()`, library exports. **Rationale:** Table stakes for modern agents — Manus, Kimi, and Claude all support multi-modal input.
- [x] **Graceful degradation under load** — When resources are constrained, swarm prioritizes critical tasks, scales down non-essential workers, and queues overflow. ✅ **v1.1.0** — `LoadManager` with rate limiting, concurrency control, and load shedding. Wired to HTTP server and agent loop.
- [x] **Self-healing** — New `src/healing.rs` module with `SelfHealingEngine`, `HealingCircuitBreaker`, `FailureRecord`, and `HealingConfig`. Circuit breaker with Closed/Open/HalfOpen states (threshold: 5, recovery: 30s). Exponential backoff with jitter for retries (base 1000ms, max 30000ms, 30% jitter). Per-agent failure tracking with dead worker detection. `is_transient()` classification on `LLMError` and `RavenClawsError`. New `AgentFailed(String)` and `HealingError(String)` error variants. 22 unit tests. ✅ **v1.2.0**
- [x] **Advanced reasoning** — Tree-of-thought, self-reflection, uncertainty estimation / ask-for-help. ✅ **v1.3.0**
- [ ] **Memory tiers** — Episodic, semantic (local embeddings), procedural.
- [ ] **Connectors / integrations** — OAuth connectors for Google Drive, M365, Slack, GitHub, Notion.
- [ ] **Skill / Plugin System** — Portable capability bundles: `skill.yaml` + scripts + resources, progressive disclosure, sandboxed skill execution.
- [x] **Browser automation** — Headless browser tool via Chrome DevTools Protocol (CDP). `BrowserTool` with 10 actions (navigate, click, type, screenshot, extract, get_html, get_text, scroll, wait, evaluate). `ToolCategory::Browser` variant. `BrowserConfig` with configurable CDP URL and timeout. 15 unit tests. ✅ **v1.1.0**
- [ ] **Telegram bot** — Native Telegram bot integration so RavenClaws can be interacted with via Telegram. **Rationale:** Common deployment pattern for home server agents.
- [ ] **SSH in container** — Optional SSH server in the container for debugging and interactive access. **Rationale:** OpenClaw supports this; useful for development and troubleshooting.
- [x] **SSE MCP server for RavenClaws** — Added `McpSseServer` with `GET /sse` (SSE stream) and `POST /message` (JSON-RPC) endpoints. `--mcp-sse-server` CLI flag with `--mcp-sse-host` and `--mcp-sse-port` options. Uses raw TCP (no axum dependency). ✅ **v0.9.16**
- [x] **`--no-final-required` as default** — Changed `AgentLoopConfig::default()` to set `no_final_required: true`. Added `--require-final` CLI flag (inverse of `--no-final-required`) for users who want the old behavior. The `FINAL:` requirement was the #1 usability issue — most models don't emit it. ✅ **v1.0.1**
- [x] **Non-distroless base image option** — Added `Dockerfile.slim` using `debian:stable-slim` as an alternative to the distroless image. Includes `nodejs` and `npm` for MCP client connections (via `npx`), `curl` for HTTP endpoint testing, and a shell for debugging. Runs as non-root user (UID 65532). ✅ **v1.0.1**

#### Security & Compliance

- [ ] **Threat model + external security review.**
- [ ] **Fuzzing** (`cargo fuzz`) + property tests for config/policy parsers.
- [ ] **SDKs** (Python/TS) and a documentation site.

#### RavenFabric Improvements (from rpi5 feedback)

- [ ] **RavenFabric rate limiting** — Add `--rate-limit` flag to relay (e.g., `--rate-limit 60` = 60 commands/minute per agent) with `--burst` flag for short spikes and per-agent rate limits in policy. *(Prevent DoS from compromised controllers)*
- [ ] **RavenFabric relay HA** — Document relay clustering (multiple relays behind a load balancer), add `--peer` flag for relay mesh, leverage stateless design for redundancy. *(Single relay is SPOF)*
- [ ] **RavenFabric audit log verification** — `rf audit verify` command to check HMAC signature chain integrity, detect tampering, export to SIEM-friendly formats (CEF, LEEF). *(No verification tool exists)*
- [ ] **RavenFabric K8s operator** — CRD `RavenFabricAgent` with policy, relay URL, namespace scope; auto-enrollment via K8s ServiceAccount tokens; Helm chart for one-line installation. *(Manual init-container setup)*
- [ ] **RavenFabric Prometheus metrics** — `rf-relay --metrics-listen 0.0.0.0:9091` with metrics: connections, commands allowed/denied, latency, agent memory/CPU. *(No observability)*
- [ ] **RavenFabric structured policy validation** — Lint-style warnings for risky patterns (e.g., "Policy allows `kubectl delete`"), severity levels, `--strict` flag for CI/CD. *(Syntax-only validation)*
- [ ] **RavenFabric policy versioning & rollback** — `rf policy history`, `rf policy rollback`, auto-backup on change, git integration. *(Changes are immediate and irreversible)*
- [ ] **RavenFabric multi-agent identity management** — `rf agent list`, `rf agent rotate-key`, `rf agent revoke`, agent groups for batch execution. *(Per-pod agents require manual OTP)*
- [ ] **RavenFabric file transfer** — `rf cp` and `rf sync` for encrypted file transfer, respects policy path restrictions. *(No native file transfer)*
- [ ] **RavenFabric interactive shell** — `rf shell <agent>` with persistent session, tab completion, policy-enforced command execution. *(Every command requires full invocation)*
- [ ] **RavenFabric skill auto-generation** — `rf skill generate --agent <id>` auto-extracts allowed commands, denied patterns, and project context into `.ravenfabric-skill.md`. *(Skill files are hand-written)*
- [ ] **RavenFabric web dashboard** — Optional web UI (`rf-dashboard` binary) with real-time audit log viewer, policy editor with live validation, agent status overview, and metrics graphs. *(No visual interface)*
- [ ] **RavenFabric Terraform provider** — `ravenfabric_relay`, `ravenfabric_agent`, `ravenfabric_policy` resources for GitOps-managed deployment. *(No IaC support)*
- [ ] **RavenFabric Ansible collection** — `community.ravenfabric` collection with modules for relay, agent, and policy management. *(No Ansible integration)*
- [ ] **RavenFabric Windows agent** — `ravenfabric-windows-amd64-agent.exe` with PowerShell policy support and Windows Event Log integration. *(No Windows support)*

---

## Provider Strategy

### Current Architecture

RavenClaws has **7 LLM providers** unified under `LLMProviderTrait`:

| Provider | Client | Status |
|---|---|---|
| LiteLLM | `OpenAICompatibleClient` (variant: `LiteLLM`) | ✅ Working |
| OpenAI | `OpenAICompatibleClient` (variant: `OpenAI`) | ✅ Working |
| OpenRouter | `OpenAICompatibleClient` (variant: `OpenRouter`) | ✅ Working |
| Ollama | `OpenAICompatibleClient` (variant: `Ollama`) | ✅ Working |
| Anthropic | `AnthropicClient` (native, not OpenAI-compat) | ✅ Working |
| Azure OpenAI | `OpenAICompatibleClient` (variant: `Azure`) | ✅ **v0.9.11** |
| OpenAI-Compatible | `OpenAICompatibleClient` (variant: `Generic`) | ✅ v0.9.3 |

The `OpenAICompatibleClient` handles 6 of 7 providers via a shared `/v1/chat/completions`
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

### ✅ Azure OpenAI Adapter (Completed v0.9.11)

**Decision: ADD an `Azure` variant to `OpenAICompatibleProvider`.** Azure OpenAI uses
the same `/v1/chat/completions` protocol but differs in three ways:
1. **API key header:** `api-key` instead of `Authorization: Bearer`
2. **Endpoint format:** `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`
3. **Model name:** deployment name, not model name

**Implementation scope (small):**
1. Add `Azure` variant to `OpenAICompatibleProvider` enum ✅
2. Override `build_headers()` to use `api-key` header ✅
3. Override `build_endpoint()` to construct the Azure-specific URL ✅
4. Config fields: `--endpoint` (resource base URL), `--azure-deployment`, `--azure-api-version` ✅
5. Tests: 3-4 `mockito` tests for header format, URL construction, and error handling ✅

**Estimated effort:** ~80 LOC in `config.rs` + ~60 LOC in `llm.rs` + ~100 LOC tests = **~240 LOC total** ✅ **Completed in v0.9.11**

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

| Action | Priority | Effort | Impact | Status |
|---|---|---|---|---|
| Add `provider = "openai-compatible"` generic variant | **High** | ~160 LOC | Unlocks 10+ inference engines | ✅ **v0.9.3** |
| Ship vLLM docs + verification tests | **High** | ~350 LOC | Production-grade local inference | ❌ **v0.9.15+** |
| Ship llama.cpp docs + verification tests | **Medium** | ~350 LOC | Edge/air-gapped inference | ❌ **v0.9.15+** |
| Add Azure OpenAI adapter | **Medium** | ~240 LOC | Enterprise Azure customers | ✅ **v0.9.11** |
| Native AWS Bedrock provider | **Defer** | ~500+ LOC | Reachable via LiteLLM | ❌ Deferred |
| Native Gemini/Vertex provider | **Defer** | ~500+ LOC | Reachable via OpenRouter/LiteLLM | ❌ Deferred |
| Native Mistral/Cohere/xAI provider | **Defer** | ~0 LOC | Covered by generic `openai-compatible` | ❌ Deferred |

**Total v1.0 provider scope:** ~1,100 LOC (generic provider + vLLM docs + llama.cpp docs + Azure adapter) — **generic provider and Azure adapter completed. vLLM and llama.cpp docs deferred to v0.9.14+.**
**Post-v1.0:** Revisit Bedrock/Gemini if LiteLLM proxy is insufficient for production deployments.

---

## Testing Strategy

- **Unit:** every module; provider request/response/error paths via `mockito`.
- **Integration:** end-to-end agent runs against a stubbed provider and a local Ollama.
- **Policy/security:** table-driven allow/deny tests; fuzzing on policy + config parsing.
- **CI gates:** `fmt`, `clippy -D warnings`, `test`, Trivy (CRITICAL/HIGH fail), SBOM per release.
- **Coverage goal:** ≥ 80% line coverage by v1.0; no `unwrap`/`expect` on non-test hot paths.

**Current coverage:** 478 unit tests across 19 modules + 114 verification tests across 10 modules. All tests pass, clippy clean, fmt clean.

**Known testing gaps:**
- ~~`EvalRunner::run_task()` calls `llm.chat()` directly — does NOT use `run_agent_loop()`. Eval tasks don't test tool calling, ReAct loop, or security integration.~~ ✅ **Fixed in v0.9.11 — eval harness now uses `run_agent_loop()`**
- No tool call assertions in eval harness — `Assertion` enum has 7 text-based types but no assertion for checking tool calls were made or specific tools were invoked.
- ~~`run_agent_loop` and `run_agent_loop_with_mcp` have ~500 lines of duplicated code — no shared test coverage for the common logic.~~ ✅ **Fixed in v0.9.11 — deduplicated via `run_agent_loop_inner()`**
- No integration tests for `RavenFabricClient` execution paths (client is created but never called).
- No integration tests for `ProviderFallbackChain` or `TokenBudget` (both are dead code).
- No integration tests for `AgentMessageBus` or `SwarmHealthMonitor` (both are dead code in orchestration).
- No verification tests for token tracking or tool call counting metrics (both show 0 in `/metrics`). *(Discovered in v0.9.11 rpi5 audit)*
- No verification tests for MCP server JSON-RPC `params` optionality. *(Discovered in v0.9.11 rpi5 audit)*
- No verification tests for RavenFabric policy pipe detection. *(Discovered in v0.9.11 rpi5 audit)*

---

## Performance Targets (v1.0)

| Metric | Target | Current |
|---|---|---|
| Stripped binary size | < 15 MB | 5.2 MB ✅ |
| Container image size | < 30 MB | 15.8 MB ✅ *(UPX compressed, v0.9.10)* |
| Cold start (single mode) | < 50 ms | 5.2 ms ✅ |
| Idle memory (server mode) | < 20 MB RSS | ~8 MiB RSS ✅ *(measured on rpi5 K3s)* |
| Memory under load (server mode) | < 30 MB RSS | ~10 MiB RSS ✅ *(after 3,597 requests, 7.5h test)* |
| Provider failover decision | < 5 ms | ✅ (v0.5.1) |
| Tool-call audit write | non-blocking, < 1 ms enqueue | ✅ (wired) |
| `/health` response time | < 10 ms | 3 ms ✅ *(measured on rpi5 K3s)* |
| `/chat` response time | < 1 s | 899 ms ✅ *(measured on rpi5 K3s)* |
| `/ready` response time | < 500 ms | 1,259 ms ⚠️ *(LLM-dependent; 30s TTL caching added in v0.9.14)* |
| Zero errors under sustained load | 0 errors | ✅ *(3,597 requests, 0 errors, 0 restarts)* |
| Memory leak | 0 | ✅ *(only +2 MiB from idle after 7.5h heavy testing)* |
| Token tracking | Accurate counts | ✅ **v0.9.14** *(wired to LLM response `usage` field)* |
| Tool calls counter | Accurate counts | ✅ **v0.9.14** *(incremented on each tool execution)* |
| MCP server `params` optional | Accept without | ✅ **v0.9.14** *(per JSON-RPC 2.0 spec)* |
| RavenFabric pipe policy | Allow pipes | ✅ **v0.9.14** *(segment-level validation)* |

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
| 0.9.11 | Azure OpenAI adapter (api-key auth). Agent loop deduplication. Eval harness agent loop integration. |
| 0.9.12 | Durable execution (checkpoint/resume) — agent loop saves state after each iteration. |
| 0.9.13 | Multi-agent patterns (debate, review-loop, research-synthesize, voting). |
| 0.9.14+ | Token tracking wired to LLM responses. Tool calls counter wired. RavenFabric pipe policy detection. MCP server optional `params`. `/ready` caching. |
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
8. ~~**No config hot-reload** — Changes require restart.~~ ✅ **Fixed in v0.9.6 (SIGHUP handler)** — but distroless container has no `kill` binary, making it hard to use in production. See v0.10 for `/reload` HTTP endpoint solution.
9. ~~**Container image ~50 MB** — Target is < 30 MB.~~ ✅ **Fixed in v0.9.10 (UPX compression)** — now ~15.8 MB.
10. **cargo-udeps findings** — Unused dependencies detected. *(periodic review)*
11. **cargo-outdated findings** — Dependencies behind latest. *(periodic review)*
12. **~60 `#[allow(dead_code)]` annotations** — Significant unwired infrastructure: `RavenFabricClient`, `ProviderFallbackChain`, `TokenBudget`, `AgentMessageBus`, `SwarmHealthMonitor`, `WebSearchConfig`, and ~15 unused error variants, ~15 unused struct fields, ~15 unused methods, ~5 dead error enums.
13. **`unwrap()` on audit log mutex** — 7+ calls on hot path (`audit.rs` lines 181, 315, 320, 325, 330, 361, 367). Will panic if mutex is poisoned.
14. ~~**`run_agent_loop` and `run_agent_loop_with_mcp` are nearly identical** — ~500 lines of duplicated code. The only difference is MCP tool registration.~~ ✅ **Deduplicated in v0.9.11 via `run_agent_loop_inner()`**
15. **Legacy `TOOL_CALL:` / `ARGS:` format still supported** — Dead code path in agent loop. No LLM provider generates this format.
16. ~~**`EvalRunner::run_task()` bypasses agent loop** — Calls `llm.chat()` directly instead of `run_agent_loop()`. Eval tasks don't test tool calling, ReAct loop, or security integration.~~ ✅ **Fixed in v0.9.11 — eval harness now uses `run_agent_loop()`**
17. **No tool call assertions in eval harness** — `Assertion` enum has 7 text-based types but no assertion for checking tool calls were made or specific tools were invoked.
18. ~~**Server mode has no agent execution endpoints** — Only `/health`, `/ready`, `/metrics`. No `/chat`, `/execute`, or `/tools`.~~ ✅ **Fixed in v0.9.6 — 6 new endpoints added**
19. ~~**Token tracking not wired to LLM responses** — `/metrics` shows 0 tokens across all requests. The `usage` field from LLM responses is not parsed and accumulated. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14**
20. ~~**Tool calls counter not wired** — `/metrics` shows 0 tool calls. Counter not incremented on tool execution in agent loop. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14**
21. ~~**`/ready` latency is LLM-dependent** — 1.26s response time because it waits for LLM connectivity check. Consider caching the LLM check result with a configurable TTL. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14 (30s TTL caching)**
22. ~~**`--eval /dev/null` produces no meaningful output** — When given `/dev/null` as input, the eval harness produces no meaningful output. Should handle empty input gracefully. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14**
23. ~~**MCP server JSON-RPC `params` field required** — Some MCP clients omit the `params` field in JSON-RPC requests. The server should accept requests without `params` (treat as empty). *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14**
24. ~~**RavenFabric policy blocks piped shell interpreters** — `sh -c "cmd | cmd2"` is blocked by policy because the pipe character is not in the allowed command list. Policy engine needs pipe detection. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14**
25. ~~**`imagePullPolicy: IfNotPresent` for `:latest` tag** — Stale `:latest` image not pulled on pod restart. Should use `imagePullPolicy: Always` when tag is `:latest`. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.14**
26. ~~**Distroless container limits HTTP testing** — No `curl`/`wget` in distroless image. Need to document `kubectl port-forward` as the testing method. *(Discovered in v0.9.11 rpi5 audit)*~~ ✅ **Fixed in v0.9.15**
27. ~~**`/tools/{name}` path parameter routing returns 404** — The `/tools` endpoint lists all tools correctly, but `/tools/{name}` returns `Not Found`. *(Discovered in v0.9.6 rpi5 testing)*~~ ✅ **Fixed — added `GET /tools/{name}` handler + improved error status mapping**
28. ~~**RavenFabric health check URL builder error** — Uses `ws://` scheme for HTTP health check endpoint. *(Discovered in v0.9.9 rpi5 testing, #42)*~~ ✅ **Fixed — `http_url()` helper converts `ws://` → `http://` and `wss://` → `https://`**
29. ~~**`/execute` returns empty result without `no_final_required`** — Agent loop hits max iterations when model doesn't emit `FINAL:`. *(Discovered in v0.9.9 rpi5 testing, #39)*~~ ✅ **Fixed — `no_final_required: true` default for background tasks**
30. ~~**Distroless container has no `kill` binary** — SIGHUP config reload requires `kill` or procfs access. *(Discovered in v0.9.6 rpi5 testing)*~~ ✅ **Fixed — added `POST /reload` HTTP endpoint as distroless-friendly alternative**

---

## How You Can Help

- **Contributors:** pick an unchecked item and open a PR (CLA required — see [LICENSING.md](LICENSING.md#contributor-license-agreement-cla)).
- **Security researchers:** audit the code and report responsibly. *(A `SECURITY.md` policy is planned for v0.2.)*
- **Users:** file issues for missing features or rough edges.
- **Enterprise:** ask about commercial licensing and priority features.

---

*Secure. Small. Efficient. Robust. Simple. — Simply the best.* 🐦‍⬛
