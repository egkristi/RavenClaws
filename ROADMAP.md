# 🐦‍⬛ RavenClaw Roadmap

**Vision:** RavenClaw shall become the ultimate AI agentic assistant and worker —
the supreme, most trusted, and most capable autonomous agent. Simply the best.

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

- Not a heavyweight orchestration platform — RavenClaw stays a small worker; large-scale mesh coordination is delegated to **RavenFabric**.
- Not a UI/IDE — RavenClaw is a headless binary + library; frontends consume it.
- No telemetry phone-home, ever. Observability is opt-in and self-hosted.

---

## Current State

**Version:** 0.1.0 (Pre-Alpha) — active development, APIs unstable.
**Stats:** 8 source modules, ~2,500 LOC, 4 LLM providers, multi-arch CI with signed images + SBOM.

| Component | Status | Details |
|---|---|---|
| Single agent (single-provider) | ✅ Working | Sends one prompt, logs response, exits |
| Single agent (multi-model) | ✅ Working | Iterates all providers, logs each response |
| LLM providers (4) | ✅ Working | LiteLLM, OpenAI, OpenRouter, Ollama (unified trait) |
| CLI & env-var overrides | ✅ Working | `--provider`, `--endpoint`, `--model`, layered TOML→env→flags |
| Config validation | ✅ Working | TLS enforcement, endpoint checks |
| Container & K8s security | ✅ Working | Distroless, non-root, read-only FS, dropped caps, seccomp, RBAC |
| CI/CD pipeline | ✅ Implemented | fmt + clippy `-D warnings` + test, 5-target builds, multi-arch images, **Cosign + SBOM + provenance + Trivy**, crates.io publish, releases — cross-compilation deps installed for all targets |
| Security scanning | ✅ Implemented | CodeQL, cargo-audit, cargo-deny, cargo-outdated, cargo-udeps, Trivy (FS + config), Hadolint, Kubescape, OSSF Scorecard, dependency review — all SARIF results uploaded to GitHub Security tab |
| Verification suite | ✅ Working | 94 system/integration checks · 8 modules · 4 targets (`scripts/verify.sh`: local, Docker, Linux, K8s, security, performance, LLM-quality) — shell-orchestrated, requires live services |
| Multi-model routing | ⚠️ Partial | `next_client()` round-robin exists but is never called; no intelligent routing |
| RavenFabric integration | ⚠️ Partial | Config struct exists, agent binary baked into the image with checksum verification; runtime integration not wired |
| `--exec` one-shot mode | ✅ Working | Sends prompt to LLM, prints response to stdout; full test coverage |
| Swarm / Supervisor modes | ⚠️ Stub | Return clear error instead of silent exit 0 |
| Rust unit tests | ✅ Working | 157 tests across all 5 modules; `mockito`-based HTTP tests for all 4 providers covering success, auth failure, rate limit, server error, and invalid JSON paths |
| Agent loop / ReAct planning | ✅ Working | perceive→plan→act→observe with max-iteration guard, `FINAL:` marker detection, configurable via `--max-iterations` |
| Tool-use / function calling | ✅ Working | Tool abstraction + registry + 4 built-in tools (shell, read/write file, web fetch) + agent loop wiring (`TOOL_CALL:` / `ARGS:` / `OBSERVATION:` pattern) |
| Deny-by-default policy | ✅ Working | `PolicyEngine` with shell, path, and network allow-lists |
| Sandboxed execution | ✅ Working | Workdir jail, resource limits, timeouts, path resolution |
| Audit log | ✅ Working | HMAC-SHA256 chained, tamper-evident, structured JSON output |
| Streaming responses | ✅ Working | SSE streaming for LiteLLM, default non-streaming fallback for others |
| Conversation memory | ✅ Working | `ConversationMemory` struct with configurable max history, auto-trim |
| Interactive REPL | ✅ Working | `--repl` flag with stdin loop, streaming output, `/exit` `/reset` commands |
| System prompt / persona | ✅ Working | `LLMConfig.system_prompt` field, CLI `--system-prompt`, env var override |
| Pre-built binary releases | 📋 Wired, untagged | CI produces them on tag; none released yet |

### 🔧 Known build & correctness blockers

These break real usage today and are the first thing to fix (see [Technical Debt](#technical-debt)):

1. ~~**`Cargo.lock` is git-ignored, but `--locked` is used in CI, Docker, `build.sh`, and `cargo publish`** → every fresh checkout fails to build. *(blocker)*~~ ✅ Fixed
2. ~~**Dockerfile pins the builder to `$BUILDPLATFORM` then cross-compiles to `$TARGET` with no cross-linker** → `linux/arm64` image build fails at link time. *(blocker)*~~ ✅ Fixed — cross-linkers installed
3. ~~**Dockerfile `curl | chmod +x` of the RavenFabric agent has no checksum/signature check** — supply-chain gap in a "secure by default" project. *(security)*~~ ✅ Fixed — SHA256 checksum verification added
4. ~~**CI cross-compilation builds fail** — `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-gnu` targets missing toolchain deps on runners. *(blocker)*~~ ✅ Fixed — `musl-tools` and `gcc-aarch64-linux-gnu` installed in CI
5. **The binary exits after one request, but the k8s Deployment expects a long-running process** → CrashLoopBackOff until server mode (v0.7) exists.

---

## Architecture

### Today

```text
        ┌──────────┐
        │  main.rs │  CLI (clap) · JSON logging · mode dispatch
        └────┬─────┘
   ┌─────────┼──────────────────────┐
┌──┴───┐ ┌───┴────┐ ┌───┴─────┐ ┌───┴───┐
│agent │ │ config │ │  error  │ │ tools │
│ loop │ │        │ │         │ │policy │
│ mem  │ │        │ │         │ │audit  │
└──┬───┘ └────────┘ └─────────┘ │sandbox│
   │                             └───────┘
┌──┴───────────────────────────┐
│ llm  (LLMProviderTrait)       │
│  LiteLLM · OpenAI · OpenRouter│
│  · Ollama · MultiModelManager │
└───────────────────────────────┘
```

### Target (v1.0)

```text
                    ┌──────────┐
                    │   CLI    │  single · serve · swarm · supervisor
                    └────┬─────┘
                  ┌──────┴───────┐
                  │  Agent Core  │  perceive → plan → act → observe (+ memory)
                  └──┬────┬───┬──┘
          ┌──────────┘    │   └──────────┐
     ┌────┴────┐    ┌─────┴────┐   ┌──────┴───────┐
     │  Tools  │    │ Providers│   │ Orchestration │
     │ policy  │    │ routing+ │   │ swarm/superv. │
     │ sandbox │    │ fallback+│   │ + RavenFabric │
     │ audit ✅│    │ budgets  │   │  (E2E remote) │
     └─────────┘    └──────────┘   └───────────────┘
          │
   ┌──────┴───────┐
   │ Observability│  metrics · tracing · health endpoint
   └──────────────┘
```

---

## Competitive Positioning

RavenClaw aims to be the **preferred alternative** to the current field — including
Nemoclaw, Hermes Agent, TrustClaw, ZeroClaw, PicoClaw, NanoClaw, Claude Cowork,
Manus, Perplexity Computer, Kimi Claw, and Vellum.

We don't win by out-featuring them. We win by refusing to compromise on all five
pillars at once. By category:

- **vs. cloud / hosted assistants** (Claude Cowork, Manus, Perplexity Computer, Kimi Claw): RavenClaw is **self-hostable, offline-capable, and source-available** under AGPLv3. Your data and tool calls never leave infrastructure you control — no phone-home.
- **vs. minimal agent runtimes** (ZeroClaw, PicoClaw, NanoClaw, TrustClaw): RavenClaw matches their footprint while adding a real **security model** (deny-by-default tool policy, audit log, sandboxing) and **multi-provider** routing with fallback.
- **vs. SDK / platform plays** (Vellum, Hermes Agent, Nemoclaw): RavenClaw is a **single dependency-light binary**, not a service you rent or a framework you marry. Embed it, ship it, forget it.

The bar: anything the field can do, RavenClaw should do **smaller, safer, and
simpler** — or deliberately not at all.

> **Where RavenClaw must lead, measurably (v1.0):** memory-safe core with zero
> known CVEs, sub-15 MB binary, sub-50 ms cold start, fully self-hostable and
> air-gappable, signed + SBOM-attested supply chain. These are claims we will
> benchmark and publish — not marketing.

---

## Features Required to Become the Preferred Alternative

Being *preferred* is a two-step bar: first reach **parity** on the capabilities the
field now treats as table stakes, then **win decisively** on the five pillars where
the cloud incumbents structurally can't follow. This is the gap analysis, grounded in
what the competition actually ships today.

### Part 1 — Table stakes (reach parity)

Baseline expectations for any "agentic assistant and worker" in 2026. Items marked
**NEW** are gaps not yet in the phase plan below; they are folded into the noted phase.

| Capability | Why it's table stakes (who has it) | In RavenClaw | Target |
|---|---|:--:|:--:|
| Agent loop (plan → act → observe) | Without it there is no "agent" | ✅ | v0.3 |
| Tool / function calling | The substrate for every action | 📋 | v0.4 |
| **MCP — client *and* server** **NEW** | The lingua franca for tools — adopted by Anthropic, OpenAI, Google, Microsoft, Salesforce; Vellum's agent node already does MCP discovery. Consume MCP tools *and* expose RavenClaw as an MCP server. | ❌ | **v0.4** |
| Sandboxed code execution | Now a native primitive (OpenAI Agents SDK); also our security wedge | 📋 | v0.4 |
| Persistent memory (short + long-term, vector recall) | Without it every session starts from zero | 📋 | v0.3 → v0.9 |
| Web search + headless browser tool **NEW** | Manus and Perplexity Comet center on browse / summarize / fill-forms / compare | ❌ | **v0.4** |
| File operations (read / write / edit) | Codex-style filesystem tools; core to "worker" | 📋 | v0.4 |
| Sub-agents / swarm orchestration | Kimi K2.6 runs **300 sub-agents / 4,000 steps**; the sub-agent pattern beats monolithic on long-horizon work | 📋 | v0.6 |
| Async / long-horizon background runs **NEW** | Manus's killer feature (cloud background); Kimi's 12-hour runs; persistent 24/7 agents | ⚠️ | **v0.7** |
| Scheduling / triggers (cron, webhook, file-watch) **NEW** | Proactive, set-and-forget operation | ❌ | **v0.7** |
| Streaming + intermediate results | First-class in Vellum; needed for interactive UX | 📋 | v0.3 |
| Multi-modal input (images, PDFs, docs) **NEW** | Manus and Kimi are multimodal; a "worker" must read documents | ❌ | **v0.5** |
| Connectors / integrations (OAuth: Drive, M365, Slack, GitHub, Notion) **NEW** | Claude-style connectors. Manus's weakness is *no* integrations — our opening | ❌ | **v0.6** |
| Skills / plugins (portable capability bundles) | Claude Agent Skills: instructions + scripts + resources, progressive disclosure | 📋 | pull earlier → v0.5 |
| Retries / provider fallback / fail-early | Vellum: retry, fall back to another provider, fail early | 📋 | v0.5 |
| Evals + observability + run inspection **NEW** | Vellum/Microsoft: evals, middleware logging, session inspection | ⚠️ | **v0.7** + eval harness |
| Human-in-the-loop approvals / guardrails **NEW** | Enterprises require guardrails + audit + HITL fallback | ❌ | **v0.4** |
| Output artifacts (docs, sheets, slides, sites) **NEW** | Manus builds sites/apps/decks; Claude skills emit pptx/xlsx/docx/pdf | ❌ | v0.8 (via skills) |

### Part 2 — Where RavenClaw wins (the "preferred" wedge)

Parity gets RavenClaw onto the shortlist. These pillar-based advantages get it
*chosen* — and the cloud incumbents (Manus, Perplexity, Kimi, Cowork-class) cannot
match all of them at once without abandoning their model.

| Differentiator | Why it beats the field | Pillars | Phase |
|---|---|:--:|:--:|
| **Local-first / self-hosted / air-gapped** | Manus is cloud-only with no free tier; Comet's "Local" mode is a browser, not a worker. RavenClaw runs fully offline incl. Ollama — data never leaves your control. | Secure · Simple | ✅ core, deepen v0.4 |
| **Security model: deny-by-default policy + sandbox + tamper-evident audit** | The field bolts security on; enterprises must add guardrails/audit/HITL themselves. We ship it in core. | Secure | v0.4 |
| **Memory-safe ~3 MB single binary, edge/embeddable** | No cloud agent runs on a Raspberry Pi or embeds inside another product. | Small · Efficient | ✅ |
| **Provider-agnostic + cost-aware routing + budgets** | Not locked to one model vendor; route cheap → capable and cap spend. | Efficient · Robust | v0.5 |
| **RavenFabric mesh: E2E-encrypted remote exec across a fleet** | Unique — competitors are single-host or single-cloud. Turns RavenClaw into a *distributed* workforce. | Robust | v0.6 |
| **No telemetry · deterministic · reproducible · signed + SBOM** | Trust as a feature, verifiable end to end. | Secure | ✅ → v1.0 |
| **Open core + commercial** | No lock-in, vs. proprietary cloud. | Simple | ✅ |

### Part 3 — The five that move the needle most

If focus is limited, these close the biggest "preferred" gap fastest:

1. **MCP client + server (v0.4)** — instant access to the entire tool ecosystem instead of reinventing it. Single highest-leverage feature.
2. **Agent loop + tools + sandbox (v0.3–v0.4)** — turns RavenClaw from a chat client into an actual worker.
3. **Local-first privacy + the security model (v0.4)** — the wedge no cloud agent can copy.
4. **Async / background + scheduling (v0.7)** — matches Manus's "assign-and-walk-away" and enables 24/7 agents.
5. **RavenFabric distributed execution (v0.6)** — the capability *no competitor has*.

> Table stakes get RavenClaw onto the shortlist. The pillars — local, secure, tiny,
> open, distributed — are why it gets picked. Build parity fast; never compromise the wedge.

---

## Phased Plan

Versions are capability milestones, not dates. Each must keep all five pillars green.

### v0.2 — Foundations: make the build honest and green 🔧

Cheapest, highest-leverage work. Nothing new ships until the basics are solid.

- [x] **Commit `Cargo.lock`** (remove from `.gitignore`) so `--locked` works in CI/Docker/publish.
- [x] **Fix multi-arch Docker build** — install cross-linkers (`gcc-aarch64-linux-gnu`) + set the cargo target linker.
- [x] **Verify the RavenFabric agent download** against a published checksum / Cosign signature.
- [x] **Single source of version truth** — wire clap `--version` to `env!("CARGO_PKG_VERSION")`.
- [x] **Replace `.expect()` on HTTP client construction** with error propagation (no abort path under `panic = "abort"`).
- [x] **Decide `--exec`**: implement one-shot mode (preferred, see v0.3) or remove the flag.
- [x] **Make swarm/supervisor fail loudly** — return a clear error instead of `exit 0` until implemented.
- [x] **Expand tests** — use `mockito` to exercise request/response/error paths for every provider; cover config parsing and the multi-model manager.
- [x] **README status-honesty.** ✅ done in this pass

**Exit criteria:** `cargo fmt && cargo clippy -D warnings && cargo test` green; `docker buildx` produces working `amd64`+`arm64` images; fresh clone builds with `--locked`.

### v0.3 — A real agent 🧠

Turn the client into an actual worker. *This is the milestone that makes RavenClaw an agent.*

- [x] **Agent loop**: perceive → plan → act → observe, with max-iteration guard and cancellation.
- [x] **`--exec "<task>"`** one-shot mode — sends prompt to LLM, prints response to stdout.
- [x] **Interactive REPL** (stdin) — continuous conversation mode.
- [x] **Conversation memory** — context across turns; configurable window (last N turns or token budget); session save/restore.
- [x] **Streaming responses** end to end (`stream = true`) across the trait and all clients.
- [x] **System-prompt / persona** configuration.
- [x] **Robust errors** — typed retries, timeouts, graceful provider failure. All error paths covered with `thiserror` + `anyhow`; 26 error tests across 7 variants.

**Exit criteria:** `ravenclaw --exec "summarize this repo"` performs a real multi-step task and returns a result.

### v0.4 — Tools and safety 🧰🔒

Agency with guardrails — the security differentiator.

- [x] **Tool / function-calling abstraction** (provider-agnostic schema + registry).
- [x] **Built-in tools**: shell exec, file read/write, web fetch — each behind a capability flag.
- [x] **Tool wiring into agent loop** — `run_agent_loop` detects `TOOL_CALL:` / `ARGS:` patterns, executes tools, injects results as `OBSERVATION:`.
- [x] **Deny-by-default policy** (command / path / host allow-lists), à la RavenFabric's RPCPolicy.
- [x] **Sandboxed execution** (workdir jail, resource limits, timeouts).
- [x] **Audit log** — structured, HMAC-chained, tamper-evident trail of every tool call.
- [ ] **MCP — client *and* server** *(NEW — highest-leverage)* — consume any Model Context Protocol tool/server, and expose RavenClaw itself as an MCP server. The industry tool standard (Anthropic, OpenAI, Google, Microsoft, Salesforce).
- [ ] **Web search + headless browser tool** *(NEW)* — search, navigate, extract, and fill forms (beyond simple web fetch).
- [ ] **Wire `zeroize`** for secret material; automatic secret/PII redaction in logs.
- [ ] **Honor `token_lifetime_secs`** for any issued credentials.
- [ ] **Prompt-injection defense** — instruction-boundary enforcement, output schema validation.
- [ ] **Human-in-the-loop approvals** *(NEW)* — configurable approval gates for sensitive tool calls (allow / deny / ask).

**Exit criteria:** an agent runs tools, but only those allowed by policy, with a complete audit log.

### v0.5 — Providers and routing 🔀

- [ ] **Collapse duplicated OpenAI-compatible clients** (LiteLLM/OpenAI/OpenRouter) into one parameterized client; keep Ollama as the documented variant. (`handle_response` is currently copy-pasted 4×.)
- [ ] **Routing strategies**: round-robin (load balance), cost-aware (cheap model for easy tasks), **fallback chains** on error/rate-limit.
- [ ] **Resilience**: retries with exponential backoff + jitter; per-provider circuit breaker.
- [ ] **Token accounting & per-run budgets/limits.**
- [ ] **Native Anthropic provider**; embeddings endpoint; tool-calling parity across providers.
- [ ] **Multi-modal input** *(NEW)* — images, PDFs, and documents as agent input.
- [ ] **Skill / plugin system** *(NEW — pulled from v0.9)* — portable capability bundles (instructions + scripts + resources), à la Claude Agent Skills, with progressive disclosure.

**Exit criteria:** a single run transparently fails over between providers and respects a token budget.

### v0.6 — Swarm, supervisor, and RavenFabric 🕸️

- [ ] **Supervisor mode** — task decomposition, sub-agent spawning, result aggregation, quality checks.
- [ ] **Swarm mode** — coordinated agents with a shared blackboard/state; per-subtask model selection.
- [ ] **RavenFabric integration** — secure E2E remote command execution + mesh coordination (the headline capability).
- [ ] **Agent communication** — structured message passing; conflict resolution across agents.
- [ ] **Connectors / integrations** *(NEW)* — OAuth connectors for Google Drive, M365, Slack, GitHub, Notion (acts as the user, not a shared service account).

**Exit criteria:** a supervisor decomposes a task across ≥3 sub-agents over RavenFabric and aggregates results.

### v0.7 — Observability and ops 📈

- [ ] **Long-running server mode** with a real HTTP `/health` `/ready` `/metrics` endpoint (fixes the k8s CrashLoop).
- [ ] **Prometheus metrics** (requests, tokens, tool calls, errors, latencies).
- [ ] **OpenTelemetry tracing** (opt-in, self-hosted collector, correlation IDs).
- [ ] **Graceful shutdown**, signal handling, `health_interval_secs` honored.
- [ ] **Helm chart**; systemd unit; optional self-update with rollback.
- [ ] **Async / long-horizon background runs** *(NEW)* — assign-and-walk-away background execution, resumable across restarts (matches Manus's headline UX).
- [ ] **Scheduling & triggers** *(NEW — moved from v0.9)* — cron, webhook, and file-watch activation for proactive 24/7 agents.
- [ ] **Eval harness + run inspection** *(NEW)* — golden-task evals, assertions on intermediate steps, and replayable run traces.

**Exit criteria:** RavenClaw runs as a stable long-lived workload with green probes and exported metrics.

### v0.8 — Enterprise and compliance 🏢 *(commercial-licensed)*

Maps to the commercial tier in [LICENSING.md](LICENSING.md).

- [ ] **RBAC + multi-tenant isolation** (separate workspaces, secrets, quotas).
- [ ] **SSO / SAML.**
- [ ] **SecurityPolicy** — immutable rules, blast-radius limits.
- [ ] **Multi-level audit logging** — levels (`off`/`basic`/`detailed`/`debug`), formats (JSON/CEF/LEEF/Syslog), shipping sinks, integrity chaining.
- [ ] **Compliance presets & reporting** (SOC2, ISO 27001, HIPAA, GDPR, PCI-DSS).
- [ ] **Air-gap / offline licensing**; runtime feature-flag gating.
- [ ] **Output artifacts & reporting** *(NEW)* — generate documents, spreadsheets, slides, and sites via the skill system (v0.5); underpins compliance and executive reporting.

### v0.9 — Hardening, ecosystem, advanced reasoning 💎

- [ ] **Threat model + external security review.**
- [ ] **Fuzzing** (`cargo fuzz`) + property tests for config/policy parsers.
- [ ] **Skill/plugin marketplace + WASM sandboxing** for third-party extensions (core MCP ships in v0.4, the skill system in v0.5).
- [ ] **SDKs** (Python/TS) and a documentation site.
- [ ] **Advanced reasoning** — tree-of-thought, self-reflection, uncertainty estimation / ask-for-help.
- [ ] **Memory tiers** — episodic, semantic (local embeddings), procedural.

### v1.0 — Simply the best 🏆

- [ ] **API stability** guarantees + semver discipline.
- [ ] **All performance targets met** and benchmarked against the field (published).
- [ ] **Complete docs**, examples, migration guides.
- [ ] **Reproducible builds.**

---

## Testing Strategy

- **Unit:** every module; provider request/response/error paths via `mockito`.
- **Integration:** end-to-end agent runs against a stubbed provider and a local Ollama.
- **Policy/security:** table-driven allow/deny tests; fuzzing on policy + config parsing.
- **CI gates:** `fmt`, `clippy -D warnings`, `test`, Trivy (CRITICAL/HIGH fail), SBOM per release.
- **Coverage goal:** ≥ 80% line coverage by v1.0; no `unwrap`/`expect` on non-test hot paths.

---

## Performance Targets (v1.0)

| Metric | Target |
|---|---|
| Stripped binary size | < 15 MB |
| Container image size | < 30 MB |
| Cold start (single mode) | < 50 ms |
| Idle memory (server mode) | < 20 MB RSS |
| Provider failover decision | < 5 ms |
| Tool-call audit write | non-blocking, < 1 ms enqueue |

---

## Security Hardening (by version)

| Version | Hardening added |
|---|---|
| 0.1 | Memory-safe Rust, TLS check, no creds in config, distroless, signed images, SBOM, Trivy. |
| 0.2 | Verified supply chain for downloaded binaries (SHA256 checksum); no panic/abort on client init; cross-compilation deps in CI. |
| 0.4 | Deny-by-default tool policy, sandboxed execution, audit log, secret zeroization, prompt-injection defense. |
| 0.6 | E2E-encrypted remote exec via RavenFabric. |
| 0.8 | RBAC, SecurityPolicy with blast-radius limits, compliance reporting. |
| 0.9 | External security review, fuzzing, published threat model. |

---

## Design Decisions

- **Rust, `unsafe` forbidden** — memory safety and small static binaries are foundational to "secure + small."
- **OpenAI-compatible core** — most providers speak it; one client shape covers LiteLLM/OpenAI/OpenRouter, with Ollama as the documented exception.
- **AGPLv3 + Commercial dual license** — keeps the core open, closes the SaaS loophole, funds development. See [LICENSING.md](LICENSING.md).
- **Delegate heavy orchestration to RavenFabric** — RavenClaw stays a small worker; the mesh/remote-exec substrate is a separate, specialized system.
- **No phone-home** — observability is opt-in and self-hosted; trust is a feature.

---

## Technical Debt

Concrete items carried from the current codebase:

1. ~~**`Cargo.lock` git-ignored vs. `--locked` everywhere** — breaks fresh-clone/CI/Docker/publish. *(blocker)*~~ ✅ Fixed
2. ~~**Docker arm64 cross-compile** lacks a cross-linker under `--platform=$BUILDPLATFORM`. *(blocker)*~~ ✅ Fixed
3. ~~**Unverified `curl | chmod +x`** of the RavenFabric agent in the Dockerfile. *(security)*~~ ✅ Fixed — SHA256 checksum verification
4. ~~**CI cross-compilation builds fail** — missing `musl-tools` and `gcc-aarch64-linux-gnu` on runners. *(blocker)*~~ ✅ Fixed
5. **k8s Deployment runs a program that exits immediately** → needs server mode (v0.7) or a Job manifest meanwhile.
6. **Client duplication** across LiteLLM/OpenAI/OpenRouter (`handle_response` ×4). *(v0.5)*
7. **Dead/unwired code:** `next_client`, `rustls` + `zeroize` deps, and all `security`/`ravenfabric` config fields. *(v0.5)*
8. ~~**Rust unit tests are shallow** — only 3 constructor/smoke tests; `mockito` unused.~~ ✅ Fixed — 149 tests across all modules
9. ~~**`.expect()` on HTTP client build** under `panic = "abort"` — aborts on a config hiccup.~~ ✅ Fixed
10. ~~**Version literal duplicated** in `main.rs` instead of `CARGO_PKG_VERSION`.~~ ✅ Fixed
11. ~~**README historically over-claimed** vs. implemented state~~ ✅ Fixed
12. ~~**`--exec` dead code** — CLI arg parsed but never used.~~ ✅ Fixed — fully implemented

---

## How You Can Help

- **Contributors:** pick an unchecked item and open a PR (CLA required — see [LICENSING.md](LICENSING.md#contributor-license-agreement-cla)).
- **Security researchers:** audit the code and report responsibly. *(A `SECURITY.md` policy is planned for v0.2.)*
- **Users:** file issues for missing features or rough edges.
- **Enterprise:** ask about commercial licensing and priority features.

---

*Secure. Small. Efficient. Robust. Simple. — Simply the best.* 🐦‍⬛
