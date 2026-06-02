#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — Shared Library
# =============================================================================
# Common functions, colors, paths, and helpers used by all verification modules.
# Source this file: source "$(dirname "$0")/lib/common.sh"
# =============================================================================

set -euo pipefail

# ── Colors ────────────────────────────────────────────────────────────────────
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export NC='\033[0m' # No Color

# ── Paths ─────────────────────────────────────────────────────────────────────
export SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
export BINARY="$PROJECT_DIR/target/release/ravenclaw"
export LINUX_BINARY="$PROJECT_DIR/target/aarch64-unknown-linux-gnu/release/ravenclaw"
export X86_BINARY="$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/ravenclaw"
export TEST_CONFIG="$PROJECT_DIR/tests/config/ravenclaw-test.toml"
export MULTI_CONFIG="$PROJECT_DIR/tests/config/ravenclaw-multi-test.toml"
export K8S_CONFIG="$PROJECT_DIR/tests/config/ravenclaw-k8s-test.toml"
export SWARM_CONFIG="$PROJECT_DIR/tests/config/ravenclaw-swarm-test.toml"
export SUPERVISOR_CONFIG="$PROJECT_DIR/tests/config/ravenclaw-supervisor-test.toml"
export RESULTS_DIR="$PROJECT_DIR/target/verification-results"
export TIMESTAMP=$(date +%Y%m%d-%H%M%S)
export DOCKER_TAG="ravenclaw-verify:${TIMESTAMP}"

# ── Global counters ───────────────────────────────────────────────────────────
export PASS=0
export FAIL=0
export SKIP=0

# ── Logging ───────────────────────────────────────────────────────────────────

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[PASS]${NC}  $*"; PASS=$((PASS + 1)); }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*"; FAIL=$((FAIL + 1)); }
log_skip()  { echo -e "${YELLOW}[SKIP]${NC}  $*"; SKIP=$((SKIP + 1)); }
log_step()  { echo -e "\n${CYAN}════════════════════════════════════════════════${NC}"; echo -e "${CYAN}  $*${NC}"; echo -e "${CYAN}════════════════════════════════════════════════${NC}"; }
log_sub()   { echo -e "  ${YELLOW}▶${NC} $*"; }
log_detail() { echo -e "    ${BLUE}→${NC} $*"; }

# ── Prerequisite checks ───────────────────────────────────────────────────────

check_prereq() {
    if ! command -v "$1" &>/dev/null; then
        log_skip "Prerequisite '$1' not found — skipping related tests"
        return 1
    fi
    return 0
}

check_litellm() {
    if ! curl -sf http://localhost:4000/health/readiness >/dev/null 2>&1; then
        log_skip "LiteLLM not reachable at http://localhost:4000 — skipping LLM tests"
        return 1
    fi
    return 0
}

check_binary() {
    if [[ ! -x "$BINARY" ]]; then
        log_skip "RavenClaw binary not found at $BINARY — run 'cargo build --release' first"
        return 1
    fi
    # Verify it's a macOS binary (not cross-compiled Linux)
    local file_type
    file_type=$(file "$BINARY" 2>/dev/null)
    if ! echo "$file_type" | grep -q "Mach-O"; then
        log_skip "Binary at $BINARY is not a macOS binary (got: $file_type) — rebuild with 'cargo build --release'"
        return 1
    fi
    return 0
}

check_linux_binary() {
    local bin_path="$1"
    local label="$2"
    if [[ ! -x "$bin_path" ]]; then
        log_skip "Linux binary ($label) not found at $bin_path"
        return 1
    fi
    local file_type
    file_type=$(file "$bin_path" 2>/dev/null)
    if ! echo "$file_type" | grep -q "ELF"; then
        log_skip "File at $bin_path is not a Linux ELF binary (got: $file_type)"
        return 1
    fi
    return 0
}

check_docker_image() {
    if ! docker image inspect "$DOCKER_TAG" >/dev/null 2>&1; then
        log_skip "Docker image '$DOCKER_TAG' not found — run Docker build first"
        return 1
    fi
    return 0
}

# ── Test runner ───────────────────────────────────────────────────────────────

run_test() {
    local name="$1"
    local result_file="$RESULTS_DIR/${TIMESTAMP}-${name// /-}.log"
    shift
    log_sub "Running: $name"
    if "$@" > "$result_file" 2>&1; then
        log_ok "$name"
        return 0
    else
        local exit_code=$?
        log_fail "$name (exit code: $exit_code)"
        if [[ -s "$result_file" ]]; then
            log_sub "Last 20 lines of output:"
            tail -20 "$result_file" | sed 's/^/    /'
        fi
        return 1
    fi
}

run_test_verbose() {
    local name="$1"
    local result_file="$RESULTS_DIR/${TIMESTAMP}-${name// /-}.log"
    shift
    log_sub "Running: $name"
    log_detail "Command: $*"
    if "$@" > "$result_file" 2>&1; then
        log_ok "$name"
        return 0
    else
        local exit_code=$?
        log_fail "$name (exit code: $exit_code)"
        if [[ -s "$result_file" ]]; then
            log_sub "Full output:"
            cat "$result_file" | sed 's/^/    /'
        fi
        return 1
    fi
}

# ── LLM response quality check ────────────────────────────────────────────────

check_llm_response_quality() {
    local log_file="$1"
    local model_name="$2"
    
    if [[ ! -f "$log_file" ]]; then
        log_fail "Model $model_name — no log file found"
        return 1
    fi
    
    # Check that we got a response (not empty)
    if grep -q '"Agent response received"' "$log_file" 2>/dev/null; then
        local response
        response=$(grep -o '"response":"[^"]*"' "$log_file" 2>/dev/null | head -1)
        local response_len=${#response}
        if [[ "$response_len" -gt 20 ]]; then
            log_ok "Model $model_name — responded (${response_len} chars)"
            return 0
        else
            log_fail "Model $model_name — response too short"
            return 1
        fi
    elif grep -q '"LLM request failed"' "$log_file" 2>/dev/null; then
        log_fail "Model $model_name — LLM request failed"
        return 1
    else
        log_fail "Model $model_name — no response detected in log"
        return 1
    fi
}

# ── Summary ───────────────────────────────────────────────────────────────────

print_summary() {
    local suite_name="${1:-RavenClaw}"
    local total=$((PASS + FAIL + SKIP))
    echo -e "\n${CYAN}════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  ${suite_name} — Verification Summary${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════${NC}"
    echo -e "  Total:   ${total}"
    echo -e "  ${GREEN}Passed:   ${PASS}${NC}"
    echo -e "  ${RED}Failed:   ${FAIL}${NC}"
    echo -e "  ${YELLOW}Skipped:  ${SKIP}${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════${NC}"
    echo -e "  Results: ${RESULTS_DIR}/${TIMESTAMP}-*.log"
    echo -e "${CYAN}════════════════════════════════════════════════${NC}"

    if [[ $FAIL -eq 0 ]]; then
        echo -e "${GREEN}  ✓ ALL VERIFICATIONS PASSED${NC}"
    else
        echo -e "${RED}  ✗ SOME VERIFICATIONS FAILED${NC}"
    fi
    echo -e "${CYAN}════════════════════════════════════════════════${NC}"
}

# ── Initialization ────────────────────────────────────────────────────────────

init_verification() {
    mkdir -p "$RESULTS_DIR"
    echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║     RavenClaw Verification Suite                ║${NC}"
    echo -e "${CYAN}║     $(date)              ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "  Project:    $PROJECT_DIR"
    echo "  Binary:     $BINARY"
    echo "  Config:     $TEST_CONFIG"
    echo "  Results:    $RESULTS_DIR"
    echo "  Platform:   $(uname -a | cut -d' ' -f1-4)"
    echo ""
}
