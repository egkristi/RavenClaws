#!/usr/bin/env bash
# RavenClaws Demo 3: Server & MCP Modes
# Shows: HTTP server (health/ready/metrics), MCP server, MCP SSE server
#
# Record:
#   asciinema rec --title "RavenClaws v1.2.0 — Server & MCP" \
#     --command "./scripts/demos/demo-server-mcp.sh" \
#     --overwrite demos/demo-server-mcp.cast
#
# Run directly:
#   ./scripts/demos/demo-server-mcp.sh

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

cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

CONFIG_FLAG=""
[ -f "tests/config/ravenclaws-test.toml" ] && CONFIG_FLAG="-c tests/config/ravenclaws-test.toml"

# ── Demo ────────────────────────────────────────────────────────────────────

section "1. HTTP SERVER MODE"

# Start server in background
"$BINARY" $CONFIG_FLAG --serve --server-port 9877 > /dev/null 2>&1 &
SERVER_PID=$!
for i in $(seq 1 10); do
  curl -sf http://localhost:9877/health > /dev/null 2>&1 && break
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

# Stop server for next demo
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
sleep 0.5

section "2. MCP SERVER MODE (stdio)"

type_cmd "$BINARY" --mcp-server --help 2>&1 | head -5
sleep 0.5
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | timeout 3 "$BINARY" $CONFIG_FLAG --mcp-server 2>/dev/null | head -5 || \
  echo "(MCP server started — exposes RavenClaws tools over stdio)"
sleep 1

section "3. MCP SSE SERVER MODE (HTTP transport)"

type_cmd "$BINARY" --mcp-sse-server --help 2>&1 | head -5
sleep 0.5
echo "(MCP SSE server exposes tools over HTTP with SSE transport on port 8081)"
echo "  --mcp-sse-host <HOST>  default: 0.0.0.0"
echo "  --mcp-sse-port <PORT>  default: 8081"
sleep 1

section "4. CLI FLAGS OVERVIEW"

type_cmd "$BINARY" --help 2>&1 | grep -E 'serve|mcp|server|port|host' | head -15
sleep 1

# ── Done ─────────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  RavenClaws v1.2.0 — Server & MCP Demo Complete            ║"
echo "║  https://ravenclaws.io                                      ║"
echo "║  https://github.com/egkristi/RavenClaws                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo
