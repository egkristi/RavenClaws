# 🐦‍⬛ RavenClaw Roadmap

**Vision:** RavenClaw shall become the supreme, most trusted, and most capable agentic AI worker, automation agent, and AI assistant.

**Core Principles:**
- **Small** — Not bloated. A lean, focused codebase that does one thing exceptionally well.
- **Sleek** — Elegant architecture. Clean APIs. Minimal cognitive overhead.
- **Secure** — Security by design, not as an afterthought. Fail-closed. Memory-safe.
- **Easy to use** — One command to run. Sensible defaults. Zero-config for common cases.
- **Robust** — Battle-tested. Deterministic. Predictable under load.

---

## Priority: Critical — v0.1.0 Release Blockers

*Fix critical issues. Ship a working developer preview. Establish trust.*

- [ ] **Fix `--exec` dead code** — CLI arg exists but is never read in `main.rs`. Wire it up or remove the flag.
- [ ] **Fix swarm/supervisor stubs** — `--mode swarm` and `--mode supervisor` print "not yet implemented" and exit 0. Return a clear error instead.
- [ ] **Set up CI/CD pipeline**
  - [ ] GitHub Actions workflow — build binaries for all targets on every push
  - [ ] Release workflow — auto-build and attach binaries to GitHub Releases on tag push
  - [ ] Container registry push — auto-build and push multi-arch Docker images to GHCR on release
- [ ] **Ship pre-built binaries** — Make `curl -LO` installable binaries for all 5 target triples
- [ ] **Expand `cargo test`** — Currently only 2 unit tests. Add tests for config parsing, LLM client creation, error handling, and multi-model manager.
- [ ] **Tag and release v0.1.0** — Create and push version tag

---

## Priority: High — Core Agent Loop & Modes

*Solidify the core. Ship what works. Establish trust.*

- [ ] **Implement tool-use (function calling)**
  - [ ] Tool definition schema (name, description, parameters)
  - [ ] Tool registry — register and discover available tools
  - [ ] Function calling support in LLM provider trait
  - [ ] Tool execution engine — call tools and return results to LLM
  - [ ] Built-in tools: file read/write, shell command, web fetch, code analysis
- [ ] **Implement agent loop with planning**
  - [ ] ReAct-style reasoning: Think → Act → Observe → Repeat
  - [ ] Max iteration guard — prevent infinite loops
  - [ ] Plan persistence — save/restore in-progress plans
- [ ] **Add conversation memory**
  - [ ] Persistent context across turns (not just in-memory messages)
  - [ ] Configurable memory window (last N turns or token budget)
  - [ ] Session save/restore on restart
- [ ] **Add streaming responses**
  - [ ] Token-by-token output for interactive use
  - [ ] Support in LLM provider trait and all 4 clients
  - [ ] CLI streaming output (print as tokens arrive)
- [ ] **Implement swarm mode**
  - [ ] Multiple agents collaborating on tasks with RavenFabric coordination
  - [ ] Round-robin load balancing across providers
  - [ ] Model-specific routing (cheap models for simple tasks, expensive for complex)
  - [ ] Fallback chain on provider errors
- [ ] **Implement supervisor mode**
  - [ ] Orchestrator agent that delegates to sub-agents
  - [ ] Task decomposition and assignment
  - [ ] Result aggregation and quality checking
  - [ ] Recursive sub-agent spawning
- [ ] **Integrate RavenFabric** — Config struct and docker-compose placeholder exist, but no actual integration. Crate is commented out in Cargo.toml.
- [ ] **Harden security**
  - [ ] Prompt injection defense — input sanitization, instruction boundary enforcement
  - [ ] Tool sandboxing — each tool call validated against allowlist
  - [ ] Output validation — structured output parsing with schema enforcement
  - [ ] Rate limiting — per-agent, per-provider rate control
  - [ ] Secrets redaction — automatic detection and masking of secrets in logs/outputs
- [ ] **Expand verification**
  - [ ] Property-based testing — fuzz config parsing, LLM response handling
  - [ ] Integration test suite — end-to-end tests with mock LLM server
  - [ ] Benchmark suite — track latency, throughput, memory usage over time

---

## Priority: Medium — Advanced Agentic Capabilities

*Make RavenClaw the most capable agent framework available.*

- [ ] **Build advanced tool system**
  - [ ] Built-in tool library: file system ops, shell execution (sandboxed), web fetching, code analysis, database querying (SQLite, PostgreSQL), API client (REST, GraphQL)
  - [ ] MCP (Model Context Protocol) support — compatible with the MCP ecosystem
  - [ ] Custom tool SDK — define tools in Rust or via WASM plugins
  - [ ] Tool composition — chain tools together into reusable workflows
- [ ] **Implement memory & state management**
  - [ ] Episodic memory — recall past conversations and task outcomes
  - [ ] Semantic memory — vector-based knowledge retrieval (local embeddings)
  - [ ] Procedural memory — learn and reuse successful task patterns
  - [ ] Working memory — short-term context window management
  - [ ] State persistence — save and restore agent state across restarts
- [ ] **Implement multi-agent orchestration**
  - [ ] Agent specialization — define agent roles with specific tools and models
  - [ ] Hierarchical delegation — supervisor → specialist → worker tree
  - [ ] Agent communication — structured message passing between agents
  - [ ] Conflict resolution — handle contradictory outputs from multiple agents
- [ ] **Add proactive & scheduled operation**
  - [ ] Cron/heartbeat scheduling — run agents on a timer
  - [ ] Event-driven triggers — webhook-based activation
  - [ ] File system watchers — react to file changes
  - [ ] Idle-time processing — background tasks during inactivity

---

## Priority: Medium — Enterprise & Observability

*Make RavenClaw the default choice for production AI workloads.*

- [ ] **Add enterprise security features**
  - [ ] RBAC (Role-Based Access Control) — multi-tenant agent isolation
  - [ ] SSO/SAML integration — enterprise authentication
  - [ ] Audit trail — immutable, cryptographically signed audit logs
  - [ ] Compliance reporting — SOC2, HIPAA, GDPR-ready reporting
  - [ ] Air-gap deployment — fully offline operation
  - [ ] Multi-tenant isolation — separate workspaces, secrets, and quotas per tenant
- [ ] **Add observability**
  - [ ] Prometheus metrics — request count, latency, error rates, token usage
  - [ ] OpenTelemetry tracing — distributed tracing across agent calls
  - [ ] Structured logging — JSON logs with correlation IDs
  - [ ] Health check API — `/health`, `/ready`, `/metrics` endpoints
  - [ ] Agent dashboard — web UI for monitoring agent activity

---

## Priority: Low — Ecosystem & Operations

*Nice-to-have improvements with no fixed timeline.*

- [ ] **Improve deployment options**
  - [ ] Helm chart — production-grade Kubernetes deployment
  - [ ] Terraform module — infrastructure-as-code deployment
  - [ ] Systemd service — native Linux service management
  - [ ] Auto-update — self-updating binary with rollback
- [ ] **Build ecosystem**
  - [ ] Plugin system — dynamic loading of community extensions
  - [ ] Agent marketplace — share and discover agent configurations
  - [ ] Skill system — reusable capability packages
  - [ ] Community templates — pre-built agents for common tasks
  - [ ] CI/CD integration — GitHub Actions, GitLab CI plugins
- [ ] **Add advanced reasoning**
  - [ ] Tree-of-Thought — explore multiple reasoning paths in parallel
  - [ ] Self-reflection — agent critiques and improves its own outputs
  - [ ] Multi-step planning — decompose complex goals into executable plans
  - [ ] Uncertainty estimation — know when to ask for human help
  - [ ] Tool discovery — agent autonomously discovers and learns new tools
- [ ] **Add learning & adaptation**
  - [ ] Preference learning — adapt to user preferences over time
  - [ ] Task automation — learn repetitive patterns and automate them
  - [ ] Feedback integration — improve from explicit and implicit feedback
  - [ ] Transfer learning — apply knowledge across different domains
- [ ] **Add human-AI collaboration**
  - [ ] Interactive mode — real-time human-in-the-loop collaboration
  - [ ] Approval workflows — configurable approval gates for sensitive actions
  - [ ] Explanation generation — clear, concise explanations of agent decisions
  - [ ] Multi-modal interaction — process images, audio, and documents

---

## Competitive Differentiation

| Capability | RavenClaw (Target) | OpenClaw | OpenManus | Vellum | Others |
|---|---|---|---|---|---|
| **Language** | Rust (memory-safe, fast) | TypeScript/Node | Python | TypeScript | Python/TS |
| **Binary size** | <5MB | ~100MB+ | N/A (Python) | N/A (Bun/TS) | Varies |
| **Startup time** | <15ms | ~500ms+ | ~2s+ | ~1s+ | Varies |
| **Security posture** | Fail-closed, memory-safe | 15+ CVEs in 2026 | DIY security | Sandboxed | Varies |
| **Multi-provider** | Unified trait, 4 providers | Plugin-based | OpenAI-centric | Multi-provider | Varies |
| **Swarm mode** | Native (Rust) | Via plugins | Via Python | Via gateway | Varies |
| **Deployment** | Binary, Docker, K8s | npm, Docker | pip, Docker | Bun, Docker | Varies |
| **License** | AGPLv3 + Commercial | Proprietary? | MIT | MIT | Varies |
| **Verification** | 88 tests, 8 modules, all targets | Limited | Community tests | Internal tests | Varies |

### Key Battlegrounds

1. **Security** — OpenClaw had 15+ CVEs in 2026 alone (sandbox escapes, prompt injection, path traversal, auth bypass). RavenClaw's Rust foundation and fail-closed design are our strongest differentiator. **We must never ship a security vulnerability.**

2. **Performance** — Rust gives us 10-100x faster startup, lower memory, and smaller binaries than Node.js or Python competitors. This matters for edge deployment, serverless, and high-density hosting.

3. **Simplicity** — One binary. Zero dependencies at runtime. No Python runtime, no Node runtime, no virtualenv. `./ravenclaw --mode single` and it works.

4. **Verification** — Our 88-test suite across 8 modules and 5 deployment targets is already more comprehensive than any competitor. We will maintain this as a point of pride.

5. **Open Source + Commercial** — AGPLv3 protects against cloud provider exploitation while the commercial license funds development. MIT alternatives (OpenManus, Vellum) risk the MongoDB/Elasticsearch fate.

---

## How You Can Help

- **Contributors:** Pick an unassigned item and open a PR.
- **Security researchers:** Audit our code. Report vulnerabilities via [security policy](SECURITY.md).
- **Users:** File issues for missing features or rough edges.
- **Enterprise customers:** Contact us about commercial licensing and priority features.

---

*RavenClaw — Small. Sleek. Secure. Supreme.* 🐦‍⬛
