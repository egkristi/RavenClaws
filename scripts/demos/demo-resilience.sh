#!/usr/bin/env bash
# RavenClaws Demo 4: Resilience & Self-Healing
# Shows: self-healing engine internals, circuit breakers, failure tracking,
#        graceful degradation, load management, retry with backoff
#
# Record:
#   asciinema rec --title "RavenClaws v1.2.0 — Resilience" \
#     --command "./scripts/demos/demo-resilience.sh" \
#     --overwrite demos/demo-resilience.cast
#
# Run directly:
#   ./scripts/demos/demo-resilience.sh

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

section "1. SELF-HEALING ENGINE OVERVIEW"

type_cmd cat src/healing.rs | head -30
sleep 1

section "2. CIRCUIT BREAKER STATES"

type_cmd grep -A 10 'pub enum HealingCircuitState' src/healing.rs
sleep 1

section "3. SELF-HEALING ENGINE STRUCT"

type_cmd grep -A 15 'pub struct SelfHealingEngine' src/healing.rs
sleep 1

section "4. FAILURE RECORD TRACKING"

type_cmd grep -A 15 'pub struct FailureRecord' src/healing.rs
sleep 1

section "5. RETRY WITH EXPONENTIAL BACKOFF"

type_cmd grep -A 20 'pub async fn retry_with_backoff' src/healing.rs | head -20
sleep 1

section "6. GRACEFUL DEGRADATION"

type_cmd grep -A 10 'pub struct LoadManager' src/load.rs
sleep 0.5
type_cmd grep -A 10 'pub struct TokenBucket' src/load.rs
sleep 0.5
type_cmd grep -A 10 'pub struct LoadConfig' src/load.rs
sleep 1

section "7. SELF-HEALING UNIT TESTS"

type_cmd grep -A 5 '#\[test\]' src/healing.rs | head -30
sleep 1

# ── Done ─────────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  RavenClaws v1.2.0 — Resilience Demo Complete              ║"
echo "║  https://ravenclaws.io                                      ║"
echo "║  https://github.com/egkristi/RavenClaws                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo
