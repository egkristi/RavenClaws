# RavenClaws Makefile
# Container build helpers

IMAGE_NAME ?= ravenclaws
REGISTRY   ?= ghcr.io/egkristi
VERSION    ?= $(shell cargo metadata --format-version 1 --no-deps | \
              python3 -c "import sys,json; print(json.load(sys.stdin)['packages'][0]['version'])" 2>/dev/null || echo "dev")

.PHONY: help build buildx scan sbom clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build container for host architecture
	docker build -t $(IMAGE_NAME):latest .

buildx: ## Build multi-arch container (linux/amd64 + linux/arm64)
	docker buildx build \
		--platform linux/amd64,linux/arm64 \
		-t $(IMAGE_NAME):latest \
		-t $(REGISTRY)/$(IMAGE_NAME):$(VERSION) \
		--load \
		.

push: ## Build and push multi-arch container to registry
	docker buildx build \
		--platform linux/amd64,linux/arm64 \
		-t $(REGISTRY)/$(IMAGE_NAME):latest \
		-t $(REGISTRY)/$(IMAGE_NAME):$(VERSION) \
		--push \
		.

scan: ## Run Trivy vulnerability scan on the image
	trivy image --severity CRITICAL,HIGH --exit-code 1 $(IMAGE_NAME):latest

sbom: ## Generate SBOM with Syft
	syft $(IMAGE_NAME):latest -o spdx-json=sbom.spdx.json

compose-up: ## Start development environment
	docker compose up -d

compose-down: ## Stop development environment
	docker compose down

compose-logs: ## View development logs
	docker compose logs -f

clean: ## Clean build artifacts
	docker builder prune -f
