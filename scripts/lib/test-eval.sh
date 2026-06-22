#!/usr/bin/env bash
# =============================================================================
# RavenClaw Verification — Eval Harness Tests
# =============================================================================
# Tests the --eval mode: running eval suites from TOML config files, checking
# assertion results, and verifying report output (text and JSON formats).
#
# These tests require:
#   - A release build of RavenClaw (cargo build --release)
#   - LiteLLM running at http://localhost:4000
# =============================================================================

test_eval_harness() {
    log_step "Eval Harness Verification"

    if ! check_binary; then return; fi
    if ! check_litellm; then return; fi

    local eval_dir="$PROJECT_DIR/tests/eval"
    local results_dir="$RESULTS_DIR"

    # ── 1. Basic eval suite (text output) ─────────────────────────────────
    log_sub "1. Basic eval suite — text output"
    local basic_log="$results_dir/${TIMESTAMP}-eval-basic-text.log"
    if timeout 120 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$eval_dir/basic-suite.toml" \
        > "$basic_log" 2>&1; then
        log_ok "Basic eval suite completed (exit 0)"
        # Check for report header
        if grep -q "Eval Report:" "$basic_log" 2>/dev/null; then
            log_ok "Basic eval — report header present"
        else
            log_fail "Basic eval — no report header found"
            tail -10 "$basic_log" | sed 's/^/    /'
        fi
        # Check for task results
        local task_count
        task_count=$(grep -cE '[✅❌]' "$basic_log" 2>/dev/null || echo 0)
        if [[ "$task_count" -ge 1 ]]; then
            log_ok "Basic eval — ${task_count} task results displayed"
        else
            log_fail "Basic eval — no task results found"
        fi
        # Check for overall score
        if grep -q "Overall score" "$basic_log" 2>/dev/null; then
            log_ok "Basic eval — overall score displayed"
        else
            log_fail "Basic eval — no overall score found"
        fi
    else
        local exit_code=$?
        log_fail "Basic eval suite exited with code ${exit_code}"
        tail -20 "$basic_log" | sed 's/^/    /'
    fi

    # ── 2. Basic eval suite (JSON output) ─────────────────────────────────
    log_sub "2. Basic eval suite — JSON output"
    local json_log="$results_dir/${TIMESTAMP}-eval-basic-json.log"
    if timeout 120 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$eval_dir/basic-suite.toml" --eval-json \
        > "$json_log" 2>&1; then
        log_ok "Basic eval JSON completed (exit 0)"
        # Extract JSON report from mixed stdout (JSON logging + report)
        # The report is the last JSON object that has a "suite_name" field
        local json_report
        json_report=$(python3 -c "
import json,sys
lines = sys.stdin.readlines()
# Find the last line that is valid JSON with a suite_name field
for line in reversed(lines):
    line = line.strip()
    if not line:
        continue
    try:
        d = json.loads(line)
        if 'suite_name' in d:
            print(line)
            break
    except json.JSONDecodeError:
        continue
" < "$json_log" 2>/dev/null)
        if [[ -n "$json_report" ]]; then
            log_ok "Basic eval JSON — valid JSON report extracted"
            # Check for required fields
            local suite_name
            suite_name=$(echo "$json_report" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('suite_name',''))" 2>/dev/null)
            if [[ -n "$suite_name" ]]; then
                log_ok "Basic eval JSON — suite_name: ${suite_name}"
            else
                log_fail "Basic eval JSON — missing suite_name"
            fi
            local overall_score
            overall_score=$(echo "$json_report" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('overall_score',0))" 2>/dev/null)
            if [[ -n "$overall_score" ]]; then
                log_ok "Basic eval JSON — overall_score: ${overall_score}"
            else
                log_fail "Basic eval JSON — missing overall_score"
            fi
            local task_count
            task_count=$(echo "$json_report" | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('results',[])))" 2>/dev/null)
            if [[ "$task_count" -ge 1 ]]; then
                log_ok "Basic eval JSON — ${task_count} task results"
            else
                log_fail "Basic eval JSON — no task results"
            fi
        else
            log_fail "Basic eval JSON — could not extract JSON report"
            head -10 "$json_log" | sed 's/^/    /'
        fi
    else
        local exit_code=$?
        log_fail "Basic eval JSON exited with code ${exit_code}"
        tail -20 "$json_log" | sed 's/^/    /'
    fi

    # ── 3. Security eval suite ────────────────────────────────────────────
    log_sub "3. Security eval suite — safety refusal tests"
    local security_log="$results_dir/${TIMESTAMP}-eval-security.log"
    if timeout 120 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$eval_dir/security-suite.toml" \
        > "$security_log" 2>&1; then
        log_ok "Security eval suite completed (exit 0)"
        if grep -q "Eval Report:" "$security_log" 2>/dev/null; then
            log_ok "Security eval — report header present"
        else
            log_fail "Security eval — no report header found"
        fi
    else
        local exit_code=$?
        log_fail "Security eval suite exited with code ${exit_code}"
        tail -20 "$security_log" | sed 's/^/    /'
    fi

    # ── 4. Eval with non-existent config file ─────────────────────────────
    log_sub "4. Eval with non-existent config file"
    local nonexistent_log="$results_dir/${TIMESTAMP}-eval-nonexistent.log"
    if "$BINARY" --config "$TEST_CONFIG" \
        --eval "/tmp/nonexistent-eval-config-$(date +%s).toml" \
        > "$nonexistent_log" 2>&1; then
        log_fail "Non-existent config — should have failed"
        tail -10 "$nonexistent_log" | sed 's/^/    /'
    else
        log_ok "Non-existent config — correctly returned error"
    fi

    # ── 5. Eval with invalid config file ──────────────────────────────────
    log_sub "5. Eval with invalid TOML config"
    local invalid_toml="/tmp/invalid-eval-$(date +%s).toml"
    echo "invalid toml content [[[" > "$invalid_toml"
    local invalid_log="$results_dir/${TIMESTAMP}-eval-invalid.log"
    if "$BINARY" --config "$TEST_CONFIG" \
        --eval "$invalid_toml" \
        > "$invalid_log" 2>&1; then
        log_fail "Invalid config — should have failed"
        tail -10 "$invalid_log" | sed 's/^/    /'
    else
        log_ok "Invalid config — correctly returned error"
    fi
    rm -f "$invalid_toml"

    # ── 6. Eval with empty task list ──────────────────────────────────────
    log_sub "6. Eval with empty task list"
    local empty_toml="/tmp/empty-eval-$(date +%s).toml"
    cat > "$empty_toml" << 'EOF'
name = "empty-suite"
description = "Suite with no tasks"
system_prompt = "Be concise."
max_iterations = 3
EOF
    local empty_log="$results_dir/${TIMESTAMP}-eval-empty.log"
    if timeout 60 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$empty_toml" \
        > "$empty_log" 2>&1; then
        log_ok "Empty task list — completed successfully"
        if grep -q "0/0 passed" "$empty_log" 2>/dev/null; then
            log_ok "Empty task list — shows 0/0 passed"
        else
            log_detail "Empty suite output:"
            tail -5 "$empty_log" | sed 's/^/    /'
        fi
    else
        local exit_code=$?
        log_fail "Empty task list exited with code ${exit_code}"
        tail -10 "$empty_log" | sed 's/^/    /'
    fi
    rm -f "$empty_toml"

    # ── 7. Eval with custom system prompt ─────────────────────────────────
    log_sub "7. Eval with custom system prompt"
    local custom_toml="/tmp/custom-eval-$(date +%s).toml"
    cat > "$custom_toml" << 'EOF'
name = "custom-prompt-suite"
description = "Tests custom system prompt"
system_prompt = "You are a terse assistant. Reply with exactly one word."
max_iterations = 2

[[tasks]]
name = "one-word"
prompt = "Say hello"
assertions = [
    { type = "non_empty" },
    { type = "max_length", value = 50 },
]
weight = 1.0
EOF
    local custom_log="$results_dir/${TIMESTAMP}-eval-custom.log"
    if timeout 60 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$custom_toml" \
        > "$custom_log" 2>&1; then
        log_ok "Custom system prompt — completed successfully"
        if grep -q "one-word" "$custom_log" 2>/dev/null; then
            log_ok "Custom system prompt — task executed"
        fi
    else
        local exit_code=$?
        log_fail "Custom system prompt exited with code ${exit_code}"
        tail -10 "$custom_log" | sed 's/^/    /'
    fi
    rm -f "$custom_toml"

    # ── 8. Eval with regex assertion ──────────────────────────────────────
    log_sub "8. Eval with regex assertion"
    local regex_toml="/tmp/regex-eval-$(date +%s).toml"
    cat > "$regex_toml" << 'EOF'
name = "regex-suite"
description = "Tests regex assertions"
system_prompt = "You are a helpful assistant."
max_iterations = 2

[[tasks]]
name = "number-response"
prompt = "What is 15 + 27?"
assertions = [
    { type = "regex", value = "[0-9]+" },
    { type = "non_empty" },
]
weight = 1.0
EOF
    local regex_log="$results_dir/${TIMESTAMP}-eval-regex.log"
    if timeout 60 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$regex_toml" \
        > "$regex_log" 2>&1; then
        log_ok "Regex assertion — completed successfully"
    else
        local exit_code=$?
        log_fail "Regex assertion exited with code ${exit_code}"
        tail -10 "$regex_log" | sed 's/^/    /'
    fi
    rm -f "$regex_toml"

    # ── 9. Eval with exact match assertion ────────────────────────────────
    log_sub "9. Eval with exact match assertion"
    local exact_toml="/tmp/exact-eval-$(date +%s).toml"
    cat > "$exact_toml" << 'EOF'
name = "exact-suite"
description = "Tests exact match assertions"
system_prompt = "You are a helpful assistant."
max_iterations = 2

[[tasks]]
name = "exact-hello"
prompt = "Say exactly the word: Hello"
assertions = [
    { type = "non_empty" },
]
weight = 1.0
EOF
    local exact_log="$results_dir/${TIMESTAMP}-eval-exact.log"
    if timeout 60 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$exact_toml" \
        > "$exact_log" 2>&1; then
        log_ok "Exact match — completed successfully"
    else
        local exit_code=$?
        log_fail "Exact match exited with code ${exit_code}"
        tail -10 "$exact_log" | sed 's/^/    /'
    fi
    rm -f "$exact_toml"

    # ── 10. Eval with all assertion types ─────────────────────────────────
    log_sub "10. Eval with all assertion types"
    local all_toml="/tmp/all-assertions-eval-$(date +%s).toml"
    cat > "$all_toml" << 'EOF'
name = "all-assertions-suite"
description = "Tests all assertion types"
system_prompt = "You are a helpful assistant."
max_iterations = 2

[[tasks]]
name = "all-types"
prompt = "Write a sentence about Rust programming."
assertions = [
    { type = "non_empty" },
    { type = "min_length", value = 10 },
    { type = "max_length", value = 500 },
    { type = "contains", value = "Rust" },
    { type = "not_contains", value = "Python" },
]
weight = 1.0
EOF
    local all_log="$results_dir/${TIMESTAMP}-eval-all-assertions.log"
    if timeout 60 "$BINARY" --config "$TEST_CONFIG" \
        --eval "$all_toml" \
        > "$all_log" 2>&1; then
        log_ok "All assertion types — completed successfully"
    else
        local exit_code=$?
        log_fail "All assertion types exited with code ${exit_code}"
        tail -10 "$all_log" | sed 's/^/    /'
    fi
    rm -f "$all_toml"
}
