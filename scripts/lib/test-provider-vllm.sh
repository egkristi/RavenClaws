#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — vLLM Provider Tests
# =============================================================================
# Tests that vLLM is running and RavenClaws can connect via openai-compatible.
# Skipped if vLLM is not available.
# =============================================================================

test_provider_vllm() {
    log_step "vLLM Provider"

    # Check if vLLM is running
    if ! curl -sf http://localhost:8000/health >/dev/null 2>&1; then
        log_skip "vLLM not reachable at http://localhost:8000 — skipping vLLM tests"
        return 0
    fi

    log_ok "vLLM is reachable"

    # Get available models
    local model
    model=$(curl -sf http://localhost:8000/v1/models 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
models = [m['id'] for m in data.get('data', [])]
print(models[0] if models else '')
" 2>/dev/null || echo "")

    if [[ -z "$model" ]]; then
        log_skip "No models found in vLLM — skipping vLLM tests"
        return 0
    fi

    log_detail "Using model: $model"

    # Test basic connectivity
    run_test "vLLM basic prompt" env \
        RAVENCLAWS__LLM__PROVIDER="openai-compatible" \
        RAVENCLAWS__LLM__ENDPOINT="http://localhost:8000/v1/chat/completions" \
        RAVENCLAWS__LLM__MODEL="$model" \
        timeout 60 "$BINARY" --exec "Respond with exactly: OK" 2>&1

    # Test agent loop with --no-final-required
    run_test "vLLM agent loop" env \
        RAVENCLAWS__LLM__PROVIDER="openai-compatible" \
        RAVENCLAWS__LLM__ENDPOINT="http://localhost:8000/v1/chat/completions" \
        RAVENCLAWS__LLM__MODEL="$model" \
        timeout 120 "$BINARY" --exec "What is 2+2? Respond with just the number." --no-final-required 2>&1
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_provider_vllm
    print_summary "vLLM Provider"
fi
