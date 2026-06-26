#!/usr/bin/env bash
# =============================================================================
# RavenClaws — Git Hooks Setup Script
# =============================================================================
# Configures git to use the .githooks directory and makes hooks executable.
#
# Usage:
#   .githooks/setup.sh              # Configure hooks for this repo
#   .githooks/setup.sh --check      # Verify hooks are properly configured
#   .githooks/setup.sh --remove     # Restore default git hooks
# =============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_DIR="$PROJECT_DIR/.githooks"

ok()    { echo -e "  ${GREEN}✓${NC} $1"; }
info()  { echo -e "  ${CYAN}→${NC} $1"; }
warn()  { echo -e "  ${YELLOW}⚠${NC} $1"; }
fail()  { echo -e "  ${RED}✗${NC} $1"; }

do_setup() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║     RavenClaws Git Hooks Setup                   ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"
    echo ""

    # Make hooks executable
    chmod +x "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-push" 2>/dev/null || true
    info "Made hooks executable"

    # Configure git to use .githooks directory
    git config core.hooksPath "$HOOKS_DIR"
    ok "Git hooks path set to: $HOOKS_DIR"

    echo ""
    echo -e "  ${GREEN}✓ RavenClaws git hooks are now active!${NC}"
    echo ""
    echo -e "  ${CYAN}Hooks installed:${NC}"
    echo -e "    pre-commit   — Runs on every commit: fmt, clippy, tests, binary size, secrets"
    echo -e "    pre-push     — Runs on every push: full pre-commit + release build + Docker + security"
    echo ""
    echo -e "  ${YELLOW}To skip hooks (emergency only):${NC}"
    echo -e "    git commit --no-verify"
    echo -e "    git push --no-verify"
    echo ""
}

do_check() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║     RavenClaws Git Hooks — Status Check          ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"
    echo ""

    local current_hooks_path
    current_hooks_path=$(git config core.hooksPath 2>/dev/null || echo "")

    if [[ "$current_hooks_path" == "$HOOKS_DIR" ]]; then
        ok "Git hooks path correctly set to: $HOOKS_DIR"
    elif [[ -z "$current_hooks_path" ]]; then
        warn "Git hooks path not set (using default .git/hooks)"
        info "Run '.githooks/setup.sh' to configure"
    else
        warn "Git hooks path set to different location: $current_hooks_path"
        info "Run '.githooks/setup.sh' to reconfigure"
    fi

    if [[ -x "$HOOKS_DIR/pre-commit" ]]; then
        ok "pre-commit hook is executable"
    else
        warn "pre-commit hook not found or not executable"
    fi

    if [[ -x "$HOOKS_DIR/pre-push" ]]; then
        ok "pre-push hook is executable"
    else
        warn "pre-push hook not found or not executable"
    fi

    echo ""
}

do_remove() {
    echo ""
    echo -e "${YELLOW}╔══════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║     Removing RavenClaws Git Hooks               ║${NC}"
    echo -e "${YELLOW}╚══════════════════════════════════════════════════╝${NC}"
    echo ""

    # Restore default hooks path
    git config --unset core.hooksPath 2>/dev/null || true
    ok "Git hooks path restored to default (.git/hooks)"

    echo ""
    echo -e "  ${GREEN}✓ RavenClaws git hooks removed.${NC}"
    echo -e "  ${YELLOW}  The .githooks/ directory still exists — delete it manually if desired.${NC}"
    echo ""
}

# ── Main ──────────────────────────────────────────────────────────────────────

case "${1:-setup}" in
    setup|--setup|-s)
        do_setup
        ;;
    check|--check|-c)
        do_check
        ;;
    remove|--remove|-r)
        do_remove
        ;;
    *)
        echo "Usage: $0 [setup|check|remove]"
        echo ""
        echo "  setup   Configure git to use .githooks (default)"
        echo "  check   Verify hooks are properly configured"
        echo "  remove  Restore default git hooks"
        exit 1
        ;;
esac
