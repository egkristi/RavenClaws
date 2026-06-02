# RavenClaw Docker Bake configuration
# Multi-arch build with automatic tagging
# Usage: docker buildx bake -f docker-bake.hcl --push

variable "IMAGE_NAME" {
  default = "ravenclaw"
}

variable "REGISTRY" {
  default = "ghcr.io/egkristi"
}

variable "VERSION" {
  default = ""
}

variable "GIT_SHA" {
  default = ""
}

# Auto-detect version from Cargo.toml
target "metadata" {
  description = "Extract version from Cargo.toml"
}

target "default" {
  description = "Build multi-arch container image"
  dockerfile = "Dockerfile"
  platforms = ["linux/amd64", "linux/arm64"]
  args = {
    RAVENFABRIC_VERSION = "v0.25.1"
  }
  tags = concat(
    ["${REGISTRY}/${IMAGE_NAME}:latest"],
    VERSION != "" ? [
      "${REGISTRY}/${IMAGE_NAME}:${VERSION}",
      "${REGISTRY}/${IMAGE_NAME}:${regex_replace(VERSION, "^(\\d+\\.\\d+).*", "$1")}",
      "${REGISTRY}/${IMAGE_NAME}:${regex_replace(VERSION, "^(\\d+).*", "$1")}",
    ] : [],
    GIT_SHA != "" ? ["${REGISTRY}/${IMAGE_NAME}:sha-${GIT_SHA}"] : []
  )
  labels = {
    "org.opencontainers.image.title" = "RavenClaw"
    "org.opencontainers.image.description" = "Lightweight, secure Rust agent framework"
    "org.opencontainers.image.source" = "https://github.com/egkristi/RavenClaw"
    "org.opencontainers.image.licenses" = "AGPL-3.0-or-later"
    "org.opencontainers.image.version" = VERSION != "" ? VERSION : "dev"
    "org.opencontainers.image.revision" = GIT_SHA != "" ? GIT_SHA : "dev"
  }
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}
