# RavenClaw — Multi-stage build for minimal production image
# Supports: linux/amd64, linux/arm64
# Usage: docker buildx build --platform linux/amd64,linux/arm64 -t ravenclaw:latest .

# Stage 1: Builder
# syntax=docker/dockerfile:1
ARG TARGETPLATFORM
ARG BUILDPLATFORM

FROM --platform=$BUILDPLATFORM rust:1.82-slim-bookworm AS builder

WORKDIR /app

# Install dependencies and cross-compilation tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/

# Build optimized release for the target platform
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        "linux/amd64")  TARGET="x86_64-unknown-linux-gnu" ;; \
        "linux/arm64")  TARGET="aarch64-unknown-linux-gnu" ;; \
        *)              TARGET="x86_64-unknown-linux-gnu" ;; \
    esac && \
    rustup target add "$TARGET" && \
    cargo build --release --locked --target "$TARGET"

# Stage 2: Runtime (minimal)
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# Copy binary from builder
ARG TARGETPLATFORM
COPY --from=builder --chown=nonroot:nonroot \
    /app/target/$(case "$TARGETPLATFORM" in \
        "linux/amd64") echo "x86_64-unknown-linux-gnu" ;; \
        "linux/arm64") echo "aarch64-unknown-linux-gnu" ;; \
        *) echo "x86_64-unknown-linux-gnu" ;; \
    esac)/release/ravenclaw .

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
