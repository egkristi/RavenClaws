#!/usr/bin/env bash
# RavenClaws Demo 1: Quickstart
# Shows: version, help, binary profile, config, one-shot exec mode
#
# Record:
#   asciinema rec --title "RavenClaws v1.2.0 — Quickstart" \
#     --command "./scripts/demos/demo-quickstart.sh" \
#     --overwrite demos/demo-quickstart.cast
#
# Run directly:
#   ./scripts/demos/demo-quickstart.sh

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

LLM_AVAILABLE=false
curl -sf http://localhost:4000/health >/dev/null 2>&1 && LLM_AVAILABLE=true

CONFIG_FLAG=""
[ -f "tests/config/ravenclaws-test.toml" ] && CONFIG_FLAG="-c tests/config/ravenclaws-test.toml"

# ── Demo ────────────────────────────────────────────────────────────────────

section "1. VERSION & HELP"

type_cmd "$BINARY" --version
sleep 0.5
type_cmd "$BINARY" --help | head -20
echo "  ... (50+ more options, use --help to see all)"
sleep 2

section "2. BINARY PROFILE"

type_cmd ls -lh "$BINARY"
sleep 0.3
type_cmd file "$BINARY"
sleep 0.3
type_cmd otool -L "$BINARY"
sleep 1

section "3. CONFIGURATION"

type_cmd cat tests/config/ravenclaws-test.toml
sleep 0.5
type_cmd cat tests/config/ravenclaws-multi-test.toml
sleep 1

section "4. ONE-SHOT EXEC MODE"

if [ "$LLM_AVAILABLE" = true ]; then
  echo '  Prompt: "Write a haiku about Rust."'
  echo "  Response:"
  echo '  "Write a haiku about Rust."' | "$BINARY" $CONFIG_FLAG --exec "Write a haiku about Rust." 2>/dev/null | tail -1
else
  echo "(LLM provider not available — showing config validation instead)"
  type_cmd "$BINARY" $CONFIG_FLAG --exec "Write a haiku about Rust." 2>&1 | head -10
fi
sleep 1

# ── Done ─────────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  RavenClaws v1.2.0 — Quickstart Demo Complete              ║"
echo "║  https://ravenclaws.io                                      ║"
echo "║  https://github.com/egkristi/RavenClaws                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo
