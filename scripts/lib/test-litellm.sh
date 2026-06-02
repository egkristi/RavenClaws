#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — LiteLLM Connectivity Tests
# =============================================================================
# Tests that LiteLLM is running and accessible.
# =============================================================================

test_litellm_connectivity() {
    log_step "1. LiteLLM Connectivity"

    # Health check
    run_test "LiteLLM health check" curl -sf http://localhost:4000/health/readiness

    # Models list
    run_test "LiteLLM models list" curl -sf http://localhost:4000/models

    # Verify specific models are available
    log_sub "Checking available models..."
    local models
    models=$(curl -sf http://localhost:4000/models 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
for m in data['data']:
    print(m['id'])
" 2>/dev/null || echo "")
    
    if [[ -z "$models" ]]; then
        log_fail "No models returned from LiteLLM"
        return
    fi
    
    local model_count
    model_count=$(echo "$models" | wc -l | tr -d ' ')
    log_ok "LiteLLM reports ${model_count} models available"
    
    # Check for key models
    for required in "best-coding" "best-chat" "fast" "cheap"; do
        if echo "$models" | grep -q "$required"; then
            log_detail "Required model '$required' — available"
        else
            log_detail "Required model '$required' — NOT available (may be aliased)"
        fi
    done
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_litellm_connectivity
    print_summary "LiteLLM Connectivity"
fi
