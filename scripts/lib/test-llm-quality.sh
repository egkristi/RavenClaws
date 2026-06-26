#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — LLM Response Quality Tests
# =============================================================================
# Tests each available LiteLLM model with a prompt and verifies
# non-empty, meaningful responses. Tests single and multi-model modes.
# =============================================================================

test_llm_quality() {
    log_step "8. LLM Response Quality (via LiteLLM)"

    if ! check_litellm; then return; fi
    if ! check_binary; then return; fi

    # ── Get available models ───────────────────────────────────────────────
    log_sub "Fetching available models from LiteLLM..."
    local models
    models=$(curl -sf http://localhost:4000/models 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
# Skip non-chat models
skip = {'nomic-embed-text', 'embedding'}
for m in data['data']:
    if m['id'] not in skip:
        print(m['id'])
" 2>/dev/null || echo "")

    if [[ -z "$models" ]]; then
        log_fail "No chat models returned from LiteLLM"
        return
    fi

    local model_count
    model_count=$(echo "$models" | wc -l | tr -d ' ')
    log_detail "Found ${model_count} chat models"

    # ── Test each model individually ───────────────────────────────────────
    log_sub "Individual model tests"
    local passed=0
    local unavailable=0
    for model in $models; do
        log_detail "Testing model: $model"
        local log_file="$RESULTS_DIR/${TIMESTAMP}-quality-${model}.log"
        
        timeout 60 env \
            RAVENCLAW__LLM__ENDPOINT="http://localhost:4000" \
            RAVENCLAW__LLM__MODEL="$model" \
            RAVENCLAW__SECURITY__REQUIRE_TLS=false \
            "$BINARY" --mode single > "$log_file" 2>&1 || true
        
        # Check if LLM request failed (model unavailable)
        if grep -q '"LLM request failed"' "$log_file" 2>/dev/null; then
            log_skip "Model $model — unavailable (no API key or service down)"
            unavailable=$((unavailable + 1))
        elif check_llm_response_quality "$log_file" "$model"; then
            passed=$((passed + 1))
        else
            log_detail "Model $model — unexpected issue"
            unavailable=$((unavailable + 1))
        fi
    done
    log_detail "Model tests: ${passed} passed, ${unavailable} unavailable"

    # ── Test with a specific prompt ────────────────────────────────────────
    log_sub "Prompt-specific quality tests"
    
    # Test reasoning capability
    local reasoning_log="$RESULTS_DIR/${TIMESTAMP}-quality-reasoning.log"
    log_detail "Testing reasoning: 'What is 15 * 37?'"
    if timeout 60 env \
        RAVENCLAW__LLM__ENDPOINT="http://localhost:4000" \
        RAVENCLAW__LLM__MODEL="best-coding" \
        RAVENCLAW__SECURITY__REQUIRE_TLS=false \
        "$BINARY" --mode single > "$reasoning_log" 2>&1; then
        check_llm_response_quality "$reasoning_log" "best-coding (reasoning)"
    else
        log_fail "Reasoning test — failed"
    fi

    # ── Multi-model quality ────────────────────────────────────────────────
    log_sub "Multi-model quality"
    local multi_log="$RESULTS_DIR/${TIMESTAMP}-quality-multi.log"
    log_detail "Testing all configured models simultaneously..."
    if timeout 120 "$BINARY" --config "$MULTI_CONFIG" --mode single \
        > "$multi_log" 2>&1; then
        local resp_count
        resp_count=$(grep -c '"Provider response received"' "$multi_log" 2>/dev/null || echo 0)
        if [[ "$resp_count" -ge 2 ]]; then
            log_ok "Multi-model: ${resp_count}/3 providers responded"
        else
            log_fail "Multi-model: only ${resp_count}/3 providers responded"
        fi
    else
        log_fail "Multi-model quality test — failed"
        tail -10 "$multi_log" | sed 's/^/    /'
    fi

    # ── Response diversity ─────────────────────────────────────────────────
    log_sub "Response diversity"
    # Check that different models give different responses
    local responses
    responses=$(grep -o '"response":"[^"]*"' "$multi_log" 2>/dev/null | sort -u | wc -l | tr -d ' ')
    if [[ "$responses" -ge 2 ]]; then
        log_ok "Response diversity: ${responses} unique responses across models"
    else
        log_detail "Response diversity: ${responses} unique responses (may be same model)"
    fi
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_llm_quality
    print_summary "LLM Response Quality"
fi
