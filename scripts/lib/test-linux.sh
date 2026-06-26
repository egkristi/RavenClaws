#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — Linux Binary Tests (via Orbstack Docker)
# =============================================================================
# Tests cross-compiled Linux binaries (aarch64 and x86_64) using
# Orbstack Docker containers with Debian slim images.
# =============================================================================

test_linux_binary() {
    log_step "4. Linux Binary Verification (via Orbstack Docker)"

    if ! check_prereq docker; then return; fi

    local linux_binaries=()
    local linux_labels=()

    # Check for Linux ARM64 binary
    if [[ -x "$LINUX_BINARY" ]]; then
        linux_binaries+=("$LINUX_BINARY")
        linux_labels+=("aarch64")
        log_detail "Found Linux ARM64 binary: $LINUX_BINARY ($(du -h "$LINUX_BINARY" | cut -f1))"
    fi

    # Check for Linux x86_64 binary
    if [[ -x "$X86_BINARY" ]]; then
        linux_binaries+=("$X86_BINARY")
        linux_labels+=("x86_64")
        log_detail "Found Linux x86_64 binary: $X86_BINARY ($(du -h "$X86_BINARY" | cut -f1))"
    fi

    if [[ ${#linux_binaries[@]} -eq 0 ]]; then
        log_skip "No cross-compiled Linux binaries found"
        log_detail "Build with: cargo build --release --target aarch64-unknown-linux-gnu"
        log_detail "Or:        cargo build --release --target x86_64-unknown-linux-gnu"
        return
    fi

    for i in "${!linux_binaries[@]}"; do
        local bin="${linux_binaries[$i]}"
        local arch="${linux_labels[$i]}"
        
        log_sub "Testing Linux binary (${arch})"

        # Basic checks
        run_test "Linux ${arch} binary --version" \
            docker run --rm -v "${bin}:/ravenclaws:ro" debian:bookworm-slim /ravenclaws --version
        run_test "Linux ${arch} binary --help" \
            docker run --rm -v "${bin}:/ravenclaws:ro" debian:bookworm-slim /ravenclaws --help

        # LLM connectivity (via host network)
        if check_litellm; then
            log_detail "Testing LLM connectivity (${arch})..."
            if timeout 45 docker run --rm --network host \
                -v "${bin}:/ravenclaws:ro" \
                -e RAVENCLAW__LLM__ENDPOINT="http://localhost:4000" \
                -e RAVENCLAW__LLM__MODEL="best-coding" \
                -e RAVENCLAW__SECURITY__REQUIRE_TLS=false \
                debian:bookworm-slim /ravenclaws --mode single \
                > "$RESULTS_DIR/${TIMESTAMP}-linux-${arch}-llm.log" 2>&1; then
                check_llm_response_quality "$RESULTS_DIR/${TIMESTAMP}-linux-${arch}-llm.log" "best-coding (Linux ${arch})"
            else
                log_fail "Linux ${arch} — LLM chat failed"
                tail -10 "$RESULTS_DIR/${TIMESTAMP}-linux-${arch}-llm.log" | sed 's/^/    /'
            fi

            # Multi-model on Linux
            log_detail "Testing multi-model (${arch})..."
            if timeout 90 docker run --rm --network host \
                -v "${bin}:/ravenclaws:ro" \
                -v "$MULTI_CONFIG:/config/multi.toml:ro" \
                -e RAVENCLAW__SECURITY__REQUIRE_TLS=false \
                debian:bookworm-slim /ravenclaws --config /config/multi.toml --mode single \
                > "$RESULTS_DIR/${TIMESTAMP}-linux-${arch}-multi.log" 2>&1; then
                log_ok "Linux ${arch} multi-model — all providers responded"
            else
                log_fail "Linux ${arch} multi-model — check log"
                tail -10 "$RESULTS_DIR/${TIMESTAMP}-linux-${arch}-multi.log" | sed 's/^/    /'
            fi
        fi

        # Binary integrity
        run_test "Linux ${arch} binary is ELF" \
            bash -c "file '${bin}' 2>&1 | grep -q 'ELF'"
        run_test "Linux ${arch} binary is stripped" \
            bash -c "file '${bin}' 2>&1 | grep -q 'stripped'"
    done
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_linux_binary
    print_summary "Linux Binary"
fi
