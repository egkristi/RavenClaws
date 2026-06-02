# RavenClaw — Multi-stage build for minimal production image
# Supports: linux/amd64, linux/arm64
# Usage: docker buildx build --platform linux/amd64,linux/arm64 -t ravenclaw:latest .
# syntax=docker/dockerfile:1

# Stage 1: Builder
ARG TARGETPLATFORM
ARG BUILDPLATFORM

FROM --platform=$BUILDPLATFORM rust:1.86-slim-bookworm AS builder

WORKDIR /app

# Map TARGETPLATFORM to Rust target triple
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        "linux/amd64")  echo "x86_64-unknown-linux-gnu" > /tmp/rust_target.txt ;; \
        "linux/arm64")  echo "aarch64-unknown-linux-gnu" > /tmp/rust_target.txt ;; \
        *)              echo "x86_64-unknown-linux-gnu" > /tmp/rust_target.txt ;; \
    esac

# Install dependencies and cross-compilation tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

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

# Copy binary from known path in builder
COPY --from=builder /app/ravenclaw /app/ravenclaw

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
