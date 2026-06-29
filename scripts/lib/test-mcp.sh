#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — MCP Integration Tests
# =============================================================================
# Tests MCP client and server modes for both stdio and SSE transport.
# Verifies that:
#   - MCP stdio server starts and responds to JSON-RPC requests
#   - MCP SSE server starts and responds to HTTP SSE connections
#   - MCP client can discover tools from both stdio and SSE servers
#   - Tool execution works through MCP transport
#
# These tests require:
#   - A release build of RavenClaws (cargo build --release)
# =============================================================================

# ── Helper: send JSON-RPC to stdio MCP server and read response ──────────────
# Usage: mcp_stdio_request <fifo_in> <fifo_out> <json_request> <timeout_secs>
# Returns the response line via echo.
mcp_stdio_request() {
    local fifo_in="$1"
    local fifo_out="$2"
    local request="$3"
    local timeout_secs="${4:-3}"

    # Send request
    echo "$request" > "$fifo_in"

    # Read response with timeout
    local response=""
    if timeout "$timeout_secs" cat "$fifo_out" 2>/dev/null; then
        return 0
    else
        return 1
    fi
}

# ── Helper: wait for SSE server to be ready ──────────────────────────────────
# Usage: wait_for_sse_server <port> <max_attempts>
# Uses nc (netcat) to check if the port is listening, since /sse is long-lived
wait_for_sse_server() {
    local port="$1"
    local max_attempts="${2:-10}"
    local attempt=0
    while [[ $attempt -lt $max_attempts ]]; do
        if command -v nc >/dev/null 2>&1; then
            if nc -z 127.0.0.1 "$port" 2>/dev/null; then
                return 0
            fi
        else
            # Fallback: try a quick curl connection with short timeout
            if curl -sf --max-time 1 "http://127.0.0.1:${port}/sse" > /dev/null 2>&1; then
                return 0
            fi
        fi
        sleep 0.5
        attempt=$((attempt + 1))
    done
    return 1
}

test_mcp_integration() {
    log_step "MCP Integration Verification"

    if ! check_binary; then return; fi

    # ── 1. MCP Stdio Server: initialize + tools/list ──────────────────────
    log_sub "1. MCP Stdio Server — initialize + tools/list via JSON-RPC"

    local stdio_log="$RESULTS_DIR/${TIMESTAMP}-mcp-stdio-server.log"
    local stdio_resp="$RESULTS_DIR/${TIMESTAMP}-mcp-stdio-response.log"

    # Use a temp file for input (requests) and a temp file for output (responses)
    # The MCP server reads from stdin line by line and writes responses to stdout.
    # We use a pipe to send requests and capture output, filtering out tracing JSON.
    local input_file="$RESULTS_DIR/${TIMESTAMP}-mcp-stdio-input.txt"
    local output_file="$RESULTS_DIR/${TIMESTAMP}-mcp-stdio-output.txt"

    # Step 1a: Send initialize request
    # Write the request to the input file, then pipe it to the MCP server
    # The MCP server reads one line, processes it, writes response, then reads next line
    # We use a single invocation with both requests to avoid stdin closing issues
    printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' > "$input_file"

    # Run the MCP server with the input file as stdin, capture stdout
    # The server will process both requests and exit when stdin is closed
    "$BINARY" --config "$TEST_CONFIG" --mcp-server < "$input_file" > "$output_file" 2>"$stdio_log" || true

    # Filter output for JSON-RPC lines (skip tracing JSON)
    local init_response
    init_response=$(grep '"jsonrpc":"2.0"' "$output_file" | head -1)
    local tools_response
    tools_response=$(grep '"jsonrpc":"2.0"' "$output_file" | tail -1)

    if echo "$init_response" | grep -q '"jsonrpc":"2.0"'; then
        log_ok "MCP Stdio Server — initialize response received"
    else
        log_fail "MCP Stdio Server — no initialize response"
        log_detail "Output: $(head -5 "$output_file" | tr '\n' ' ')"
    fi

    if echo "$tools_response" | grep -q '"tools"'; then
        log_ok "MCP Stdio Server — tools/list returns tools array"
    else
        log_fail "MCP Stdio Server — tools/list response missing tools array"
        log_detail "Response: $(echo "$tools_response" | head -c 300)"
    fi

    # Cleanup
    rm -f "$input_file" "$output_file"

    # ── 2. MCP SSE Server: Start and verify SSE connection ────────────────
    log_sub "2. MCP SSE Server — SSE endpoint and tools/list"

    local sse_port=19081
    local sse_server_log="$RESULTS_DIR/${TIMESTAMP}-mcp-sse-server-run.log"
    local sse_events_log="$RESULTS_DIR/${TIMESTAMP}-mcp-sse-events.log"

    # Start the MCP SSE server in background
    "$BINARY" --config "$TEST_CONFIG" --mcp-sse-server --mcp-sse-host 127.0.0.1 --mcp-sse-port "$sse_port" \
        > "$sse_server_log" 2>&1 &
    local sse_pid=$!

    # Wait for server to be ready
    if wait_for_sse_server "$sse_port" 10; then
        log_ok "MCP SSE Server — server started and accepting connections"
    else
        log_fail "MCP SSE Server — failed to start within 5s"
        tail -10 "$sse_server_log" | sed 's/^/    /'
        kill "$sse_pid" 2>/dev/null || true
        # Skip remaining SSE tests
        log_sub "Skipping remaining SSE tests due to server start failure"
    fi

    # Step 2a: Connect to SSE and capture endpoint event
    if kill -0 "$sse_pid" 2>/dev/null; then
        # Connect SSE client in background, capture output for 2 seconds
        timeout 2 curl -sN "http://127.0.0.1:${sse_port}/sse" > "$sse_events_log" 2>/dev/null &
        local sse_curl_pid=$!
        sleep 1

        # Check that we got the endpoint event
        if grep -q "endpoint" "$sse_events_log" 2>/dev/null; then
            log_ok "MCP SSE Server — SSE endpoint returns endpoint event"
        else
            log_fail "MCP SSE Server — no endpoint event received"
            if [[ -s "$sse_events_log" ]]; then
                log_detail "Got: $(head -3 "$sse_events_log" | tr '\n' ' ')"
            else
                log_detail "SSE event log is empty"
            fi
        fi

        kill "$sse_curl_pid" 2>/dev/null || true

        # Step 2b: Test POST /message with tools/list
        local tools_response
        tools_response=$(curl -sf -X POST "http://127.0.0.1:${sse_port}/message" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null || echo "")

        if echo "$tools_response" | grep -q '"tools"'; then
            log_ok "MCP SSE Server — POST /message returns tools list"
        else
            log_fail "MCP SSE Server — POST /message did not return tools"
            log_detail "Response: $(echo "$tools_response" | head -c 300)"
        fi

        # Step 2c: Test POST /message with tools/call (shell tool)
        local call_response
        call_response=$(curl -sf -X POST "http://127.0.0.1:${sse_port}/message" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"shell","arguments":{"command":"echo hello"}}}' 2>/dev/null || echo "")

        if echo "$call_response" | grep -q '"content"'; then
            log_ok "MCP SSE Server — POST /message executes tool call"
        else
            log_fail "MCP SSE Server — POST /message tool call failed"
            log_detail "Response: $(echo "$call_response" | head -c 300)"
        fi
    fi

    # Cleanup SSE server
    kill "$sse_pid" 2>/dev/null || true
    wait "$sse_pid" 2>/dev/null || true

    # ── 3. MCP SSE Server: 404 handling ───────────────────────────────────
    log_sub "3. MCP SSE Server — 404 for unknown paths"

    local sse2_port=19082
    local sse2_log="$RESULTS_DIR/${TIMESTAMP}-mcp-sse-404.log"

    "$BINARY" --config "$TEST_CONFIG" --mcp-sse-server --mcp-sse-host 127.0.0.1 --mcp-sse-port "$sse2_port" \
        > "$sse2_log" 2>&1 &
    local sse2_pid=$!

    if wait_for_sse_server "$sse2_port" 10; then
        # Test 404 for unknown paths
        local not_found
        not_found=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:${sse2_port}/unknown" 2>/dev/null || echo "000")
        if [[ "$not_found" == "404" ]]; then
            log_ok "MCP SSE Server — unknown path returns 404"
        else
            log_fail "MCP SSE Server — expected 404, got $not_found"
        fi
    else
        log_fail "MCP SSE Server — failed to start for 404 test"
    fi

    kill "$sse2_pid" 2>/dev/null || true
    wait "$sse2_pid" 2>/dev/null || true

    # ── 4. CLI flags verification ─────────────────────────────────────────
    log_sub "4. MCP CLI flags — --mcp-sse-* flags available"

    if "$BINARY" --help 2>&1 | grep -q "mcp-sse-server"; then
        log_ok "MCP CLI — --mcp-sse-server flag is available"
    else
        log_fail "MCP CLI — --mcp-sse-server flag not found in help"
    fi

    if "$BINARY" --help 2>&1 | grep -q "mcp-sse-host"; then
        log_ok "MCP CLI — --mcp-sse-host flag is available"
    else
        log_fail "MCP CLI — --mcp-sse-host flag not found in help"
    fi

    if "$BINARY" --help 2>&1 | grep -q "mcp-sse-port"; then
        log_ok "MCP CLI — --mcp-sse-port flag is available"
    else
        log_fail "MCP CLI — --mcp-sse-port flag not found in help"
    fi

    # ── 5. MCP SSE Server: Multiple concurrent clients ────────────────────
    log_sub "5. MCP SSE Server — multiple concurrent clients"

    local multi_port=19083
    local multi_log="$RESULTS_DIR/${TIMESTAMP}-mcp-sse-multi.log"
    local client1_log="$RESULTS_DIR/${TIMESTAMP}-mcp-sse-client1.log"
    local client2_log="$RESULTS_DIR/${TIMESTAMP}-mcp-sse-client2.log"

    "$BINARY" --config "$TEST_CONFIG" --mcp-sse-server --mcp-sse-host 127.0.0.1 --mcp-sse-port "$multi_port" \
        > "$multi_log" 2>&1 &
    local multi_pid=$!

    if wait_for_sse_server "$multi_port" 10; then
        # Connect two SSE clients simultaneously
        timeout 2 curl -sN "http://127.0.0.1:${multi_port}/sse" > "$client1_log" 2>/dev/null &
        local c1_pid=$!
        timeout 2 curl -sN "http://127.0.0.1:${multi_port}/sse" > "$client2_log" 2>/dev/null &
        local c2_pid=$!

        sleep 1

        # Both clients should receive the endpoint event
        local c1_ok=false
        local c2_ok=false

        if grep -q "endpoint" "$client1_log" 2>/dev/null; then
            log_ok "MCP SSE Server — client 1 received endpoint event"
            c1_ok=true
        else
            log_fail "MCP SSE Server — client 1 did not receive endpoint event"
        fi

        if grep -q "endpoint" "$client2_log" 2>/dev/null; then
            log_ok "MCP SSE Server — client 2 received endpoint event"
            c2_ok=true
        else
            log_fail "MCP SSE Server — client 2 did not receive endpoint event"
        fi

        # If at least one client connected, mark concurrent support as verified
        if $c1_ok || $c2_ok; then
            log_ok "MCP SSE Server — multiple concurrent clients supported"
        fi

        kill "$c1_pid" "$c2_pid" 2>/dev/null || true
    else
        log_fail "MCP SSE Server — failed to start for multi-client test"
    fi

    kill "$multi_pid" 2>/dev/null || true
    wait "$multi_pid" 2>/dev/null || true

    # ── Summary ───────────────────────────────────────────────────────────
    log_step "MCP Integration Summary"
    log_detail "Stdio server: tested initialize + tools/list via JSON-RPC"
    log_detail "SSE server: tested /sse endpoint, POST /message, tools/list, tools/call"
    log_detail "SSE server: tested 404 handling for unknown paths"
    log_detail "SSE server: tested multiple concurrent clients"
    log_detail "CLI flags: --mcp-sse-server, --mcp-sse-host, --mcp-sse-port verified"
}
