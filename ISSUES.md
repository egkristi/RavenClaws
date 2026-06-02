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

### Container Build fails: `aquasecurity/trivy-action@0.29.0` not found

**Problem:** The Container Build workflow fails immediately with:
`Unable to resolve action 'aquasecurity/trivy-action@0.29.0', unable to find version '0.29.0'`

**Root cause:** The Trivy action version `0.29.0` does not exist or was retracted.
The workflow file pins an invalid version.

**Fix:** Updated `.github/workflows/container.yml`, `.github/workflows/build.yml`, and
`.github/workflows/security-scan.yml` to use `aquasecurity/trivy-action@v0.36.0`.

**Status:** ✅ Resolved — Trivy action updated to `v0.36.0` in all 3 workflows.

### Security Scan: `kubescape/action` repository not found

**Problem:** The K8s Manifest Validation job fails with:
`Unable to resolve action kubescape/action, repository not found`

**Root cause:** The Kubescape action repository may have been renamed, moved, or
removed. The workflow references `kubescape/action` which no longer resolves.

**Fix:** Updated `.github/workflows/security-scan.yml` to use `kubescape/github-action@main`
with updated parameters (`outputFile`, `severityThreshold`, `frameworks`).

**Status:** ✅ Resolved — Kubescape action migrated to `kubescape/github-action@main`.

**Status:** ✅ Resolved — Kubescape action migrated to `kubescape/github-action@main`.

### Container Images: RavenFabric agent download fails (exit code 22)

**Problem:** The Container Images job (in both `build.yml` and `container.yml`)
fails during Docker build with:
`process ... did not complete successfully: exit code: 22`

**Root cause:** The Dockerfile downloads `ravenfabric-linux-${RF_ARCH}-agent.sha256`
per-binary checksum file, but the RavenFabric-Published release only provides a
single `SHA256SUMS` file containing all checksums. The per-binary `.sha256` file
returns 404, causing curl to exit with code 22.

**Fix:** Updated `Dockerfile` to download `SHA256SUMS` and grep for the specific
binary's checksum instead of downloading a per-binary `.sha256` file.

**Status:** ✅ Resolved — Dockerfile now uses `SHA256SUMS` with grep filtering.

### Build (aarch64-unknown-linux-gnu): Cross-compilation fails (exit code 101)

**Problem:** The `Build (aarch64-unknown-linux-gnu)` job in `build.yml` fails
during `cargo build --release --locked --target aarch64-unknown-linux-gnu` with
exit code 101.

**Root cause:** Likely a cross-compilation issue with the `ring` crate (used by
`rustls` via `reqwest`). The `ring` crate uses assembly code and requires the
aarch64 cross-compiler toolchain (`gcc-aarch64-linux-gnu`) to be properly
configured. The `libssl-dev` package installed is for the host architecture
(x86_64) and may not help.

**Status:** ❌ Unresolved — needs investigation. Possible fixes:
- Add `cmake` and `go` to cross-compilation dependencies
- Verify `aarch64-linux-gnu-gcc` is correctly configured as the linker
- Consider using `ring` with the `wasm_c` feature for cross-compilation
- Fall back to `native-tls` with cross-compiled OpenSSL

### Security Scan: Cargo Udeps reports unused dependencies (exit code 101)

**Problem:** The `cargo-udeps` job exits with code 101, indicating unused
dependencies were found. The job itself succeeds (the tool ran), but the exit
code signals findings.

**Status:** ⚠️ Informational — job succeeds, exit code is a warning signal.
Needs review to determine if unused deps should be removed.

### Security Scan: Cargo Outdated reports outdated dependencies (exit code 1)

**Problem:** The `cargo-outdated` job exits with code 1, indicating outdated
dependencies exist. The job itself succeeds, but the exit code signals findings.

**Status:** ⚠️ Informational — job succeeds, exit code is a warning signal.
Needs periodic review to keep deps up to date.

### Security Scan: Cargo Deny exits with code 1

**Problem:** The `cargo-deny` job exits with code 1, indicating license or
security advisory violations were found in the dependency tree.

**Status:** ❌ Unresolved — needs investigation. Run `cargo deny check` locally
to see which advisories or licenses are flagged.

### Security Scan: Trivy (Filesystem) exits with code 1

**Problem:** The Trivy filesystem scan job exits with code 1, indicating
vulnerabilities were found in the workspace files.

**Status:** ❌ Unresolved — needs investigation. Likely false positives from
test fixtures or vendored files. May need a `.trivyignore` file.

### Security Scan: Trivy (IaC Config) exits with code 1

**Problem:** The Trivy IaC config scan job exits with code 1, indicating
misconfigurations were found in infrastructure-as-code files.

**Status:** ❌ Unresolved — needs investigation. May need to adjust severity
thresholds or add ignore rules.

### Security Scan: K8s Manifest Validation produces invalid SARIF

**Problem:** The K8s Manifest Validation job (Kubescape) produces an invalid
SARIF output file, causing the upload-sarif step to fail with:
`Invalid SARIF. JSON syntax error: Unexpected end of JSON input`

**Root cause:** The `kubescape/github-action@main` may produce an empty or
truncated SARIF file when no issues are found, or the `outputFile` parameter
may not be compatible with the SARIF upload format.

**Status:** ❌ Unresolved — needs investigation. Possible fixes:
- Add `continue-on-error: true` to the upload-sarif step
- Check if `outputFile` should include a `.sarif` extension
- Verify Kubescape action output format

### GitHub Actions: Node.js 20 deprecation warnings

**Problem:** Multiple workflow jobs emit warnings that Node.js 20 actions are
deprecated. Node.js 20 will be removed from the runner on September 16th, 2026.

**Affected actions:** `actions/checkout@v4`, `github/codeql-action/upload-sarif@v3`

**Fix:** Update affected actions to versions that support Node.js 24, or set
`FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` environment variable.

**Status:** ⚠️ Warning — not blocking, but needs attention before Sep 2026.

### GitHub Actions: CodeQL Action v3 deprecation (Dec 2026)

**Problem:** CodeQL Action v3 will be deprecated in December 2026.

**Fix:** Update all occurrences of `github/codeql-action/*@v3` to `@v4` in
workflow files.

**Status:** ⚠️ Warning — not blocking, but needs attention before Dec 2026.

### Container Build workflow may fail at "Set up job"

**Problem:** The `Container Images` job in `.github/workflows/build.yml` may fail
intermittently at the "Set up job" step with an infrastructure error.

**Root cause:** GitHub Actions runner provisioning issue — not a code defect.
The workflow uses `docker/build-push-action@v6` with multi-arch (`linux/amd64`,
`linux/arm64`) and requires QEMU + Buildx setup.

**Frequency:** Intermittent — depends on GitHub Actions runner availability.

**Workaround:** Re-run the workflow manually. If it persists, investigate GitHub
Actions runner availability for the repository.

**Status:** ✅ Dockerfile now has cross-linkers (`gcc-aarch64-linux-gnu`,
`gcc-x86_64-linux-gnu`) and SHA256 checksum verification for RavenFabric agent.
CI `build.yml` now installs `musl-tools` and `gcc-aarch64-linux-gnu` for
cross-compilation targets.

### Security Scan workflow may fail

**Problem:** The `Security Scan` workflow (`.github/workflows/security-scan.yml`)
may fail due to:
- `cargo-outdated` exit code 1 when dependencies are outdated (non-blocking, informational)
- `cargo-udeps` detecting unused dependencies (non-blocking, informational)
- Trivy scanner finding MEDIUM severity issues in dependencies
- Kubescape threshold violations on K8s manifests

**Root cause:** These are informational scans configured with `continue-on-error: true`
or lenient thresholds. Failures are expected for some scans and do not block the pipeline.

**Status:** All scans are configured. CodeQL, cargo-audit, cargo-deny, Hadolint,
and OSSF Scorecard are blocking. Trivy and Kubescape may produce findings that
need periodic review.

---

## 🧪 Code Quality

### ~~`next_client()` round-robin method never called~~ ✅ Fixed

**Problem:** `MultiModelManager::next_client()` in `src/llm.rs` implements
round-robin load balancing across providers, but was never invoked anywhere in
the codebase.

**Fix:** Changed return type to `Option`, removed `#[allow(dead_code)]`, wired
into `run_single_multi()` in agent.rs. Added 2 new tests.

### ~~`handle_response()` code duplicated across providers~~ ✅ Fixed

**Problem:** The `handle_response()` method in each LLM client contained nearly
identical JSON parsing logic.

**Fix:** Extracted shared `handle_openai_response()` async function. Replaced
duplicated code in LiteLLM, OpenRouter, and OpenAI clients. Ollama kept its own
handler (different API format).

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

## ✅ Resolved Issues

### Linux cross-compilation builds fail (RESOLVED)

**Fix:** CI `build.yml` now installs `musl-tools` and `gcc-aarch64-linux-gnu`
before building cross-compilation targets. Dockerfile has cross-linkers configured
for multi-arch builds. SHA256 checksum verification added for RavenFabric agent download.

### ROADMAP.md v0.2 exit criteria (RESOLVED)

All v0.2 items are complete:
- ✅ `Cargo.lock` committed, `--locked` works everywhere
- ✅ Multi-arch Docker build fixed (cross-linkers installed)
- ✅ RavenFabric agent download verified with SHA256 checksum
- ✅ `--version` wired to `CARGO_PKG_VERSION`
- ✅ `.expect()` on HTTP client replaced with error propagation
- ✅ `--exec` one-shot mode implemented
- ✅ Swarm/supervisor stubs return clear errors
- ✅ Tests expanded to 149 across all modules with `mockito`
- ✅ `cargo fmt && cargo clippy -D warnings && cargo test` all green

---

## 🔮 Future Considerations

### No graceful shutdown / signal handling

The binary does not handle SIGTERM/SIGINT. When running in interactive mode,
Ctrl+C will abort immediately without cleanup.

### No configuration hot-reload

Changes to `ravenclaw.toml` require a restart. No file-watch mechanism exists.
