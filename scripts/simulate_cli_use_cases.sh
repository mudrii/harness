#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${HARNESS_BIN:-}" ]]; then
  RUNNER=("$HARNESS_BIN")
elif [[ -x "./target/debug/harness" ]]; then
  RUNNER=("./target/debug/harness")
else
  RUNNER=("cargo" "run" "--quiet" "--")
fi

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command '$1' not found" >&2
    exit 1
  fi
}

require_cmd git
require_cmd mktemp

TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

PASS=0
FAIL=0

run_case() {
  local name="$1"
  local expected_code="$2"
  local stdout_contains="$3"
  local stderr_contains="$4"
  shift 4

  local out_file="$TMP_ROOT/${name}.out"
  local err_file="$TMP_ROOT/${name}.err"

  set +e
  "$@" >"$out_file" 2>"$err_file"
  local code=$?
  set -e

  local ok=true

  if [[ "$code" -ne "$expected_code" ]]; then
    ok=false
    echo "[FAIL] $name: expected exit $expected_code, got $code"
  fi

  if [[ -n "$stdout_contains" ]] && ! grep -Fq "$stdout_contains" "$out_file"; then
    ok=false
    echo "[FAIL] $name: stdout missing '$stdout_contains'"
  fi

  if [[ -n "$stderr_contains" ]] && ! grep -Fq "$stderr_contains" "$err_file"; then
    ok=false
    echo "[FAIL] $name: stderr missing '$stderr_contains'"
  fi

  if [[ "$ok" == true ]]; then
    PASS=$((PASS + 1))
    echo "[PASS] $name"
  else
    FAIL=$((FAIL + 1))
    echo "  stdout:"
    sed -n '1,40p' "$out_file" | sed 's/^/    /'
    echo "  stderr:"
    sed -n '1,40p' "$err_file" | sed 's/^/    /'
  fi
}

new_repo() {
  local repo="$1"
  mkdir -p "$repo"
  git -C "$repo" init >/dev/null 2>&1
}

case_analyze_non_git() {
  local repo="$TMP_ROOT/repo_non_git"
  mkdir -p "$repo"
  cat >"$repo/harness.toml" <<'EOF'
[project]
name = "sample"
profile = "general"
EOF
  run_case "analyze_non_git" 3 "" "not a git repository" \
    "${RUNNER[@]}" analyze "$repo"
}

case_analyze_valid_json() {
  local repo="$TMP_ROOT/repo_valid"
  new_repo "$repo"
  mkdir -p "$repo/docs/context"
  cat >"$repo/AGENTS.md" <<'EOF'
# Agents
map
EOF
  cat >"$repo/README.md" <<'EOF'
Architecture reference: ARCHITECTURE.md
EOF
  cat >"$repo/ARCHITECTURE.md" <<'EOF'
# Architecture
EOF
  cat >"$repo/docs/context/INDEX.md" <<'EOF'
index
EOF
  cat >"$repo/harness.toml" <<'EOF'
[project]
name = "sample"
profile = "general"

[verification]
required = ["cargo check"]
pre_completion_required = true
loop_guard_enabled = true
EOF
  run_case "analyze_valid_json" 0 "\"overall_score\"" "" \
    "${RUNNER[@]}" analyze "$repo" --format json
}

case_analyze_malformed_config() {
  local repo="$TMP_ROOT/repo_malformed"
  new_repo "$repo"
  echo "[project" >"$repo/harness.toml"
  run_case "analyze_malformed_config" 3 "" "config parse error" \
    "${RUNNER[@]}" analyze "$repo"
}

case_suggest_export_diff() {
  local repo="$TMP_ROOT/repo_suggest"
  new_repo "$repo"
  run_case "suggest_export_diff" 0 "plan file:" "" \
    "${RUNNER[@]}" suggest "$repo" --export-diff

  if compgen -G "$repo/.harness/plans/*.json" >/dev/null; then
    echo "[PASS] suggest_export_diff_plan_file_created"
    PASS=$((PASS + 1))
  else
    echo "[FAIL] suggest_export_diff_plan_file_created: no plan json generated"
    FAIL=$((FAIL + 1))
  fi
}

case_apply_preview_clean_repo() {
  local repo="$TMP_ROOT/repo_apply_clean"
  new_repo "$repo"
  run_case "apply_preview_clean_repo" 0 "scope:" "" \
    "${RUNNER[@]}" apply "$repo" --plan-all --apply-mode preview
}

case_apply_preview_dirty_rejected() {
  local repo="$TMP_ROOT/repo_apply_dirty"
  new_repo "$repo"
  echo "dirty" >"$repo/untracked.txt"
  run_case "apply_preview_dirty_rejected" 3 "" "working tree is dirty" \
    "${RUNNER[@]}" apply "$repo" --plan-all --apply-mode preview
}

case_lint_deprecated_blocking() {
  local repo="$TMP_ROOT/repo_lint_deprecated"
  new_repo "$repo"
  cat >"$repo/harness.toml" <<'EOF'
[project]
name = "sample"
profile = "general"

[tools.deprecated]
deprecated = ["grep"]
EOF
  run_case "lint_deprecated_blocking" 2 "tools.deprecated" "" \
    "${RUNNER[@]}" lint "$repo"
}

echo "Harness CLI simulation runner"
echo "runner: ${RUNNER[*]}"
echo

case_analyze_non_git
case_analyze_valid_json
case_analyze_malformed_config
case_suggest_export_diff
case_apply_preview_clean_repo
case_apply_preview_dirty_rejected
case_lint_deprecated_blocking

echo
echo "Simulation summary: pass=$PASS fail=$FAIL"
if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
