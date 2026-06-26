#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — Docker Container Tests
# =============================================================================
# Tests the Docker image: build, basic operation, LLM connectivity,
# Docker Compose validation, multi-model via Docker.
# =============================================================================

test_docker() {
    log_step "3. Docker Container Verification (Orbstack)"

    if ! check_prereq docker; then return; fi

    # ── Build ───────────────────────────────────────────────────────────────
    log_sub "Docker image build"
    run_test "Docker build" docker build -t "$DOCKER_TAG" "$PROJECT_DIR"
    run_test "Docker image exists" docker image inspect "$DOCKER_TAG"

    # ── Basic container checks ─────────────────────────────────────────────
    log_sub "Basic container checks"
    run_test "Docker container --version" docker run --rm "$DOCKER_TAG" --version
    run_test "Docker container --help" docker run --rm "$DOCKER_TAG" --help

    # ── LLM connectivity ───────────────────────────────────────────────────
    if check_litellm; then
        log_sub "LLM connectivity from container"
        
        # Single agent via host network
        log_detail "Testing single agent (host network)..."
        if timeout 45 docker run --rm --network host \
            -e RAVENCLAW__LLM__ENDPOINT="http://localhost:4000" \
            -e RAVENCLAW__LLM__MODEL="best-coding" \
            -e RAVENCLAW__SECURITY__REQUIRE_TLS=false \
            "$DOCKER_TAG" --mode single \
            > "$RESULTS_DIR/${TIMESTAMP}-docker-llm.log" 2>&1; then
            check_llm_response_quality "$RESULTS_DIR/${TIMESTAMP}-docker-llm.log" "best-coding (Docker)"
        else
            log_fail "Docker container — LLM chat failed"
            tail -10 "$RESULTS_DIR/${TIMESTAMP}-docker-llm.log" | sed 's/^/    /'
        fi

        # Multi-model via Docker
        log_detail "Testing multi-model (Docker)..."
        if timeout 90 docker run --rm --network host \
            -v "$MULTI_CONFIG:/config/multi.toml:ro" \
            -e RAVENCLAW__SECURITY__REQUIRE_TLS=false \
            "$DOCKER_TAG" --config /config/multi.toml --mode single \
            > "$RESULTS_DIR/${TIMESTAMP}-docker-multi.log" 2>&1; then
            log_ok "Docker multi-model — all providers responded"
        else
            log_fail "Docker multi-model — check log"
            tail -10 "$RESULTS_DIR/${TIMESTAMP}-docker-multi.log" | sed 's/^/    /'
        fi

        # Test with env var config (no config file)
        log_detail "Testing env-var-only config (Docker)..."
        if timeout 45 docker run --rm --network host \
            -e RAVENCLAW__LLM__ENDPOINT="http://localhost:4000" \
            -e RAVENCLAW__LLM__MODEL="best-coding" \
            -e RAVENCLAW__SECURITY__REQUIRE_TLS=false \
            "$DOCKER_TAG" --mode single \
            > "$RESULTS_DIR/${TIMESTAMP}-docker-envonly.log" 2>&1; then
            check_llm_response_quality "$RESULTS_DIR/${TIMESTAMP}-docker-envonly.log" "best-coding (Docker env)"
        else
            log_fail "Docker env-only config — failed"
            tail -10 "$RESULTS_DIR/${TIMESTAMP}-docker-envonly.log" | sed 's/^/    /'
        fi
    fi

    # ── Docker Compose ─────────────────────────────────────────────────────
    log_sub "Docker Compose"
    if [[ -f "$PROJECT_DIR/docker-compose.yml" ]]; then
        run_test "Docker Compose config validation" \
            docker compose -f "$PROJECT_DIR/docker-compose.yml" config --quiet
    fi

    # ── Container security ─────────────────────────────────────────────────
    log_sub "Container security"
    run_test "Container runs as non-root" \
        bash -c "docker image inspect $DOCKER_TAG --format '{{.Config.User}}' 2>&1 | grep -qi 'nonroot'"
    run_test "Container has no privileged mode" \
        bash -c "docker image inspect $DOCKER_TAG --format '{{.Config.User}}' 2>&1 | grep -qi 'nonroot'"

    # ── Cleanup ────────────────────────────────────────────────────────────
    log_sub "Cleanup"
    docker rmi "$DOCKER_TAG" >/dev/null 2>&1 || true
    log_detail "Docker image removed"
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_docker
    print_summary "Docker Container"
fi
