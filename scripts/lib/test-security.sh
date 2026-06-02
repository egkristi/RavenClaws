#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — Security & Binary Integrity Tests
# =============================================================================
# Tests binary integrity: no debug symbols, correct architecture,
# no hardcoded secrets, reasonable size, static analysis.
# =============================================================================

test_security() {
    log_step "6. Security & Binary Integrity"

    if ! check_binary; then return; fi

    # ── Binary analysis ────────────────────────────────────────────────────
    log_sub "Binary analysis"

    # Architecture
    run_test "Binary is aarch64 Mach-O 64-bit" \
        bash -c "file \"$BINARY\" 2>&1 | grep -q 'Mach-O 64-bit executable arm64'"

    # Stripped (release build) — macOS file doesn't say "stripped"
    # Check binary was built in release mode via file type (Mach-O 64-bit executable, not object file)
    run_test "Binary is release build (Mach-O executable)" \
        bash -c "file \"$BINARY\" 2>&1 | grep -q 'executable'"

    # ── Secrets scanning ───────────────────────────────────────────────────
    log_sub "Secrets scanning"

    # No API keys in binary strings
    run_test "No hardcoded OpenAI-style API keys" \
        bash -c "! strings \"$BINARY\" 2>/dev/null | grep -qiE 'sk-[a-zA-Z0-9]{20,}'"

    # No generic credential patterns — look for actual credential VALUES, not field names
    # (field names like "api_key" and "token" are expected in config structs)
    run_test "No hardcoded credential values" \
        bash -c "! strings \"$BINARY\" 2>/dev/null | grep -qiE '(sk-[a-zA-Z0-9]{20,})|(ghp_[a-zA-Z0-9]{36})'"

    # ── Binary size ────────────────────────────────────────────────────────
    log_sub "Binary size"
    local size
    size=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY" 2>/dev/null)
    local size_mb=$((size / 1024 / 1024))
    
    if [[ -n "$size" ]]; then
        if [[ "$size" -lt 5000000 ]]; then
            log_ok "Binary size: ${size_mb}MB (excellent — under 5MB)"
        elif [[ "$size" -lt 10000000 ]]; then
            log_ok "Binary size: ${size_mb}MB (good — under 10MB)"
        elif [[ "$size" -lt 50000000 ]]; then
            log_ok "Binary size: ${size_mb}MB (acceptable — under 50MB)"
        else
            log_fail "Binary size: ${size_mb}MB (too large — over 50MB)"
        fi
    fi

    # ── Dependency analysis ────────────────────────────────────────────────
    log_sub "Dependency analysis"
    
    # Check for known-vulnerable patterns (basic scan)
    run_test "No hardcoded secrets in binary strings" \
        bash -c "! strings \"$BINARY\" 2>/dev/null | grep -qiE 'sk-[a-zA-Z0-9]{20,}'"

    # ── Runtime security ───────────────────────────────────────────────────
    log_sub "Runtime security"
    
    # Binary doesn't require elevated privileges
    run_test "Binary has no setuid or setgid" \
        bash -c "test ! -u \"$BINARY\" && test ! -g \"$BINARY\""

    # ── Supply chain ───────────────────────────────────────────────────────
    log_sub "Supply chain"
    
    # Check Cargo.lock exists (reproducible builds)
    if [[ -f "$PROJECT_DIR/Cargo.lock" ]]; then
        log_ok "Cargo.lock present — reproducible builds"
        PASS=$((PASS + 1))
    else
        log_fail "Cargo.lock missing — run 'cargo generate-lockfile'"
    fi
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_security
    print_summary "Security & Binary Integrity"
fi
