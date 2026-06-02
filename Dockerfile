# RavenClaw — Multi-stage build for minimal production image
# Supports: linux/amd64, linux/arm64
# Usage: docker buildx build --platform linux/amd64,linux/arm64 -t ravenclaw:latest .
# syntax=docker/dockerfile:1

# Stage 1: Builder
ARG TARGETPLATFORM
ARG BUILDPLATFORM

FROM --platform=$BUILDPLATFORM rust:1.86-slim-bookworm AS builder

WORKDIR /app

# Map TARGETPLATFORM to Rust target triple and RavenFabric arch
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        "linux/amd64")  echo "x86_64-unknown-linux-gnu" > /tmp/rust_target.txt \
                        && echo "amd64" > /tmp/rf_arch.txt ;; \
        "linux/arm64")  echo "aarch64-unknown-linux-gnu" > /tmp/rust_target.txt \
                        && echo "arm64" > /tmp/rf_arch.txt ;; \
        *)              echo "x86_64-unknown-linux-gnu" > /tmp/rust_target.txt \
                        && echo "amd64" > /tmp/rf_arch.txt ;; \
    esac

# Install dependencies and cross-compilation tools
# hadolint ignore=DL3008
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    ca-certificates \
    gcc-aarch64-linux-gnu \
    gcc-x86-64-linux-gnu \
    cmake \
    libc6-dev-arm64-cross \
    libc6-dev-amd64-cross \
    linux-libc-dev-arm64-cross \
    linux-libc-dev-amd64-cross \
    && rm -rf /var/lib/apt/lists/*

# Configure cargo for cross-compilation
RUN case "$TARGETPLATFORM" in \
        "linux/amd64")  \
            printf '[target.x86_64-unknown-linux-gnu]\nlinker = "x86_64-linux-gnu-gcc"\n' >> /usr/local/cargo/config.toml ;; \
        "linux/arm64")  \
            printf '[target.aarch64-unknown-linux-gnu]\nlinker = "aarch64-linux-gnu-gcc"\n' >> /usr/local/cargo/config.toml ;; \
    esac

# Download RavenFabric agent binary (optional runtime component)
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ARG RAVENFABRIC_VERSION=v0.25.1
RUN RF_ARCH=$(cat /tmp/rf_arch.txt) && \
    echo "Downloading RavenFabric ${RAVENFABRIC_VERSION} for ${RF_ARCH}..." && \
    curl -fsSL \
      "https://github.com/egkristi/RavenFabric-Published/releases/download/${RAVENFABRIC_VERSION}/ravenfabric-linux-${RF_ARCH}-agent" \
      -o /app/ravenfabric-agent && \
    curl -fsSL \
      "https://github.com/egkristi/RavenFabric-Published/releases/download/${RAVENFABRIC_VERSION}/SHA256SUMS" \
      -o /app/SHA256SUMS && \
    EXPECTED=$(grep "ravenfabric-linux-${RF_ARCH}-agent" /app/SHA256SUMS | cut -d' ' -f1) && \
    ACTUAL=$(sha256sum /app/ravenfabric-agent | cut -d' ' -f1) && \
    if [ "$EXPECTED" != "$ACTUAL" ]; then \
      echo "SHA256 mismatch: expected $EXPECTED, got $ACTUAL"; \
      exit 1; \
    fi && \
    echo "SHA256 match: $ACTUAL" && \
    chmod +x /app/ravenfabric-agent && \
    rm /app/SHA256SUMS

# Copy manifests
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/

# Build optimized release for the target platform
RUN TARGET=$(cat /tmp/rust_target.txt) && \
    rustup target add "$TARGET" && \
    cargo build --release --locked --target "$TARGET" && \
    cp "target/$TARGET/release/ravenclaw" /app/ravenclaw

# Stage 2: Runtime (minimal)
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# Copy RavenClaw binary
COPY --from=builder /app/ravenclaw /app/ravenclaw

# Copy RavenFabric agent binary (optional — for swarm/supervisor modes)
COPY --from=builder /app/ravenfabric-agent /app/ravenfabric-agent

# Security: run as non-root, read-only filesystem
USER nonroot

# Environment variables (set via K8s/Docker)
ENV RAVENCLAW_CONFIG=/config/ravenclaw.toml
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/app/ravenclaw", "--version"]

ENTRYPOINT ["/app/ravenclaw"]
CMD ["--mode", "single"]
