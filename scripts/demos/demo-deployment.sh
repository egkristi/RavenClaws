#!/usr/bin/env bash
# RavenClaws Demo 5: Deployment & Operations
# Shows: Dockerfile, K8s manifests, Helm chart, website, verification suite
#
# Record:
#   asciinema rec --title "RavenClaws v1.2.0 — Deployment" \
#     --command "./scripts/demos/demo-deployment.sh" \
#     --overwrite demos/demo-deployment.cast
#
# Run directly:
#   ./scripts/demos/demo-deployment.sh

set -euo pipefail

BINARY="./target/release/ravenclaws"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

# ── Helpers ─────────────────────────────────────────────────────────────────
type_cmd() { echo "\$ $*"; "$@"; }
section() {
  echo; echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║  $1"; echo "╚══════════════════════════════════════════════════════════════╝"; echo
}

# ── Demo ────────────────────────────────────────────────────────────────────

section "1. DOCKER MULTI-STAGE BUILD"

type_cmd cat Dockerfile | head -25
sleep 1

section "2. DISTROLESS SECURITY"

type_cmd grep -E 'distroless|nonroot|USER|65532' Dockerfile
sleep 0.5
echo "(No shell, no package manager — minimal attack surface)"
sleep 1

section "3. KUBERNETES DEPLOYMENT"

type_cmd head -40 k8s/deployment.yaml
sleep 1

section "4. HELM CHART"

type_cmd ls charts/ravenclaws/
sleep 0.3
type_cmd head -20 charts/ravenclaws/values.yaml
sleep 1

section "5. WEBSITE"

type_cmd ls -la website/public/index.html website/public/assets/styles.css website/public/assets/main.js
sleep 0.3
type_cmd grep -c '<section' website/public/index.html
echo "  sections on the landing page"
sleep 0.3
type_cmd grep 'softwareVersion' website/public/index.html
sleep 0.3
type_cmd head -20 website/public/_headers
sleep 1

section "6. VERIFICATION SUITE"

type_cmd ls scripts/lib/*.sh | wc -l
echo "  verification test modules"
sleep 0.3
type_cmd head -25 scripts/verify.sh
sleep 1

section "7. GIT HOOKS"

type_cmd ls .githooks/
sleep 0.3
type_cmd head -20 .githooks/pre-commit
sleep 1

# ── Done ─────────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  RavenClaws v1.2.0 — Deployment Demo Complete              ║"
echo "║  https://ravenclaws.io                                      ║"
echo "║  https://github.com/egkristi/RavenClaws                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo
