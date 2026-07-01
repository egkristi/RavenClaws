#!/usr/bin/env bash
# RavenClaws asciinema demo script
#
# Records a terminal demo showcasing RavenClaws features.
# Usage:
#   asciinema rec --title "RavenClaws v1.1.0 Demo" \
#     --command "./scripts/demo.sh" \
#     --overwrite demo.cast
#
# Or run directly (no recording):
#   ./scripts/demo.sh

set -euo pipefail

BINARY="./target/release/ravenclaws"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# ── Helpers ─────────────────────────────────────────────────────────────────
type_cmd() {
  echo "\$ $*"
  "$@"
}

section() {
  echo
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║  $1"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo
}

# Detect if an LLM provider is available (liteLLM on localhost:4000)
LLM_AVAILABLE=false
if curl -sf http://localhost:4000/health >/dev/null 2>&1; then
  LLM_AVAILABLE=true
fi

CONFIG_FLAG=""
if [ -f "ravenclaws.toml" ]; then
  CONFIG_FLAG=""
elif [ -f "tests/config/ravenclaws-test.toml" ]; then
  CONFIG_FLAG="-c tests/config/ravenclaws-test.toml"
fi

# ── Demo ────────────────────────────────────────────────────────────────────

section "1. VERSION & HELP"

type_cmd "$BINARY" --version
sleep 0.5
type_cmd "$BINARY" --help | head -30
echo "  ... (36 more options, use --help to see all)"
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

section "4. SOURCE MODULES"

type_cmd ls src/*.rs | wc -l
sleep 0.3
type_cmd grep -c '^pub fn\|^pub async fn\|^pub struct\|^pub enum\|^pub trait' src/*.rs | sort -t: -k2 -rn | head -15
sleep 0.5
type_cmd wc -l src/*.rs | sort -rn | head -10
sleep 1

section "5. TEST SUITE"

type_cmd cargo test --locked 2>&1 | grep 'test result' | tail -3
sleep 1

section "6. ONE-SHOT EXEC MODE"

if [ "$LLM_AVAILABLE" = true ]; then
  echo '  Prompt: "Write a haiku about Rust."'
  echo "  Response:"
  echo '  "Write a haiku about Rust."' | "$BINARY" $CONFIG_FLAG --exec "Write a haiku about Rust." 2>/dev/null | tail -1
else
  echo "(LLM provider not available — showing config validation instead)"
  type_cmd "$BINARY" $CONFIG_FLAG --exec "Write a haiku about Rust." 2>&1 | head -10
fi
sleep 1

section "7. HTTP SERVER MODE"

# Start server in background
"$BINARY" $CONFIG_FLAG --serve --server-port 9877 > /dev/null 2>&1 &
SERVER_PID=$!
# Wait for server to be ready (up to 10 seconds)
for i in $(seq 1 10); do
  if curl -sf http://localhost:9877/health > /dev/null 2>&1; then
    break
  fi
  sleep 1
done

type_cmd curl -s http://localhost:9877/health
echo
sleep 0.5
type_cmd curl -s http://localhost:9877/ready
echo
sleep 0.5
type_cmd curl -s http://localhost:9877/metrics 2>/dev/null | head -20
sleep 0.5

# Stop server
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
sleep 0.5

section "8. MCP SERVER MODE"

type_cmd "$BINARY" --mcp-server --help 2>&1 | head -5
sleep 0.5
# Show MCP server can list tools
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | timeout 3 "$BINARY" $CONFIG_FLAG --mcp-server 2>/dev/null | head -5 || \
  echo "(MCP server started — use 'ravenclaws --mcp-server' to expose tools over stdio)"
sleep 1

section "9. DOCKER & DEPLOYMENT"

type_cmd cat Dockerfile | head -20
sleep 0.5
type_cmd head -30 k8s/deployment.yaml
sleep 1

section "10. WEBSITE"

type_cmd ls -la website/public/index.html website/public/assets/styles.css website/public/assets/main.js
sleep 0.3
type_cmd grep -c '<section' website/public/index.html
sleep 0.3
type_cmd grep 'softwareVersion' website/public/index.html
sleep 0.3
type_cmd head -30 website/public/_headers
sleep 1

section "11. VERIFICATION SUITE"

type_cmd ls scripts/lib/*.sh | wc -l
sleep 0.3
type_cmd head -30 scripts/verify.sh
sleep 1

# ── Done ─────────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  RavenClaws v1.1.0 — Demo Complete                         ║"
echo "║  https://ravenclaws.io                                      ║"
echo "║  https://github.com/egkristi/RavenClaws                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo
