#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — llama.cpp Provider Tests
# =============================================================================
# Tests that llama.cpp is running and RavenClaws can connect via openai-compatible.
# Skipped if llama.cpp is not available.
# =============================================================================

test_provider_llamacpp() {
    log_step "llama.cpp Provider"

    # Check if llama.cpp is running
    if ! curl -sf http://localhost:8080/health >/dev/null 2>&1; then
        log_skip "llama.cpp not reachable at http://localhost:8080 — skipping llama.cpp tests"
        return 0
    fi

    log_ok "llama.cpp is reachable"

    # Get available models
    local model
    model=$(curl -sf http://localhost:8080/v1/models 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
models = [m['id'] for m in data.get('data', [])]
print(models[0] if models else '')
" 2>/dev/null || echo "")

    if [[ -z "$model" ]]; then
        log_detail "No models list from llama.cpp, using default model name"
        model="default"
    fi

    log_detail "Using model: $model"

    # Test basic connectivity
    run_test "llama.cpp basic prompt" env \
        RAVENCLAWS__LLM__PROVIDER="openai-compatible" \
        RAVENCLAWS__LLM__ENDPOINT="http://localhost:8080/v1/chat/completions" \
        RAVENCLAWS__LLM__MODEL="$model" \
        timeout 120 "$BINARY" --exec "Respond with exactly: OK" 2>&1

    # Test agent loop with --no-final-required (llama.cpp doesn't do FINAL:)
    run_test "llama.cpp agent loop" env \
        RAVENCLAWS__LLM__PROVIDER="openai-compatible" \
        RAVENCLAWS__LLM__ENDPOINT="http://localhost:8080/v1/chat/completions" \
        RAVENCLAWS__LLM__MODEL="$model" \
        timeout 120 "$BINARY" --exec "What is 2+2? Respond with just the number." --no-final-required 2>&1
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_provider_llamacpp
    print_summary "llama.cpp Provider"
fi
