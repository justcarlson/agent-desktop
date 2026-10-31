#!/usr/bin/env bash
#
# Proves scripts/ci-scope.sh in both directions: a documentation-only change
# skips the Rust jobs, every other change runs them, and the ci-ok verdict
# fails on any failure or on a skip the scope did not allow.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCOPE="$ROOT/scripts/ci-scope.sh"
WORK=$(mktemp -d "${TMPDIR:-/tmp}/ci-scope-selftest.XXXXXX")
trap 'rm -rf -- "$WORK"' EXIT

mkdir -p "$WORK/src/cli" "$WORK/docs/plans" "$WORK/apps/docs/content"
printf 'include_str!("../../docs/json-output.md")\n' > "$WORK/src/cli/drift_tests.rs"

failures=0

expect_scope() {
  local want=$1 name=$2 got
  got=$(printf '%s' "$3" | "$SCOPE" scope "$WORK")
  if [ "$got" = "code=$want" ]; then
    echo "ok   scope: $name"
  else
    echo "FAIL scope: $name: want code=$want, got $got"
    failures=$((failures + 1))
  fi
}

expect_verdict() {
  local want=$1 name=$2 code=$3 needs=$4 got=0
  printf '%s' "$needs" | "$SCOPE" verdict "$code" changes fmt >/dev/null || got=$?
  if { [ "$want" = pass ] && [ "$got" -eq 0 ]; } || { [ "$want" = fail ] && [ "$got" -ne 0 ]; }; then
    echo "ok   verdict: $name"
  else
    echo "FAIL verdict: $name: want $want, exit $got"
    failures=$((failures + 1))
  fi
}

expect_scope false "docs site page" "apps/docs/content/guide.mdx"
expect_scope false "plan no check reads" "docs/plans/plan.md"
expect_scope false "two docs-only paths" $'apps/docs/content/a.mdx\ndocs/plans/plan.md\n'
expect_scope true "docs page a test reads" "docs/json-output.md"
expect_scope true "readme a test reads" "README.md"
expect_scope true "skill compiled into the binary" "skills/agent-desktop/SKILL.md"
expect_scope true "rust source" "crates/core/src/lib.rs"
expect_scope true "workflow" ".github/workflows/ci.yml"
expect_scope true "docs page next to rust" $'apps/docs/content/a.mdx\ncrates/core/src/lib.rs\n'
expect_scope true "empty change list" ""

ok='{"changes":{"result":"success"},"fmt":{"result":"success"},"test":{"result":"success"}}'
skipped='{"changes":{"result":"success"},"fmt":{"result":"success"},"test":{"result":"skipped"}}'
expect_verdict pass "every job passed" true "$ok"
expect_verdict pass "rust job skipped for docs" false "$skipped"
expect_verdict fail "rust job skipped for code" true "$skipped"
expect_verdict fail "always job skipped" false '{"changes":{"result":"success"},"fmt":{"result":"skipped"}}'
expect_verdict fail "a job failed" false '{"changes":{"result":"success"},"fmt":{"result":"success"},"test":{"result":"failure"}}'
expect_verdict fail "a job was cancelled" true '{"changes":{"result":"success"},"fmt":{"result":"success"},"test":{"result":"cancelled"}}'
expect_verdict fail "no jobs" true '{}'

if [ "$failures" -ne 0 ]; then
  echo "ci-scope self-test: $failures failed"
  exit 1
fi
echo "ci-scope self-test: all passed"
