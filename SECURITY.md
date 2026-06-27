# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.9.x | ✅ Active development — security fixes in next release |
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
| 1.0 | External security review, fuzzing, published threat model |
