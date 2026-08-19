# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 1.x | ✅ Active development — security fixes in next release |
| 0.9.x | ⚠️ Maintenance only — security fixes backported on request |
| < 0.9 | ❌ No longer supported |

## Reporting a Vulnerability

RavenClaws takes security seriously. If you discover a security vulnerability,
please report it privately **before** disclosing it publicly.

**Do NOT report security vulnerabilities via public GitHub issues.**

### How to Report

1. **Email:** [egkristi@gmail.com](mailto:egkristi@gmail.com) with subject line
   starting with `[RAVENCLAWS-SECURITY]`
2. **Include:**
   - Description of the vulnerability
   - Steps to reproduce
   - Affected versions
   - Potential impact
   - Any suggested fix (if known)

### What to Expect

- **Acknowledgment** within 48 hours
- **Initial assessment** within 5 business days
- **Fix timeline** communicated within 10 business days
- **Coordinated disclosure** date agreed upon

## Security Features

RavenClaws is built with security as a foundational pillar:

| Feature | Description |
|---|---|
| **Memory-safe Rust** | `unsafe` code is forbidden — no raw pointer manipulation, no undefined behavior |
| **Deny-by-default policy** | All tool calls validated against allow-lists before execution |
| **Sandboxed execution** | Workdir jail, resource limits, timeouts for all tool execution |
| **Tamper-evident audit log** | HMAC-SHA256 chained, structured JSON — detect any tampering |
| **Secret zeroization** | API keys and HMAC secrets zeroized on drop via `zeroize` crate |
| **Prompt-injection defense** | Instruction-boundary enforcement, output schema validation |
| **Distroless container** | No shell, no package manager, minimal attack surface |
| **Non-root container** | Runs as UID 65532 with dropped capabilities |
| **Read-only root filesystem** | Container filesystem is immutable at runtime |
| **Signed releases** | Cosign-signed container images with SBOM and provenance attestation |
| **No telemetry** | Zero phone-home — observability is opt-in and self-hosted |

## Supply Chain Security

- All container images are **Cosign-signed** with keyless signing via GitHub OIDC
- **SBOM** (Software Bill of Materials) generated for every release
- **SLSA provenance** attestation for build integrity
- **Trivy vulnerability scanning** on every build (CRITICAL/HIGH fail the pipeline)
- **Dependency auditing** via `cargo-audit` and `cargo-deny` on every commit
- All third-party binaries verified against published checksums

## Bug Bounty

There is currently no formal bug bounty program. Security researchers who
responsibly disclose vulnerabilities will be credited in release notes.

## Security Hardening Roadmap

| Version | Hardening |
|---|---|
| 0.1 | Memory-safe Rust, TLS enforcement, distroless container, signed images |
| 0.4 | Deny-by-default policy, sandboxed execution, audit log, prompt-injection defense |
| 0.8 | Secret zeroization, human-in-the-loop approvals |
| 0.9 | Inter-agent communication encryption, swarm-wide policy enforcement |
| 1.0 | ✅ External security review (audit 2026-08), ✅ published threat model, fuzzing targets |

---

## Threat Model

RavenClaws is an **agent runtime that executes untrusted LLM output** against the
local system (shell, filesystem, network, browser). The primary security objective
is **containment**: an agent must never perform an action outside its declared
policy, even when the LLM is prompt-injected, adversarial, or misbehaving.

### Trust boundaries

| Boundary | What crosses it | Primary control |
|---|---|---|
| LLM → Agent | Tool-call intent (untrusted) | `PolicyEngine` allow-lists, `InjectionDetector` |
| Agent → Shell | Command execution | `ShellPolicy` command allow-list, pipe-segment analysis, timeouts |
| Agent → Filesystem | Read/write paths | `PathPolicy` allow-lists, size limits, workdir jail |
| Agent → Network | HTTP fetches, egress | `NetworkPolicy` host allow-list, `WebAccessPolicy` domain rules |
| Agent → Browser | CDP automation | `BrowserTool` gated behind explicit config |
| Plugin → Host | WASM guest code | `wasmtime` sandbox (feature `plugins`) |
| Host → Audit | Event records | HMAC-SHA256 chained log, key zeroized on drop |

### Threat actors & mitigations

| Threat | Attack | Mitigation |
|---|---|---|
| Prompt injection | LLM emits `shell_exec`/`write_file` to exfiltrate or destroy | Deny-by-default policy; sensitive tools require HITL approval (`--require-approval`) |
| Malicious dependency | Compromised crate ships a backdoor | `cargo audit`/`cargo-deny` in CI; signed+SBOM-attested releases |
| Supply-chain tampering | Altered binary in transit | Cosign signing, SHA256 checksums, SLSA provenance |
| Sandbox escape | WASM plugin escapes `wasmtime` | Pinned patched `wasmtime` (no known CRITICAL CVEs); plugins off by default |
| Secret exfiltration | API keys leaked in logs/config | `zeroize` on drop; keys via env/Secret, never config files |
| Denial of service | Resource exhaustion via tools | Timeouts, size limits, `LoadManager` rate limiting/shedding |

### Residual risks (accepted, documented)

- **In-memory audit key** — the HMAC key is per-process (`OsRng`) and not persisted,
  so tamper-evidence cannot be verified across restarts.
- **Plain-HTTP in-cluster LLM** — service-to-service LiteLLM traffic is HTTP by
  design; TLS is terminated at the mesh/ingress.
- **Heuristic complexity routing** — model routing is heuristic, not a trained
  classifier.

---

## Security Posture Profiles

Preset hardening levels, from least to most restrictive. Apply via the
`[security]` config section (see `docs/guides/configuration.md`).

### Profile 1 — Development (`dev`)

For local experimentation. TLS relaxed, broad tool access, no HITL.

```toml
[security]
require_tls = false
audit_log = true
prompt_injection_protection = true
```

### Profile 2 — Production (`prod`)

Default posture. TLS enforced externally, deny-by-default policy, HITL for
sensitive tools, audit + prompt-injection defense on.

```toml
[security]
require_tls = true
token_lifetime_secs = 3600
audit_log = true
prompt_injection_protection = true
```

Use `--require-approval` (or `require_approval_all = true` in policy) for
sensitive tool calls.

### Profile 3 — Air-gapped / high-assurance (`airgap`)

Maximum containment for offline or regulated environments. No network egress,
minimal tools, all writes to an isolated workspace, audit always on.

```toml
[security]
require_tls = true
token_lifetime_secs = 1800
audit_log = true
prompt_injection_protection = true
```

Combine with a `NetworkPolicy` (`deny_all = true`) and a `PathPolicy` scoped to
a single workspace directory.

### Choosing a profile

| Consideration | dev | prod | airgap |
|---|---|---|---|
| Network egress | open | allow-listed | denied |
| Tool access | broad | policy-gated | minimal |
| HITL approval | off | on (sensitive) | on (all) |
| Audit log | on | on | on |
| Suitable for | laptops, CI | servers, K8s | regulated, offline |
