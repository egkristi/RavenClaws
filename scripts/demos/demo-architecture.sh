#!/usr/bin/env bash
# RavenClaws Demo 2: Architecture & Code
# Shows: source modules, test suite, multi-agent patterns, self-healing, load
#
# Record:
#   asciinema rec --title "RavenClaws v1.2.0 — Architecture" \
#     --command "./scripts/demos/demo-architecture.sh" \
#     --overwrite demos/demo-architecture.cast
#
# Run directly:
#   ./scripts/demos/demo-architecture.sh

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

section "1. SOURCE MODULES (22 modules)"

type_cmd ls src/*.rs | wc -l
sleep 0.3
type_cmd grep -c '^pub fn\|^pub async fn\|^pub struct\|^pub enum\|^pub trait' src/*.rs | sort -t: -k2 -rn | head -15
sleep 0.5
type_cmd wc -l src/*.rs | sort -rn | head -10
sleep 1

section "2. TEST SUITE (552+ unit tests)"

type_cmd cargo test --locked 2>&1 | grep 'test result' | tail -3
sleep 1

section "3. MULTI-AGENT PATTERNS"

type_cmd grep -c '^pub async fn run_' src/patterns.rs
echo "  Patterns: debate, review-loop, research-synthesize, voting"
echo "  Each available in single-provider and multi-model variants"
sleep 0.5
type_cmd grep -A 2 'pub struct PatternConfig' src/patterns.rs
echo "..."
sleep 0.5
type_cmd grep -A 2 'pub async fn run_debate' src/patterns.rs
echo "..."
sleep 1

section "4. SELF-HEALING ENGINE"

type_cmd grep -c 'pub fn\|pub struct\|pub enum\|pub trait' src/healing.rs
echo "  28 public items in the self-healing module"
sleep 0.3
type_cmd grep -A 4 'pub struct SelfHealingEngine' src/healing.rs
echo "..."
sleep 0.5
type_cmd grep -A 4 'pub enum HealingCircuitState' src/healing.rs
echo "..."
sleep 0.5
type_cmd grep -c '#\[test\]' src/healing.rs
echo "  22 unit tests for self-healing engine"
sleep 1

section "5. GRACEFUL DEGRADATION"

type_cmd grep -c 'pub fn\|pub struct\|pub enum' src/load.rs
echo "  19 public items in the load management module"
sleep 0.3
type_cmd grep -A 4 'pub struct LoadManager' src/load.rs
echo "..."
sleep 0.5
type_cmd grep -A 4 'pub struct TokenBucket' src/load.rs
echo "..."
sleep 0.5
type_cmd grep -c '#\[test\]' src/load.rs
echo "  12 unit tests for graceful degradation"
sleep 1

# ── Done ─────────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  RavenClaws v1.2.0 — Architecture Demo Complete            ║"
echo "║  https://ravenclaws.io                                      ║"
echo "║  https://github.com/egkristi/RavenClaws                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo
