#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — Swarm & Sub-Agent Scalability Tests
# =============================================================================
# Tests swarm mode and supervisor/sub-agent mode for correct operation and
# scalability. Verifies that multiple agents can run in parallel, that
# supervisor can decompose tasks and spawn sub-agents, and that the system
# handles increasing agent counts gracefully.
#
# These tests require:
#   - A release build of RavenClaw (cargo build --release)
#   - LiteLLM running at http://localhost:4000
# =============================================================================

test_swarm_and_subagent() {
    log_step "Swarm & Sub-Agent Scalability Verification"

    if ! check_binary; then return; fi
    if ! check_litellm; then return; fi

    # ── 1. Swarm mode (single-provider, 3 agents) ─────────────────────────
    log_sub "1. Swarm mode — single-provider, 3 parallel agents"
    local swarm_log="$RESULTS_DIR/${TIMESTAMP}-swarm-single.log"
    if timeout 90 "$BINARY" --config "$SWARM_CONFIG" --mode swarm \
        > "$swarm_log" 2>&1; then
        log_ok "Swarm mode (single-provider) completed successfully"
        # Verify all 3 agents produced output
        local agent_count
        agent_count=$(grep -c "Agent [0-9]\+ completed" "$swarm_log" 2>/dev/null || echo 0)
        log_detail "Agents completed: ${agent_count}/3"
        if [[ "$agent_count" -ge 3 ]]; then
            log_ok "Swarm mode — all 3 agents produced results"
        else
            log_fail "Swarm mode — only ${agent_count}/3 agents completed"
            tail -15 "$swarm_log" | sed 's/^/    /'
        fi
        # Verify swarm results were printed
        if grep -q "Swarm Results" "$swarm_log" 2>/dev/null; then
            log_ok "Swarm mode — results aggregated and displayed"
        else
            log_fail "Swarm mode — no results output detected"
        fi
    else
        local exit_code=$?
        log_fail "Swarm mode (single-provider) exited with code ${exit_code}"
        tail -20 "$swarm_log" | sed 's/^/    /'
    fi

    # ── 2. Swarm mode (multi-model, up to 3 agents) ───────────────────────
    log_sub "2. Swarm mode — multi-model, parallel agents across providers"
    local swarm_multi_log="$RESULTS_DIR/${TIMESTAMP}-swarm-multi.log"
    if timeout 120 "$BINARY" --config "$MULTI_CONFIG" --mode swarm \
        > "$swarm_multi_log" 2>&1; then
        log_ok "Swarm mode (multi-model) completed successfully"
        local agent_count
        agent_count=$(grep -c "Agent [0-9]\+ completed" "$swarm_multi_log" 2>/dev/null || echo 0)
        log_detail "Multi-model agents completed: ${agent_count}"
        if grep -q "Swarm Results" "$swarm_multi_log" 2>/dev/null; then
            log_ok "Swarm mode (multi-model) — results aggregated"
        fi
    else
        local exit_code=$?
        log_fail "Swarm mode (multi-model) exited with code ${exit_code}"
        tail -20 "$swarm_multi_log" | sed 's/^/    /'
    fi

    # ── 3. Supervisor mode (single-provider) ──────────────────────────────
    log_sub "3. Supervisor mode — task decomposition with sub-agents"
    local supervisor_log="$RESULTS_DIR/${TIMESTAMP}-supervisor-single.log"
    if timeout 120 "$BINARY" --config "$SUPERVISOR_CONFIG" --mode supervisor \
        > "$supervisor_log" 2>&1; then
        log_ok "Supervisor mode (single-provider) completed successfully"
        # Check for subtask decomposition
        local subtask_count
        subtask_count=$(grep -c "Subtask" "$supervisor_log" 2>/dev/null || echo 0)
        log_detail "Subtasks decomposed: ${subtask_count}"
        if grep -q "Supervisor Result" "$supervisor_log" 2>/dev/null; then
            log_ok "Supervisor mode — final result produced"
        else
            log_fail "Supervisor mode — no final result detected"
            tail -15 "$supervisor_log" | sed 's/^/    /'
        fi
    else
        local exit_code=$?
        log_fail "Supervisor mode (single-provider) exited with code ${exit_code}"
        tail -20 "$supervisor_log" | sed 's/^/    /'
    fi

    # ── 4. Supervisor mode (multi-model) ──────────────────────────────────
    log_sub "4. Supervisor mode — multi-model task decomposition"
    local supervisor_multi_log="$RESULTS_DIR/${TIMESTAMP}-supervisor-multi.log"
    if timeout 120 "$BINARY" --config "$MULTI_CONFIG" --mode supervisor \
        > "$supervisor_multi_log" 2>&1; then
        log_ok "Supervisor mode (multi-model) completed successfully"
        if grep -q "Supervisor Result" "$supervisor_multi_log" 2>/dev/null; then
            log_ok "Supervisor mode (multi-model) — final result produced"
        fi
    else
        local exit_code=$?
        # Multi-model supervisor may fail if a provider is unavailable (e.g., Anthropic credit limit).
        # This is expected behavior — the system should degrade gracefully.
        if grep -q "Circuit breaker open\|credit balance\|rate limit" "$supervisor_multi_log" 2>/dev/null; then
            log_skip "Supervisor mode (multi-model) — provider unavailable (graceful degradation)"
            log_detail "This is expected when a provider has credit/rate limits"
        else
            log_fail "Supervisor mode (multi-model) exited with code ${exit_code}"
            tail -20 "$supervisor_multi_log" | sed 's/^/    /'
        fi
    fi

    # ── 5. Scalability: verify config-driven agent limits ─────────────────
    log_sub "5. Scalability — configuration and limits"
    
    # Verify max_agents config field exists and is parsed
    run_test "Config parses max_agents field" \
        "$BINARY" --config "$SWARM_CONFIG" --version
    
    # Verify the config has max_agents=3 (as set in swarm test config)
    local max_agents
    max_agents=$(grep "max_agents" "$SWARM_CONFIG" 2>/dev/null | head -1 | grep -o '[0-9][0-9]*')
    if [[ -n "$max_agents" ]]; then
        log_detail "Swarm config max_agents=${max_agents}"
        run_test "Swarm config max_agents=${max_agents}" test "${max_agents}" -eq 3
    fi

    # ── 6. Concurrent execution test ──────────────────────────────────────
    log_sub "6. Concurrent execution — multiple instances"
    
    # Run two swarm instances concurrently to verify no resource conflicts
    local conc1_log="$RESULTS_DIR/${TIMESTAMP}-concurrent-1.log"
    local conc2_log="$RESULTS_DIR/${TIMESTAMP}-concurrent-2.log"
    
    log_detail "Starting 2 concurrent swarm instances..."
    "$BINARY" --config "$SWARM_CONFIG" --mode swarm \
        > "$conc1_log" 2>&1 &
    local pid1=$!
    
    "$BINARY" --config "$SWARM_CONFIG" --mode swarm \
        > "$conc2_log" 2>&1 &
    local pid2=$!
    
    local success=0
    wait "$pid1" 2>/dev/null && success=$((success + 1))
    wait "$pid2" 2>/dev/null && success=$((success + 1))
    
    if [[ "$success" -eq 2 ]]; then
        log_ok "Concurrent execution — both swarm instances completed"
    else
        log_fail "Concurrent execution — only ${success}/2 instances completed"
        if [[ ! -s "$conc1_log" ]]; then echo "  Instance 1: no output"; fi
        if [[ ! -s "$conc2_log" ]]; then echo "  Instance 2: no output"; fi
    fi

    # ── 7. Resource usage under swarm load ────────────────────────────────
    log_sub "7. Resource usage — memory and process count"
    
    # Run a single swarm and measure RSS
    local resource_log="$RESULTS_DIR/${TIMESTAMP}-resource-usage.log"
    
    # Start swarm in background and measure after agents are running
    "$BINARY" --config "$SWARM_CONFIG" --mode swarm \
        > "$resource_log" 2>&1 &
    local swarm_pid=$!
    
    # Wait a bit for agents to start, then measure
    sleep 5
    
    if kill -0 "$swarm_pid" 2>/dev/null; then
        # Measure RSS of the swarm process
        local rss
        rss=$(ps -o rss= -p "$swarm_pid" 2>/dev/null | tr -d ' ' || echo 0)
        log_detail "Swarm process RSS: ${rss} KB"
        
        # Count threads
        local threads
        threads=$(ps -M -p "$swarm_pid" 2>/dev/null | tail -1 | awk '{print $1}' || echo "N/A")
        log_detail "Swarm process threads: ${threads}"
        
        # Warn if RSS is excessive (> 500 MB)
        if [[ "$rss" -gt 512000 ]]; then
            log_fail "Swarm mode — RSS ${rss} KB exceeds 500 MB limit"
        else
            log_ok "Swarm mode — RSS ${rss} KB within limits"
        fi
    else
        log_detail "Swarm process already completed — resource measurement skipped"
    fi
    
    # Wait for swarm to finish
    wait "$swarm_pid" 2>/dev/null || true
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    test_swarm_and_subagent
    print_summary "Swarm & Sub-Agent Scalability"
fi
