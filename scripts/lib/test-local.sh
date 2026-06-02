#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — Local macOS Binary Tests
# =============================================================================
# Tests the native macOS binary: basic operation, config loading,
# single-agent LLM chat, multi-model mode, CLI overrides, error handling.
# =============================================================================

test_local_binary() {
    log_step "2. Local macOS Binary Verification"

    if ! check_binary; then return; fi

    # ── Basic binary checks ────────────────────────────────────────────────
    log_sub "Basic binary checks"
    run_test "Binary exists and is executable" test -x "$BINARY"
    run_test "Binary --version" "$BINARY" --version
    run_test "Binary --help" "$BINARY" --help

    # ── Configuration loading ──────────────────────────────────────────────
    log_sub "Configuration loading"
    run_test "Config loading (TOML file)" "$BINARY" --config "$TEST_CONFIG" --version
    run_test "Config loading (env vars)" env \
        RAVENCLAW__LLM__ENDPOINT="http://localhost:4000" \
        RAVENCLAW__LLM__MODEL="best-coding" \
        RAVENCLAW__SECURITY__REQUIRE_TLS=false \
        "$BINARY" --version

    # ── Agent modes ────────────────────────────────────────────────────────
    log_sub "Agent modes"
    
    # Single agent mode (single provider)
    if check_litellm; then
        log_sub "Testing single agent mode (single provider)..."
        if timeout 45 "$BINARY" --config "$TEST_CONFIG" --mode single \
            > "$RESULTS_DIR/${TIMESTAMP}-local-single-agent.log" 2>&1; then
            check_llm_response_quality "$RESULTS_DIR/${TIMESTAMP}-local-single-agent.log" "best-coding (single)"
        else
            log_fail "Single agent mode — exited with error"
            tail -10 "$RESULTS_DIR/${TIMESTAMP}-local-single-agent.log" | sed 's/^/    /'
        fi
    fi

    # ── Multi-model mode ───────────────────────────────────────────────────
    if check_litellm; then
        log_sub "Testing multi-model mode (3 providers)..."
        if timeout 90 "$BINARY" --config "$MULTI_CONFIG" --mode single \
            > "$RESULTS_DIR/${TIMESTAMP}-local-multi-model.log" 2>&1; then
            log_ok "Multi-model mode — all providers responded"
            # Count provider responses
            local resp_count
            resp_count=$(grep -c '"Provider response received"' "$RESULTS_DIR/${TIMESTAMP}-local-multi-model.log" 2>/dev/null || echo 0)
            log_detail "Provider responses: ${resp_count}"
        else
            log_fail "Multi-model mode — check log"
            tail -10 "$RESULTS_DIR/${TIMESTAMP}-local-multi-model.log" | sed 's/^/    /'
        fi
    fi

    # ── CLI overrides ──────────────────────────────────────────────────────
    log_sub "CLI overrides"
    run_test "CLI --provider override" "$BINARY" \
        --config "$TEST_CONFIG" \
        --provider litellm \
        --endpoint "http://localhost:4000" \
        --model "best-coding" \
        --version

    # ── Verbose logging ────────────────────────────────────────────────────
    log_sub "Logging levels"
    run_test "Verbose logging (--verbose)" "$BINARY" --config "$TEST_CONFIG" --verbose --version

    # ── Error handling ─────────────────────────────────────────────────────
    log_sub "Error handling"
    run_test "Error: missing config file" bash -c '"$BINARY" --config /nonexistent/path 2>&1; [[ $? -ne 0 ]]'
    run_test "Error: invalid mode" bash -c '"$BINARY" --config "$TEST_CONFIG" --mode invalid 2>&1; [[ $? -ne 0 ]]'

    # ── One-shot exec mode ─────────────────────────────────────────────────
    log_sub "One-shot execution"
    if check_litellm; then
        log_sub "Testing --exec mode..."
        if timeout 45 "$BINARY" --config "$TEST_CONFIG" --exec "Hello, what is 2+2?" \
            > "$RESULTS_DIR/${TIMESTAMP}-local-exec.log" 2>&1; then
            check_llm_response_quality "$RESULTS_DIR/${TIMESTAMP}-local-exec.log" "best-coding (exec)"
        else
            log_fail "--exec mode — exited with error"
            tail -10 "$RESULTS_DIR/${TIMESTAMP}-local-exec.log" | sed 's/^/    /'
        fi
    fi
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_local_binary
    print_summary "Local macOS Binary"
fi
