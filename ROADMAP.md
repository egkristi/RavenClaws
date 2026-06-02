# 🐦‍⬛ RavenClaw Roadmap

**Vision:** RavenClaw shall become the supreme, most trusted, and most capable agentic AI worker, automation agent, and AI assistant — superior to OpenClaw, Nemoclaw, Hermes Agent, TrustClaw, MiniClaw, ZeroClaw, PicoClaw, NanoClaw, Manus/OpenManus, Claude Cowork, Kimi Claw, Vellum, and all others.

**Core Principles:**
- **Small** — Not bloated. A lean, focused codebase that does one thing exceptionally well.
- **Sleek** — Elegant architecture. Clean APIs. Minimal cognitive overhead.
- **Secure** — Security by design, not as an afterthought. Fail-closed. Memory-safe.
- **Easy to use** — One command to run. Sensible defaults. Zero-config for common cases.
- **Robust** — Battle-tested. Deterministic. Predictable under load.

---

## Phase 1: Foundation Fortification (Now — Q3 2026)

*Solidify the core. Ship what works. Establish trust.*

### 1.1 Complete Core Agent Loop
- [x] Single agent mode (basic chat completion)
- [x] Multi-provider support (LiteLLM, OpenAI, OpenRouter, Ollama)
- [x] Multi-model manager (multiple providers simultaneously)
- [x] CLI with env-var overrides
- [x] **OpenAI-compatible API support** — Any provider with an OpenAI-compatible `/v1/chat/completions` endpoint works with zero code changes (Groq, Together AI, Fireworks, Perplexity, DeepSeek native, xAI/Grok, Mistral native, etc.)
- [ ] **Tool-use (function calling)** — The #1 missing piece. Agent must call tools, not just chat.
- [ ] **Agent loop with planning** — ReAct-style reasoning: Think → Act → Observe → Repeat
- [ ] **Conversation memory** — Persistent context across turns (not just in-memory messages)
- [ ] **Streaming responses** — Real-time token-by-token output for interactive use

### 1.2 Implement Swarm & Supervisor Modes
- [ ] **Swarm mode** — Multiple agents collaborating on tasks with RavenFabric coordination
  - Round-robin load balancing across providers
  - Model-specific routing (cheap models for simple tasks, expensive for complex)
  - Fallback chain on provider errors
- [ ] **Supervisor mode** — Orchestrator agent that delegates to sub-agents
  - Task decomposition and assignment
  - Result aggregation and quality checking
  - Recursive sub-agent spawning

### 1.3 Security Hardening
- [x] TLS enforcement for production endpoints
- [x] No credentials in config files (env vars only)
- [x] Read-only root filesystem (container)
- [x] Non-root user (container)
- [x] Dropped capabilities (container)
- [x] Audit logging
- [x] Token lifetime limits
- [ ] **Prompt injection defense** — Input sanitization, instruction boundary enforcement
- [ ] **Tool sandboxing** — Each tool call validated against allowlist
- [ ] **Output validation** — Structured output parsing with schema enforcement
- [ ] **Rate limiting** — Per-agent, per-provider rate control
- [ ] **Secrets redaction** — Automatic detection and masking of secrets in logs/outputs

### 1.4 Verification & Quality
- [x] 38-test verification suite across all deployment targets
- [x] Binary integrity checks (no debug symbols, no hardcoded secrets)
- [x] Performance benchmarks (startup <100ms, config load <50ms)
- [x] LLM response quality tests across 5 models
- [ ] **Property-based testing** — Fuzz config parsing, LLM response handling
- [ ] **Integration test suite** — End-to-end tests with mock LLM server
- [ ] **Benchmark suite** — Track latency, throughput, memory usage over time

---

## Phase 2: Agentic Excellence (Q3 2026 — Q1 2027)

*Make RavenClaw the most capable agent framework available.*

### 2.1 Advanced Tool System
- [ ] **Built-in tool library:**
  - File system operations (read, write, search, glob)
  - Shell command execution (sandboxed)
  - Web fetching and scraping
  - Code analysis and manipulation
  - Database querying (SQLite, PostgreSQL)
  - API client (REST, GraphQL)
- [ ] **MCP (Model Context Protocol) support** — Compatible with the MCP ecosystem
- [ ] **Custom tool SDK** — Define tools in Rust or via WASM plugins
- [ ] **Tool composition** — Chain tools together into reusable workflows

### 2.2 Memory & State Management
- [ ] **Episodic memory** — Recall past conversations and task outcomes
- [ ] **Semantic memory** — Vector-based knowledge retrieval (local embeddings)
- [ ] **Procedural memory** — Learn and reuse successful task patterns
- [ ] **Working memory** — Short-term context window management
- [ ] **State persistence** — Save and restore agent state across restarts

### 2.3 Multi-Agent Orchestration
- [ ] **RavenFabric integration** — Full swarm coordination protocol
- [ ] **Agent specialization** — Define agent roles with specific tools and models
- [ ] **Hierarchical delegation** — Supervisor → Specialist → Worker tree
- [ ] **Agent communication** — Structured message passing between agents
- [ ] **Conflict resolution** — Handle contradictory outputs from multiple agents

### 2.4 Proactive & Scheduled Operation
- [ ] **Cron/Heartbeat scheduling** — Run agents on a timer
- [ ] **Event-driven triggers** — Webhook-based activation
- [ ] **File system watchers** — React to file changes
- [ ] **Idle-time processing** — Background tasks during inactivity

---

## Phase 3: Enterprise & Ecosystem (Q1 2027 — Q3 2027)

*Make RavenClaw the default choice for production AI workloads.*

### 3.1 Enterprise Features
- [ ] **RBAC (Role-Based Access Control)** — Multi-tenant agent isolation
- [ ] **SSO/SAML integration** — Enterprise authentication
- [ ] **Audit trail** — Immutable, cryptographically signed audit logs
- [ ] **Compliance reporting** — SOC2, HIPAA, GDPR-ready reporting
- [ ] **Air-gap deployment** — Fully offline operation
- [ ] **Multi-tenant isolation** — Separate workspaces, secrets, and quotas per tenant

### 3.2 Observability
- [ ] **Prometheus metrics** — Request count, latency, error rates, token usage
- [ ] **OpenTelemetry tracing** — Distributed tracing across agent calls
- [ ] **Structured logging** — JSON logs with correlation IDs
- [ ] **Health check API** — `/health`, `/ready`, `/metrics` endpoints
- [ ] **Agent dashboard** — Web UI for monitoring agent activity

### 3.3 Deployment & Operations
- [ ] **Helm chart** — Production-grade Kubernetes deployment
- [ ] **Terraform module** — Infrastructure-as-code deployment
- [ ] **Systemd service** — Native Linux service management
- [ ] **Auto-update** — Self-updating binary with rollback
- [ ] **Plugin system** — Dynamic loading of community extensions

### 3.4 Ecosystem
- [ ] **Agent marketplace** — Share and discover agent configurations
- [ ] **Skill system** — Reusable capability packages (inspired by AgentSkills)
- [ ] **Community templates** — Pre-built agents for common tasks
- [ ] **CI/CD integration** — GitHub Actions, GitLab CI plugins

---

## Phase 4: Intelligence & Autonomy (Q3 2027+)

*Push the boundaries of what autonomous agents can do.*

### 4.1 Advanced Reasoning
- [ ] **Tree-of-Thought** — Explore multiple reasoning paths in parallel
- [ ] **Self-reflection** — Agent critiques and improves its own outputs
- [ ] **Multi-step planning** — Decompose complex goals into executable plans
- [ ] **Uncertainty estimation** — Know when to ask for human help
- [ ] **Tool discovery** — Agent autonomously discovers and learns new tools

### 4.2 Learning & Adaptation
- [ ] **Preference learning** — Adapt to user preferences over time
- [ ] **Task automation** — Learn repetitive patterns and automate them
- [ ] **Feedback integration** — Improve from explicit and implicit feedback
- [ ] **Transfer learning** — Apply knowledge across different domains

### 4.3 Human-AI Collaboration
- [ ] **Interactive mode** — Real-time human-in-the-loop collaboration
- [ ] **Approval workflows** — Configurable approval gates for sensitive actions
- [ ] **Explanation generation** — Clear, concise explanations of agent decisions
- [ ] **Multi-modal interaction** — Process images, audio, and documents

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
| **Verification** | 38 tests, all targets | Limited | Community tests | Internal tests | Varies |

### Key Battlegrounds

1. **Security** — OpenClaw had 15+ CVEs in 2026 alone (sandbox escapes, prompt injection, path traversal, auth bypass). RavenClaw's Rust foundation and fail-closed design are our strongest differentiator. **We must never ship a security vulnerability.**

2. **Performance** — Rust gives us 10-100x faster startup, lower memory, and smaller binaries than Node.js or Python competitors. This matters for edge deployment, serverless, and high-density hosting.

3. **Simplicity** — One binary. Zero dependencies at runtime. No Python runtime, no Node runtime, no virtualenv. `./ravenclaw --mode single` and it works.

4. **Verification** — Our 38-test suite across 5 deployment targets is already more comprehensive than any competitor. We will maintain this as a point of pride.

5. **Open Source + Commercial** — AGPLv3 protects against cloud provider exploitation while the commercial license funds development. MIT alternatives (OpenManus, Vellum) risk the MongoDB/Elasticsearch fate.

---

## Immediate Priorities (Next 90 Days)

1. **Tool-use (function calling)** — Without this, RavenClaw is a chat client, not an agent. This is the #1 blocker.
2. **Agent loop with planning** — ReAct-style reasoning loop with tool execution.
3. **Streaming responses** — Required for interactive UX.
4. **Conversation memory** — Persistent context across turns.
5. **Swarm mode implementation** — Multi-agent coordination.
6. **Supervisor mode implementation** — Hierarchical task delegation.
7. **Prompt injection defense** — Input sanitization and boundary enforcement.
8. **MCP protocol support** — Ecosystem compatibility.

---

## How You Can Help

- **Contributors:** Pick an unassigned item from Phase 1 or 2 and open a PR.
- **Security researchers:** Audit our code. Report vulnerabilities via [security policy](SECURITY.md).
- **Users:** File issues for missing features or rough edges.
- **Enterprise customers:** Contact us about commercial licensing and priority features.

---

*RavenClaw — Small. Sleek. Secure. Supreme.* 🐦‍⬛
