# Known Issues

This document tracks known problems in RavenClaw that are not yet resolved.
Items are ordered by severity/impact.

---

## 🚨 Critical

### k8s Deployment enters CrashLoopBackOff

**Problem:** The binary exits after processing one request, but the k8s Deployment
(`k8s/deployment.yaml`) expects a long-running process. The pod immediately enters
`CrashLoopBackOff`.

**Root cause:** RavenClaw currently has no server/daemon mode. It processes a single
request and exits. A persistent server mode is planned for v0.7.

**Workaround:** None yet. The k8s manifest cannot be used until server mode exists.

**Tracking:** ROADMAP.md v0.7 — Async / long-horizon background runs.

---

## 🔧 Build & CI

### Container Build workflow fails at "Set up job"

**Problem:** The `Container Images` job in `.github/workflows/build.yml` fails
intermittently at the "Set up job" step with an infrastructure error.

**Root cause:** GitHub Actions runner provisioning issue — not a code defect.
The workflow uses `docker/build-push-action@v6` with multi-arch (`linux/amd64`,
`linux/arm64`) and requires QEMU + Buildx setup.

**Frequency:** Consistent failure on recent pushes.

**Workaround:** Re-run the workflow manually. If it persists, investigate GitHub
Actions runner availability for the repository.

### Linux cross-compilation builds fail

**Problem:** The following build targets fail in CI:
- `x86_64-unknown-linux-musl` — Build step fails
- `aarch64-unknown-linux-gnu` — Build step fails

**Root cause:** Missing cross-compilation toolchain on the GitHub Actions runner.
The `x86_64-unknown-linux-musl` target requires `musl-tools`, and
`aarch64-unknown-linux-gnu` requires `gcc-aarch64-linux-gnu`. These are not
pre-installed on `ubuntu-latest` runners.

**Note:** macOS targets (`x86_64-apple-darwin`, `aarch64-apple-darwin`) build
successfully on `macos-latest` runners.

**Workaround:** Install missing cross-compilation dependencies in the workflow,
or use native builds via QEMU.

### Security Scan workflow fails

**Problem:** The `Security Scan` workflow (`.github/workflows/security-scan.yml`)
fails consistently.

**Root cause:** Unknown — likely related to Trivy scanner configuration or
permissions. Needs investigation.

---

## 🧪 Code Quality

### `next_client()` round-robin method never called

**Problem:** `MultiModelManager::next_client()` in `src/llm.rs` implements
round-robin load balancing across providers, but is never invoked anywhere in
the codebase.

**Impact:** Multi-model mode initializes all providers but always uses the first
one. Round-robin distribution is dead code.

**Tracking:** ROADMAP.md v0.5 — Provider-agnostic + cost-aware routing.

### `handle_response()` code duplicated across providers

**Problem:** The `handle_response()` method in each LLM client
(`LiteLLMClient`, `OpenRouterClient`, `OllamaClient`, `OpenAIClient`) contains
nearly identical JSON parsing logic. This is a copy-paste pattern.

**Impact:** Bug fixes or improvements to response handling must be applied to
all 4 providers independently. High maintenance burden.

**Fix:** Extract a shared `handle_response()` function or use a macro.

### Dead code: unused enum variants and struct fields

Several enum variants and struct fields are annotated with `#[allow(dead_code)]`
because they are defined for future use or serde deserialization but not yet
consumed:

- `ConfigError::MissingEnvVar` — defined but never constructed
- `RavenClawError::RavenFabric` / `RavenClawError::SecurityViolation` — future use
- `LLMError::ProviderNotSupported` — defined but never constructed
- Various serde-deserialized fields in `ChatResponse`, `Choice`, `Usage`
- `RavenFabricConfig` fields (`agent_id`, `remote_exec`, `allowed_hosts`)
- `SecurityConfig` fields (`token_lifetime_secs`, `audit_log`)
- `RuntimeConfig` fields (`workdir`, `max_agents`, `health_interval_secs`)

These should be cleaned up as features are implemented.

---

## 📋 Documentation

### ROADMAP.md v0.2 exit criteria not fully met

**Problem:** The v0.2 exit criteria states:
> `cargo fmt && cargo clippy -D warnings && cargo test` green; `docker buildx`
> produces working `amd64`+`arm64` images; fresh clone builds with `--locked`.

While `cargo fmt`, `clippy`, and `test` all pass, the Docker multi-arch build
and some CI workflows still fail (see Build & CI section above).

---

## 🔮 Future Considerations

### No graceful shutdown / signal handling

The binary does not handle SIGTERM/SIGINT. When running in interactive mode,
Ctrl+C will abort immediately without cleanup.

### No configuration hot-reload

Changes to `ravenclaw.toml` require a restart. No file-watch mechanism exists.
