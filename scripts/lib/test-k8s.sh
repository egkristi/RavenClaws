#!/usr/bin/env bash
# =============================================================================
# RavenClaws Verification — Kubernetes Tests (Orbstack K8s)
# =============================================================================
# Tests K8s deployment: cluster connectivity, manifest application,
# pod startup, config loading, LLM connectivity, resource limits,
# security context, liveness/readiness probes.
# =============================================================================

test_kubernetes() {
    log_step "5. Kubernetes Verification (Orbstack K8s)"

    if ! check_prereq kubectl; then return; fi

    local K8S_NAMESPACE="ravenclaws-test"
    local K8S_DEPLOY="$PROJECT_DIR/k8s/deployment-test.yaml"

    # ── Cluster connectivity ───────────────────────────────────────────────
    log_sub "Cluster connectivity"
    run_test "K8s cluster connectivity" kubectl cluster-info
    run_test "K8s node is ready" \
        bash -c 'kubectl get nodes -o jsonpath="{.items[0].status.conditions[?(@.type==\"Ready\")].status}" 2>&1 | grep -q True'

    # ── Ensure Docker image is available ───────────────────────────────────
    if ! check_docker_image; then
        log_detail "Building Docker image first..."
        docker build -t "$DOCKER_TAG" "$PROJECT_DIR" >/dev/null 2>&1 || true
        if ! check_docker_image; then
            log_skip "Cannot build Docker image — skipping K8s tests"
            return
        fi
    fi

    # ── Clean up any previous test namespace ───────────────────────────────
    kubectl delete namespace "$K8S_NAMESPACE" --ignore-not-found --wait=true >/dev/null 2>&1 || true

    # ── Apply manifests ────────────────────────────────────────────────────
    log_sub "Manifest application"
    run_test "K8s manifests apply" kubectl apply -f "$K8S_DEPLOY"

    # ── Wait for pod ───────────────────────────────────────────────────────
    log_sub "Pod lifecycle"
    log_detail "Waiting for pod to be scheduled..."
    local pod_name=""
    for i in $(seq 1 15); do
        pod_name=$(kubectl -n "$K8S_NAMESPACE" get pods -l app.kubernetes.io/name=ravenclaws -o name 2>/dev/null | head -1)
        if [[ -n "$pod_name" ]]; then
            log_detail "Pod scheduled after ${i}s"
            break
        fi
        sleep 1
    done

    if [[ -z "$pod_name" ]]; then
        log_fail "K8s pod was never scheduled"
        kubectl -n "$K8S_NAMESPACE" describe pods 2>&1 | tail -20 | sed 's/^/    /'
        kubectl delete namespace "$K8S_NAMESPACE" --ignore-not-found >/dev/null 2>&1 || true
        return
    fi

    # Wait for pod to complete (it's a one-shot agent)
    log_detail "Waiting for pod to complete (up to 60s)..."
    local pod_status=""
    for i in $(seq 1 30); do
        pod_status=$(kubectl -n "$K8S_NAMESPACE" get "$pod_name" -o jsonpath='{.status.phase}' 2>/dev/null)
        if [[ "$pod_status" == "Running" ]] || [[ "$pod_status" == "Succeeded" ]]; then
            log_detail "Pod status: ${pod_status} after ${i}s"
            break
        fi
        sleep 2
    done

    if [[ "$pod_status" != "Running" ]] && [[ "$pod_status" != "Succeeded" ]]; then
        log_fail "Pod did not reach Running/Succeeded state (status: ${pod_status})"
        kubectl -n "$K8S_NAMESPACE" describe "$pod_name" 2>&1 | tail -20 | sed 's/^/    /'
        kubectl delete namespace "$K8S_NAMESPACE" --ignore-not-found >/dev/null 2>&1 || true
        return
    fi

    run_test "K8s pod reached Running or Succeeded" true

    # ── Pod logs ───────────────────────────────────────────────────────────
    log_sub "Pod logs verification"
    
    # Wait a moment for logs to be available
    sleep 2
    
    local pod_logs_file="$RESULTS_DIR/${TIMESTAMP}-k8s-pod-logs.log"
    kubectl -n "$K8S_NAMESPACE" logs "$pod_name" --tail=30 > "$pod_logs_file" 2>&1 || true

    if [[ ! -s "$pod_logs_file" ]]; then
        log_fail "K8s pod logs are empty"
        kubectl delete namespace "$K8S_NAMESPACE" --ignore-not-found >/dev/null 2>&1 || true
        return
    fi

    run_test "K8s pod logs: RavenClaws starting" \
        bash -c "grep -q 'RavenClaws starting' \"$pod_logs_file\""
    run_test "K8s pod logs: Configuration loaded" \
        bash -c "grep -q 'Configuration loaded' \"$pod_logs_file\""
    run_test "K8s pod logs: LLM client initialized" \
        bash -c "grep -q 'LLM client initialized' \"$pod_logs_file\""
    run_test "K8s pod logs: Provider ready" \
        bash -c "grep -q 'Provider ready' \"$pod_logs_file\""

    # Check if LLM request succeeded or failed
    if grep -q 'Agent response received' "$pod_logs_file"; then
        log_ok "K8s pod — LLM chat response received"
        PASS=$((PASS + 1))
    elif grep -q 'LLM request failed' "$pod_logs_file"; then
        log_detail "K8s pod — LLM request failed (expected if no in-cluster LiteLLM)"
        log_detail "This is informational — the pod started and loaded config correctly"
    fi

    # ── Resource limits ────────────────────────────────────────────────────
    log_sub "Resource configuration"
    run_test "K8s resource limits configured" \
        bash -c "kubectl -n $K8S_NAMESPACE get pods -l app.kubernetes.io/name=ravenclaws -o jsonpath='{.items[0].spec.containers[0].resources.limits}' 2>&1 | grep -q '.'"

    # ── Security context ───────────────────────────────────────────────────
    log_sub "Security context"
    run_test "K8s container runs as non-root user" \
        bash -c "kubectl -n $K8S_NAMESPACE get pods -l app.kubernetes.io/name=ravenclaws -o jsonpath='{.items[0].spec.containers[0].securityContext.runAsUser}' 2>&1 | grep -q '65532'"
    run_test "K8s read-only root filesystem" \
        bash -c "kubectl -n $K8S_NAMESPACE get pods -l app.kubernetes.io/name=ravenclaws -o jsonpath='{.items[0].spec.containers[0].securityContext.readOnlyRootFilesystem}' 2>&1 | grep -q 'true'"
    run_test "K8s all capabilities dropped" \
        bash -c "kubectl -n $K8S_NAMESPACE get pods -l app.kubernetes.io/name=ravenclaws -o jsonpath='{.items[0].spec.containers[0].securityContext.capabilities.drop}' 2>&1 | grep -q 'ALL'"

    # ── ConfigMap ──────────────────────────────────────────────────────────
    log_sub "ConfigMap verification"
    run_test "K8s ConfigMap exists" \
        kubectl -n "$K8S_NAMESPACE" get configmap ravenclaws-config
    run_test "K8s ConfigMap has valid config" \
        bash -c "kubectl -n $K8S_NAMESPACE get configmap ravenclaws-config -o go-template='{{index .data \"ravenclaws.toml\"}}' 2>&1 | grep -q 'endpoint'"

    # ── Cleanup ────────────────────────────────────────────────────────────
    log_sub "Cleanup"
    kubectl delete namespace "$K8S_NAMESPACE" --ignore-not-found --wait=true >/dev/null 2>&1 || true
    log_detail "K8s namespace '$K8S_NAMESPACE' removed"
}

# Run standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    source "$(dirname "$0")/common.sh"
    init_verification
    test_kubernetes
    print_summary "Kubernetes"
fi
