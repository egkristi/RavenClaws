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
**Stats:** 5 source modules, ~1,070 LOC, 4 LLM providers, multi-arch CI with signed images + SBOM.

| Component | Status | Details |
|---|---|---|
| Single agent (single-provider) | ✅ Working | Sends one prompt, logs response, exits |
| Single agent (multi-model) | ✅ Working | Iterates all providers, logs each response |
| LLM providers (4) | ✅ Working | LiteLLM, OpenAI, OpenRouter, Ollama (unified trait) |
| CLI & env-var overrides | ✅ Working | `--provider`, `--endpoint`, `--model`, layered TOML→env→flags |
| Config validation | ✅ Working | TLS enforcement, endpoint checks |
| Container & K8s security | ✅ Working | Distroless, non-root, read-only FS, dropped caps, seccomp, RBAC |
| CI/CD pipeline | ✅ Implemented | fmt + clippy `-D warnings` + test, 5-target builds, multi-arch images, **Cosign + SBOM + provenance + Trivy**, crates.io publish, releases — ⚠️ **red until `Cargo.lock` is committed** |
| Security scanning | ✅ Implemented | CodeQL, cargo-audit, cargo-deny, cargo-outdated, cargo-udeps, Trivy (FS + config), Hadolint, Kubescape, OSSF Scorecard, dependency review — all SARIF results uploaded to GitHub Security tab |
| Tests | ⚠️ Minimal | Constructor/smoke-level unit tests only; `mockito` declared but unused. *(No 90+ suite exists — that was aspirational.)* |
| Multi-model routing | ⚠️ Partial | `next_client()` round-robin exists but is never called; no intelligent routing |
| RavenFabric integration | ⚠️ Partial | Config struct exists, agent binary baked into the image; runtime integration not wired |
| `--exec` one-shot mode | ❌ Dead code | CLI arg parsed but never read |
| Swarm / Supervisor modes | ❌ Stub | Warn "not yet implemented", exit 0 |
| Agent loop / ReAct planning | ❌ Not implemented | One-shot send-and-exit; no perceive→plan→act→observe |
| Tool-use / function calling | ❌ Not implemented | Agent cannot call tools |
| Streaming responses | ❌ Not implemented | `stream: None` hardcoded |
| Conversation memory | ❌ Not implemented | In-memory messages only, lost on exit |
| Pre-built binary releases | 📋 Wired, untagged | CI produces them on tag; none released yet |

### 🔧 Known build & correctness blockers

These break real usage today and are the first thing to fix (see [Technical Debt](#technical-debt)):

1. **`Cargo.lock` is git-ignored, but `--locked` is used in CI, Docker, `build.sh`, and `cargo publish`** → every fresh checkout fails to build. *(blocker)*
2. **Dockerfile pins the builder to `$BUILDPLATFORM` then cross-compiles to `$TARGET` with no cross-linker** → `linux/arm64` image build fails at link time. *(blocker)*
3. **Dockerfile `curl | chmod +x` of the RavenFabric agent has no checksum/signature check** — supply-chain gap in a "secure by default" project. *(security)*
4. **The binary exits after one request, but the k8s Deployment expects a long-running process** → CrashLoopBackOff until server mode (v0.7) exists.

---

## Architecture

### Today

```text
        ┌──────────┐
        │  main.rs │  CLI (clap) · JSON logging · mode dispatch
        └────┬─────┘
   ┌─────────┼──────────┐
┌──┴───┐ ┌───┴────┐ ┌───┴─────┐
│agent │ │ config │ │  error  │
└──┬───┘ └────────┘ └─────────┘
   │
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
     │ policy+ │    │ routing+ │   │ swarm/superv. │
     │ sandbox+│    │ fallback+│   │ + RavenFabric │
     │ audit   │    │ budgets  │   │  (E2E remote) │
     └────┬────┘    └──────────┘   └───────────────┘
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

## Phased Plan

Versions are capability milestones, not dates. Each must keep all five pillars green.

### v0.2 — Foundations: make the build honest and green 🔧

Cheapest, highest-leverage work. Nothing new ships until the basics are solid.

- [ ] **Commit `Cargo.lock`** (remove from `.gitignore`) so `--locked` works in CI/Docker/publish.
- [ ] **Fix multi-arch Docker build** — install cross-linkers (`gcc-aarch64-linux-gnu`) + set the cargo target linker, **or** drop `--platform=$BUILDPLATFORM` and build natively per-arch under QEMU.
- [ ] **Verify the RavenFabric agent download** against a published checksum / Cosign signature.
- [ ] **Single source of version truth** — wire clap `--version` to `env!("CARGO_PKG_VERSION")`.
- [ ] **Replace `.expect()` on HTTP client construction** with error propagation (no abort path under `panic = "abort"`).
- [ ] **Decide `--exec`**: implement one-shot mode (preferred, see v0.3) or remove the flag.
- [ ] **Make swarm/supervisor fail loudly** — return a clear error instead of `exit 0` until implemented.
- [ ] **Expand tests** — use `mockito` to exercise request/response/error paths for every provider; cover config parsing and the multi-model manager.
- [ ] **README status-honesty.** ✅ done in this pass

**Exit criteria:** `cargo fmt && cargo clippy -D warnings && cargo test` green; `docker buildx` produces working `amd64`+`arm64` images; fresh clone builds with `--locked`.

### v0.3 — A real agent 🧠

Turn the client into an actual worker. *This is the milestone that makes RavenClaw an agent.*

- [ ] **Agent loop**: perceive → plan → act → observe, with max-iteration guard and cancellation.
- [ ] **`--exec "<task>"`** one-shot mode + an **interactive REPL** (stdin).
- [ ] **Conversation memory** — context across turns; configurable window (last N turns or token budget); session save/restore.
- [ ] **Streaming responses** end to end (`stream = true`) across the trait and all clients.
- [ ] **System-prompt / persona** configuration.
- [ ] **Robust errors** — typed retries, timeouts, graceful provider failure.

**Exit criteria:** `ravenclaw --exec "summarize this repo"` performs a real multi-step task and returns a result.

### v0.4 — Tools and safety 🧰🔒

Agency with guardrails — the security differentiator.

- [ ] **Tool / function-calling abstraction** (provider-agnostic schema + registry).
- [ ] **Built-in tools**: shell exec, file read/write, web fetch, code analysis — each behind a capability flag.
- [ ] **Deny-by-default policy** (command / path / host allow-lists), à la RavenFabric's RPCPolicy.
- [ ] **Sandboxed execution** (workdir jail, resource limits, timeouts).
- [ ] **Wire `audit_log`** — structured, HMAC-chained, tamper-evident trail of every tool call.
- [ ] **Wire `zeroize`** for secret material; automatic secret/PII redaction in logs.
- [ ] **Honor `token_lifetime_secs`** for any issued credentials.
- [ ] **Prompt-injection defense** — instruction-boundary enforcement, output schema validation.

**Exit criteria:** an agent runs tools, but only those allowed by policy, with a complete audit log.

### v0.5 — Providers and routing 🔀

- [ ] **Collapse duplicated OpenAI-compatible clients** (LiteLLM/OpenAI/OpenRouter) into one parameterized client; keep Ollama as the documented variant. (`handle_response` is currently copy-pasted 4×.)
- [ ] **Routing strategies**: round-robin (load balance), cost-aware (cheap model for easy tasks), **fallback chains** on error/rate-limit.
- [ ] **Resilience**: retries with exponential backoff + jitter; per-provider circuit breaker.
- [ ] **Token accounting & per-run budgets/limits.**
- [ ] **Native Anthropic provider**; embeddings endpoint; tool-calling parity across providers.

**Exit criteria:** a single run transparently fails over between providers and respects a token budget.

### v0.6 — Swarm, supervisor, and RavenFabric 🕸️

- [ ] **Supervisor mode** — task decomposition, sub-agent spawning, result aggregation, quality checks.
- [ ] **Swarm mode** — coordinated agents with a shared blackboard/state; per-subtask model selection.
- [ ] **RavenFabric integration** — secure E2E remote command execution + mesh coordination (the headline capability).
- [ ] **Agent communication** — structured message passing; conflict resolution across agents.

**Exit criteria:** a supervisor decomposes a task across ≥3 sub-agents over RavenFabric and aggregates results.

### v0.7 — Observability and ops 📈

- [ ] **Long-running server mode** with a real HTTP `/health` `/ready` `/metrics` endpoint (fixes the k8s CrashLoop).
- [ ] **Prometheus metrics** (requests, tokens, tool calls, errors, latencies).
- [ ] **OpenTelemetry tracing** (opt-in, self-hosted collector, correlation IDs).
- [ ] **Graceful shutdown**, signal handling, `health_interval_secs` honored.
- [ ] **Helm chart**; systemd unit; optional self-update with rollback.

**Exit criteria:** RavenClaw runs as a stable long-lived workload with green probes and exported metrics.

### v0.8 — Enterprise and compliance 🏢 *(commercial-licensed)*

Maps to the commercial tier in [LICENSING.md](LICENSING.md).

- [ ] **RBAC + multi-tenant isolation** (separate workspaces, secrets, quotas).
- [ ] **SSO / SAML.**
- [ ] **SecurityPolicy** — immutable rules, blast-radius limits.
- [ ] **Multi-level audit logging** — levels (`off`/`basic`/`detailed`/`debug`), formats (JSON/CEF/LEEF/Syslog), shipping sinks, integrity chaining.
- [ ] **Compliance presets & reporting** (SOC2, ISO 27001, HIPAA, GDPR, PCI-DSS).
- [ ] **Air-gap / offline licensing**; runtime feature-flag gating.

### v0.9 — Hardening, ecosystem, advanced reasoning 💎

- [ ] **Threat model + external security review.**
- [ ] **Fuzzing** (`cargo fuzz`) + property tests for config/policy parsers.
- [ ] **Plugin & skill system** (Rust or WASM); MCP (Model Context Protocol) support.
- [ ] **SDKs** (Python/TS) and a documentation site.
- [ ] **Advanced reasoning** — tree-of-thought, self-reflection, uncertainty estimation / ask-for-help.
- [ ] **Memory tiers** — episodic, semantic (local embeddings), procedural.
- [ ] **Proactive operation** — scheduling, event/webhook triggers, file watchers.

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
| 0.2 | Verified supply chain for downloaded binaries; no panic/abort on client init. |
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

Concrete items carried from the current codebase (targeted for v0.2 unless noted):

1. **`Cargo.lock` git-ignored vs. `--locked` everywhere** — breaks fresh-clone/CI/Docker/publish. *(blocker)*
2. **Docker arm64 cross-compile** lacks a cross-linker under `--platform=$BUILDPLATFORM`. *(blocker)*
3. **Unverified `curl | chmod +x`** of the RavenFabric agent in the Dockerfile. *(security)*
4. **k8s Deployment runs a program that exits immediately** → needs server mode (v0.7) or a Job manifest meanwhile.
5. **Client duplication** across LiteLLM/OpenAI/OpenRouter (`handle_response` ×4). *(v0.5)*
6. **Dead/unwired code:** `--exec`, `next_client`, `rustls` + `zeroize` deps, and all `security`/`ravenfabric` config fields.
7. **Shallow tests** — constructors only; `mockito` unused.
8. **`.expect()` on HTTP client build** under `panic = "abort"` — aborts on a config hiccup.
9. **Version literal duplicated** in `main.rs` instead of `CARGO_PKG_VERSION`.
10. **README historically over-claimed** vs. implemented state — kept honest going forward via status markers.

---

## How You Can Help

- **Contributors:** pick an unchecked item and open a PR (CLA required — see [LICENSING.md](LICENSING.md#contributor-license-agreement-cla)).
- **Security researchers:** audit the code and report responsibly. *(A `SECURITY.md` policy is planned for v0.2.)*
- **Users:** file issues for missing features or rough edges.
- **Enterprise:** ask about commercial licensing and priority features.

---

*Secure. Small. Efficient. Robust. Simple. — Simply the best.* 🐦‍⬛
