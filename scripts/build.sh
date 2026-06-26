#!/bin/bash
# RavenClaws Build Script
# Builds optimized release binaries for multiple architectures

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "🐦‍⬛ RavenClaws Build"
echo "==================="

# Detect host architecture
HOST_ARCH=$(rustc -vV | grep host | cut -d' ' -f2)
echo "Host: $HOST_ARCH"

# Define target architectures
# Format: target_triple:output_name
TARGETS=(
    "aarch64-apple-darwin:ravenclaws-aarch64-apple-darwin"
    "x86_64-apple-darwin:ravenclaws-x86_64-apple-darwin"
    "aarch64-unknown-linux-gnu:ravenclaws-aarch64-unknown-linux-gnu"
    "x86_64-unknown-linux-gnu:ravenclaws-x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl:ravenclaws-x86_64-unknown-linux-musl"
)

# Parse arguments
BUILD_ALL=false
BUILD_TARGET=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --all) BUILD_ALL=true ;;
        --target) BUILD_TARGET="$2"; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
    shift
done

build_for_target() {
    local target="$1"
    local output_name="$2"
    
    echo ""
    echo "▶ Building for $target..."
    
    if cargo build --release --locked --target "$target" 2>&1; then
        local src="target/$target/release/ravenclaws"
        local dst="target/release/$output_name"
        cp "$src" "$dst"
        local size=$(du -h "$dst" | cut -f1)
        echo "  ✅ $output_name — $size"
    else
        echo "  ❌ $target build failed"
        return 1
    fi
}

# Build for host natively (fast path)
echo ""
echo "▶ Building for host ($HOST_ARCH)..."
cargo build --release --locked
HOST_SIZE=$(du -h target/release/ravenclaws | cut -f1)
echo "  ✅ ravenclaws ($HOST_ARCH) — $HOST_SIZE"

# Build for specific target or all
if [[ -n "$BUILD_TARGET" ]]; then
    for entry in "${TARGETS[@]}"; do
        target="${entry%%:*}"
        name="${entry#*:}"
        if [[ "$target" == "$BUILD_TARGET" ]]; then
            build_for_target "$target" "$name"
            break
        fi
    done
elif [[ "$BUILD_ALL" == "true" ]]; then
    for entry in "${TARGETS[@]}"; do
        target="${entry%%:*}"
        name="${entry#*:}"
        # Skip host (already built)
        if [[ "$target" != "$HOST_ARCH" ]]; then
            build_for_target "$target" "$name"
        fi
    done
fi

# Build Docker image (optional)
if [[ "${BUILD_DOCKER:-false}" == "true" ]]; then
    echo ""
    echo "▶ Building Docker image..."
    docker build -t ravenclaws:latest .
    echo "  ✅ Docker image built"
fi

# Run tests (optional)
if [[ "${RUN_TESTS:-false}" == "true" ]]; then
    echo ""
    echo "▶ Running tests..."
    cargo test --locked
    echo "  ✅ Tests passed"
fi

echo ""
echo "==================="
echo "🐦‍⬛ Build complete!"
