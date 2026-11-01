#!/usr/bin/env bash
# Tests fork/check-upstream-ancestry.sh against throwaway repositories.
# Usage: bash fork/check-upstream-ancestry-test.sh
set -uo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
checker="$script_dir/check-upstream-ancestry.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=fixture GIT_AUTHOR_EMAIL=fixture@example.invalid
export GIT_COMMITTER_NAME=fixture GIT_COMMITTER_EMAIL=fixture@example.invalid

failures=0
pass() { printf 'ok   %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1"; failures=$((failures + 1)); }

repo="$work/repo"
git init -q -b main "$repo"
cd "$repo" || exit 1
echo base > README.md
git add README.md
git commit -q -m "chore: upstream base"
git branch upstream-main
git update-ref refs/remotes/upstream/main upstream-main

commit_file() {
  mkdir -p "$(dirname "$1")"
  echo "$2" > "$1"
  git add "$1"
  git commit -q -m "$3"
}

start_branch() { git switch -q -c "$1" upstream-main; }

run_checker() {
  output=$(bash "$checker" "$@" 2>&1)
  status=$?
}

expect_pass() {
  local name=$1
  shift
  run_checker "$@"
  if [ "$status" -eq 0 ]; then pass "$name"; else fail "$name: exit $status: $output"; fi
}

expect_fail() {
  local name=$1 needle=$2
  shift 2
  run_checker "$@"
  if [ "$status" -eq 1 ] && grep -qF -- "$needle" <<<"$output"; then
    pass "$name"
  else
    fail "$name: exit $status, wanted 1 and '$needle': $output"
  fi
}

start_branch clean
commit_file src/lib.rs "fn a() {}" "fix: keep the upstream change"
commit_file docs/test-fork-notes.md "x" "docs: mention test(fork) commits in the middle of a subject"
expect_pass "clean branch passes against the default base" clean
expect_pass "clean branch passes against an explicit base" clean upstream-main

start_branch adds-fork
commit_file src/lib.rs "fn b() {}" "fix: an upstream change"
commit_file fork/x "x" "chore: add a fork file"
expect_fail "branch that adds fork/x fails" "fork/x" adds-fork

start_branch adds-then-removes-fork
commit_file fork/x "x" "chore: add a fork file"
git rm -q fork/x
git commit -q -m "chore: remove the fork file"
expect_fail "branch that adds then removes fork/x still fails" "fork/x" adds-then-removes-fork
if grep -q "final diff" <<<"$output"; then
  fail "removed fork/x is not reported in the final diff: $output"
else
  pass "removed fork/x is reported only through its commits"
fi

start_branch test-fork-commit
commit_file src/lib.rs "fn c() {}" "test(fork): run the vm leg for this slice"
expect_fail "branch with a test(fork) commit fails" "test(fork): run the vm leg" test-fork-commit

start_branch vm-lane
commit_file test/all "x" "test: run the tiers in a vm"
expect_fail "branch that adds test/ fails" "test/all" vm-lane

start_branch vm-workflow
commit_file .github/workflows/omarchy-vm.yml "x" "ci: add the vm workflow"
expect_fail "branch that adds the vm workflow fails" ".github/workflows/omarchy-vm.yml" vm-workflow

start_branch attached-tier
commit_file tests/e2e-linux/runner/attached_session.py "x" "test: attached tier"
expect_fail "branch that adds an attached-tier file fails" "tests/e2e-linux/runner/attached_session.py" attached-tier

start_branch actionlint-label
commit_file .github/actionlint.yaml "self-hosted-runner: {labels: [ubuntu-26.04]}" "ci: teach actionlint a label"
expect_fail "branch that adds the fork runner label fails" "ubuntu-26.04" actionlint-label

start_branch renames-fork
commit_file fork/y "x" "chore: add a fork file"
mkdir -p scripts
git mv fork/y scripts/y
git commit -q -m "chore: move the fork file"
expect_fail "branch that renames a fork file out of fork/ fails" "fork/y" renames-fork

start_branch cherry-picked-fork-commit
echo z > z.rs
git add z.rs
git commit -q -m "test: run the linux e2e tiers" -m "(cherry picked from commit 8231879200000000000000000000000000000000)"
expect_fail "branch with a cherry-picked fork-only commit fails" "82318792" cherry-picked-fork-commit

start_branch many-findings
commit_file fork/a "x" "chore: one"
commit_file fork/b "x" "test(fork): two"
expect_fail "every finding is listed, not just the first" "fork/a" many-findings
if grep -qF "fork/b" <<<"$output" && grep -qF "test(fork): two" <<<"$output"; then
  pass "the list holds each path and each commit"
else
  fail "the list is incomplete: $output"
fi

run_checker
if [ "$status" -eq 2 ]; then pass "missing branch argument exits 2"; else fail "no-argument exit $status"; fi
run_checker no-such-branch
if [ "$status" -eq 2 ]; then pass "unknown branch exits 2"; else fail "unknown branch exit $status"; fi

if grep -nE '\bgit (fetch|push|pull|remote|ls-remote)\b|\bgh |curl|wget' "$checker" >/dev/null 2>&1; then
  fail "the checker reaches a remote"
elif [ -f "$checker" ]; then
  pass "the checker only reads local git"
else
  fail "the checker does not exist"
fi

if [ "$failures" -ne 0 ]; then
  printf '%d check(s) failed\n' "$failures"
  exit 1
fi
printf 'all checks passed\n'
