#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — Performance Benchmarks
# =============================================================================
# Measures startup time, config loading time, LLM response time,
# binary size, and memory usage.
# =============================================================================

test_performance() {
    log_step "7. Performance Benchmarks"

    if ! check_binary; then return; fi

    # ── Startup time ───────────────────────────────────────────────────────
    log_sub "Startup time"
    local start end elapsed
    start=$(date +%s%N)
    "$BINARY" --version >/dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    
    if [[ "$elapsed" -lt 20 ]]; then
        log_ok "Startup time: ${elapsed}ms (blazing fast)"
    elif [[ "$elapsed" -lt 50 ]]; then
        log_ok "Startup time: ${elapsed}ms (very fast)"
    elif [[ "$elapsed" -lt 100 ]]; then
        log_ok "Startup time: ${elapsed}ms (fast)"
    elif [[ "$elapsed" -lt 500 ]]; then
        log_ok "Startup time: ${elapsed}ms (acceptable)"
    else
        log_fail "Startup time: ${elapsed}ms (slow)"
    fi

    # ── Config loading time ────────────────────────────────────────────────
    log_sub "Config loading time"
    start=$(date +%s%N)
    "$BINARY" --config "$TEST_CONFIG" --version >/dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    
    if [[ "$elapsed" -lt 20 ]]; then
        log_ok "Config loading: ${elapsed}ms (blazing fast)"
    elif [[ "$elapsed" -lt 50 ]]; then
        log_ok "Config loading: ${elapsed}ms (very fast)"
    elif [[ "$elapsed" -lt 100 ]]; then
        log_ok "Config loading: ${elapsed}ms (fast)"
    elif [[ "$elapsed" -lt 200 ]]; then
        log_ok "Config loading: ${elapsed}ms (acceptable)"
    else
        log_fail "Config loading: ${elapsed}ms (slow)"
    fi

    # ── LLM response time ──────────────────────────────────────────────────
    if check_litellm; then
        log_sub "LLM response time"
        
        local total=0
        local count=3
        
        for i in $(seq 1 $count); do
            start=$(date +%s%N)
            timeout 30 "$BINARY" --config "$TEST_CONFIG" --mode single \
                > "$RESULTS_DIR/${TIMESTAMP}-perf-llm-${i}.log" 2>&1 || true
            end=$(date +%s%N)
            elapsed=$(( (end - start) / 1000000 ))
            total=$((total + elapsed))
            log_detail "Run ${i}: ${elapsed}ms"
        done
        
        local avg=$((total / count))
        if [[ "$avg" -lt 1000 ]]; then
            log_ok "Avg LLM response time: ${avg}ms (excellent)"
        elif [[ "$avg" -lt 3000 ]]; then
            log_ok "Avg LLM response time: ${avg}ms (good)"
        elif [[ "$avg" -lt 10000 ]]; then
            log_ok "Avg LLM response time: ${avg}ms (acceptable)"
        else
            log_fail "Avg LLM response time: ${avg}ms (slow)"
        fi
    fi

    # ── Binary size ────────────────────────────────────────────────────────
    log_sub "Binary size"
    local size
    size=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY" 2>/dev/null)
    local size_kb=$((size / 1024))
    local size_mb=$((size / 1024 / 1024))
    log_ok "Binary size: ${size_kb}KB (${size_mb}MB)"

    # ── Memory usage (approximate via vmmap) ───────────────────────────────
    log_sub "Memory usage"
    if command -v vmmap >/dev/null 2>&1; then
        log_detail "Measuring peak memory..."
        # Run in background and measure with timeout
        "$BINARY" --version &
        local pid=$!
        sleep 0.2
        local mem
        mem=$(timeout 5 vmmap "$pid" 2>/dev/null | grep "Physical footprint" | awk '{print $3}' || echo "N/A")
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
        if [[ "$mem" != "N/A" ]]; then
            log_ok "Memory footprint: ${mem}"
        else
            log_detail "Could not measure memory — vmmap may need permissions"
        fi
    else
        log_detail "vmmap not available — skipping memory measurement"
    fi
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_performance
    print_summary "Performance"
fi
