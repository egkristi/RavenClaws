#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification Suite — Main Orchestrator
# =============================================================================
# Runs all verification modules across all deployment targets.
#
# Usage:
#   ./scripts/verify.sh                  # Run all tests
#   ./scripts/verify.sh --list           # List available test modules
#   ./scripts/verify.sh --litellm        # LiteLLM connectivity only
#   ./scripts/verify.sh --local          # Local macOS binary only
#   ./scripts/verify.sh --docker         # Docker container only
#   ./scripts/verify.sh --linux          # Linux binary only
#   ./scripts/verify.sh --k8s            # Kubernetes only
#   ./scripts/verify.sh --security       # Security & binary integrity
#   ./scripts/verify.sh --performance    # Performance benchmarks
#   ./scripts/verify.sh --llm-quality    # LLM response quality
#   ./scripts/verify.sh --swarm          # Swarm & sub-agent scalability
#   ./scripts/verify.sh --eval           # Eval harness
#   ./scripts/verify.sh --quick          # Quick smoke test (local + litellm + swarm + eval)
#   ./scripts/verify.sh --build          # Build + all tests
#
# Environment:
#   VERBOSE=1          Show detailed output for each test
#   SKIP_BUILD=1       Skip cargo build step
#   RUST_LOG=info      Log level for RavenClaws itself
# =============================================================================

set -euo pipefail

# ── Source shared library ─────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

# ── Available modules ─────────────────────────────────────────────────────────
# Format: key:filename:function_name:description
MODULES=(
    "litellm:test-litellm.sh:test_litellm_connectivity:LiteLLM Connectivity"
    "local:test-local.sh:test_local_binary:Local macOS Binary"
    "docker:test-docker.sh:test_docker:Docker Container"
    "linux:test-linux.sh:test_linux_binary:Linux Binary"
    "k8s:test-k8s.sh:test_kubernetes:Kubernetes"
    "security:test-security.sh:test_security:Security & Binary Integrity"
    "performance:test-performance.sh:test_performance:Performance Benchmarks"
    "llm-quality:test-llm-quality.sh:test_llm_quality:LLM Response Quality"
    "swarm:test-swarm.sh:test_swarm_and_subagent:Swarm & Sub-Agent Scalability"
    "eval:test-eval.sh:test_eval_harness:Eval Harness"
)

list_modules() {
    echo -e "${CYAN}Available verification modules:${NC}"
    for module in "${MODULES[@]}"; do
        local key="${module%%:*}"
        local rest="${module#*:}"
        local file="${rest%%:*}"
        local desc="${rest#*:}"
        echo -e "  ${YELLOW}--${key}${NC}    ${desc}"
    done
    echo ""
    echo -e "${CYAN}Special targets:${NC}"
    echo -e "  ${YELLOW}--all${NC}      Run all modules (default)"
    echo -e "  ${YELLOW}--quick${NC}    Quick smoke test (litellm + local + swarm + security)"
    echo -e "  ${YELLOW}--build${NC}    Build + run all tests"
    echo -e "  ${YELLOW}--list${NC}     List this help"
}

run_module() {
    local key="$1"
    for module in "${MODULES[@]}"; do
        local mkey="${module%%:*}"
        if [[ "$mkey" == "$key" ]]; then
            local rest="${module#*:}"
            local file="${rest%%:*}"
            local rest2="${rest#*:}"
            local func_name="${rest2%%:*}"
            source "$SCRIPT_DIR/lib/$file"
            if declare -f "$func_name" >/dev/null; then
                $func_name
            else
                log_fail "Test function '$func_name' not found in $file"
            fi
            return 0
        fi
    done
    log_fail "Unknown module: $key"
    return 1
}

# ── Build step ────────────────────────────────────────────────────────────────

do_build() {
    log_step "Build: cargo build --release"
    log_detail "Building RavenClaws for macOS (aarch64-apple-darwin)..."
    if cargo build --release 2>&1; then
        log_ok "Build successful"
        local size
        size=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY" 2>/dev/null)
        local size_mb=$((size / 1024 / 1024))
        log_detail "Binary: $BINARY (${size_mb}MB)"
    else
        log_fail "Build failed"
        exit 1
    fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    local mode="${1:---all}"

    case "$mode" in
        --list|-l)
            list_modules
            exit 0
            ;;
        --build)
            do_build
            # Then run all tests
            mode="--all"
            ;;
        --quick)
            init_verification
            run_module "litellm"
            run_module "local"
            run_module "swarm"
            run_module "eval"
            run_module "security"
            print_summary "Quick Smoke Test"
            ;;
        --all)
            init_verification
            run_module "litellm"
            run_module "local"
            run_module "docker"
            run_module "linux"
            run_module "k8s"
            run_module "security"
            run_module "performance"
            run_module "llm-quality"
            print_summary "Full Verification Suite"
            ;;
        --*)
            local key="${mode#--}"
            init_verification
            run_module "$key"
            print_summary "${key} Tests"
            ;;
        *)
            echo -e "${RED}Unknown option: $mode${NC}"
            echo "Usage: $0 [--all|--quick|--build|--list|--litellm|--local|--docker|--linux|--k8s|--security|--performance|--llm-quality|--swarm|--eval]"
            exit 1
            ;;
    esac

    # Return exit code based on failures
    [[ $FAIL -eq 0 ]]
}

main "$@"
