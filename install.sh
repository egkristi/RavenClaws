#!/usr/bin/env bash
# =============================================================================
# RavenClaws — Interactive Installer
# =============================================================================
# Detects the host platform and installs the appropriate prebuilt binary (or
# builds from source), then writes a starter `ravenclaws.toml` if none exists.
#
# Usage:
#   ./install.sh                # interactive (guided)
#   ./install.sh --preset prod  # non-interactive preset
#   ./install.sh --from-source  # build from source with cargo
#   ./install.sh --help
#
# Presets: dev (default), prod, airgap
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

VERSION="${RAVENCLAWS_VERSION:-latest}"
INSTALL_DIR="${RAVENCLAWS_INSTALL_DIR:-$HOME/.local/bin}"
PRESET=""
FROM_SOURCE=false
CONFIG_ONLY=false

# ── Colours ────────────────────────────────────────────────────────────────
C_RESET=$'\033[0m'
C_CYAN=$'\033[36m'
C_GREEN=$'\033[32m'
C_YELLOW=$'\033[33m'
C_RED=$'\033[31m'

info()  { printf '%s\n' "${C_CYAN}→${C_RESET} $*"; }
ok()    { printf '%s\n' "${C_GREEN}✓${C_RESET} $*"; }
warn()  { printf '%s\n' "${C_YELLOW}⚠${C_RESET} $*"; }
fail()  { printf '%s\n' "${C_RED}✗${C_RESET} $*"; exit 1; }

# ── Usage ──────────────────────────────────────────────────────────────────
usage() {
    cat <<'EOF'
🐦‍⬛ RavenClaws Installer

Usage:
  ./install.sh [OPTIONS]

Options:
  --preset <dev|prod|airgap>   Choose a security posture preset (default: interactive)
  --from-source                Build from source using cargo (default: prebuilt binary)
  --config-only                Only write a starter ravenclaws.toml, don't install
  --install-dir <dir>          Install directory (default: ~/.local/bin)
  --version <ver>              Version tag to install (default: latest)
  -h, --help                   Show this help
EOF
}

# ── Platform detection ─────────────────────────────────────────────────────
detect_platform() {
    local os arch target
    case "$(uname -s)" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        MINGW*|MSYS*|CYGWIN*) os="pc-windows-msvc" ;;
        *)      fail "Unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        armv7l) arch="armv7"; fail "armv7 (32-bit Raspberry Pi) is not yet a prebuilt target — use --from-source" ;;
        *)      fail "Unsupported architecture: $(uname -m)" ;;
    esac

    target="${arch}-${os}"
    printf '%s' "$target"
}

# ── Security posture presets ───────────────────────────────────────────────
write_config() {
    local preset="$1" dest="$2"
    if [[ -f "$dest" ]]; then
        warn "Config already exists at $dest — leaving it untouched."
        return
    fi

    mkdir -p "$(dirname "$dest")"

    case "$preset" in
        dev)
            cat > "$dest" <<'EOF'
# RavenClaws configuration — development preset
[llm]
provider = "openai-compatible"
endpoint = "http://localhost:11434"
model = "llama3.1"
timeout_secs = 60

[security]
require_tls = false
token_lifetime_secs = 0
audit_log = true
prompt_injection_protection = true

[runtime]
workdir = "/tmp/ravenclaws"
EOF
            ;;
        prod)
            cat > "$dest" <<'EOF'
# RavenClaws configuration — production preset
[llm]
provider = "litellm"
endpoint = "http://litellm.ravenclaws.svc:4000"
model = "gpt-4o-mini"
timeout_secs = 30

[security]
require_tls = true
token_lifetime_secs = 3600
audit_log = true
prompt_injection_protection = true

[runtime]
workdir = "/workspace"
EOF
            ;;
        airgap)
            cat > "$dest" <<'EOF'
# RavenClaws configuration — air-gapped / high-assurance preset
[llm]
provider = "openai-compatible"
endpoint = "http://localhost:11434"
model = "llama3.1"
timeout_secs = 60

[security]
require_tls = true
token_lifetime_secs = 1800
audit_log = true
prompt_injection_protection = true

[runtime]
workdir = "/workspace"
EOF
            ;;
        *) fail "Unknown preset: $preset" ;;
    esac
    ok "Wrote starter config: $dest"
}

# ── Install prebuilt binary ────────────────────────────────────────────────
install_prebuilt() {
    local target="$1"
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN

    local archive="$target.tar.gz"
    local url="https://github.com/egkristi/RavenClaws/releases/${VERSION}/download/${archive}"

    info "Downloading prebuilt binary for $target (version ${VERSION})..."
    if ! curl -fsSL "$url" -o "$tmp/$archive"; then
        warn "Prebuilt binary not found at $url — falling back to building from source."
        install_from_source
        return
    fi

    tar xzf "$tmp/$archive" -C "$tmp"
    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$tmp/$target" "$INSTALL_DIR/ravenclaws" || {
        warn "Cannot write to $INSTALL_DIR — try setting RAVENCLAWS_INSTALL_DIR."
        exit 1
    }
    ok "Installed ravenclaws to $INSTALL_DIR/ravenclaws"
}

# ── Install from source ────────────────────────────────────────────────────
install_from_source() {
    command -v cargo >/dev/null 2>&1 || fail "cargo not found — install Rust (https://rustup.rs) first."
    info "Building from source..."
    ( cd "$SCRIPT_DIR" && cargo build --release --locked )
    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$SCRIPT_DIR/target/release/ravenclaws" "$INSTALL_DIR/ravenclaws"
    ok "Installed ravenclaws (from source) to $INSTALL_DIR/ravenclaws"
}

# ── Main ───────────────────────────────────────────────────────────────────
main() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --preset) PRESET="$2"; shift 2 ;;
            --from-source) FROM_SOURCE=true; shift ;;
            --config-only) CONFIG_ONLY=true; shift ;;
            --install-dir) INSTALL_DIR="$2"; shift 2 ;;
            --version) VERSION="$2"; shift 2 ;;
            -h|--help) usage; exit 0 ;;
            *) fail "Unknown option: $1 (try --help)" ;;
        esac
    done

    echo "🐦‍⬛ RavenClaws Installer"
    echo "======================="

    local platform
    platform="$(detect_platform)"
    info "Detected platform: $platform"

    # Determine preset interactively if not supplied.
    if [[ -z "$PRESET" ]]; then
        echo "Choose a security posture preset:"
        echo "  1) dev     — local experimentation, TLS relaxed"
        echo "  2) prod    — default posture, TLS + HITL"
        echo "  3) airgap  — maximum containment, no egress"
        printf "Select [1-3] (default 1): "
        read -r choice
        case "${choice:-1}" in
            1) PRESET="dev" ;;
            2) PRESET="prod" ;;
            3) PRESET="airgap" ;;
            *) PRESET="dev" ;;
        esac
    fi
    info "Preset: $PRESET"

    # Write starter config.
    write_config "$PRESET" "./ravenclaws.toml"

    if [[ "$CONFIG_ONLY" = true ]]; then
        ok "Config written; skipping binary install (--config-only)."
        exit 0
    fi

    # Install the binary.
    if [[ "$FROM_SOURCE" = true ]]; then
        install_from_source
    else
        install_prebuilt "$platform"
    fi

    echo ""
    ok "RavenClaws installed. Try: ravenclaws --version"
    echo "   If $INSTALL_DIR is not on your PATH, add it with:"
    echo "     export PATH=\"$INSTALL_DIR:\$PATH\""
}

main "$@"
