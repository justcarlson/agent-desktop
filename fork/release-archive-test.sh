#!/usr/bin/env bash
#
# Tests fork/release-archive.sh without a build. Every case runs the script
# with --check, which validates the arguments and the source, prints the plan
# and stops before it downloads or builds anything. The cases use a small
# throwaway Git repository, never the real checkout.
#
# Usage: fork/release-archive-test.sh
# Exit:  0 every case passed, 1 a case failed

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$HERE/release-archive.sh"
WORK=$(mktemp -d "${TMPDIR:-/tmp}/release-archive-test.XXXXXX")
trap 'rm -rf -- "$WORK"' EXIT

failures=0
LAST_OUTPUT=

pass() { echo "ok - $1"; }
fail() {
  echo "not ok - $1" >&2
  failures=$((failures + 1))
}

run_script() {
  local got=0
  LAST_OUTPUT=$(bash "$SCRIPT" "$@" 2>&1) || got=$?
  return "$got"
}

expect_exit() {
  local want=$1 name=$2 got=0
  shift 2
  run_script "$@" || got=$?
  if [ "$got" -eq "$want" ]; then
    pass "$name"
  else
    printf '%s\n' "$LAST_OUTPUT" >&2
    fail "$name: exit $got, expected $want"
  fi
}

expect_output() {
  local name=$1 text=$2
  if printf '%s\n' "$LAST_OUTPUT" | grep -F -- "$text" >/dev/null; then
    pass "$name"
  else
    printf '%s\n' "$LAST_OUTPUT" >&2
    fail "$name: output does not contain '$text'"
  fi
}

make_repo() {
  local dir=$1
  git init -q -b main "$dir"
  git -C "$dir" config user.name test
  git -C "$dir" config user.email test@example.invalid
  printf 'notice\n' > "$dir/THIRD_PARTY_NOTICES.md"
  git -C "$dir" add THIRD_PARTY_NOTICES.md
  git -C "$dir" commit -q -m init
  git -C "$dir" branch hypr/base
}

if [ ! -f "$SCRIPT" ]; then
  fail "fork/release-archive.sh exists"
  echo "1 case failed: the script is missing" >&2
  exit 1
fi

REPO="$WORK/repo"
OUT="$WORK/out"
make_repo "$REPO"
SHA=$(git -C "$REPO" rev-parse HEAD)
VERSION=1.0.2-hypr.0-rc1
ARCHIVE="agent-desktop-v${VERSION}-x86_64-unknown-linux-gnu.tar.gz"

expect_exit 2 "no arguments is a usage error"
expect_exit 2 "an unknown option is a usage error" --bogus "$VERSION"
expect_exit 2 "a leading v in the version is refused" --check --repo "$REPO" --out "$OUT" "v$VERSION"
expect_exit 2 "a version without three numbers is refused" --check --repo "$REPO" --out "$OUT" 1.0
expect_exit 2 "a version with a slash is refused" --check --repo "$REPO" --out "$OUT" "1.0.2-a/b"
expect_exit 2 "an unknown ref is refused" --check --repo "$REPO" --out "$OUT" --ref no/such/ref "$VERSION"
expect_output "the unknown ref is named" "no/such/ref"
expect_exit 2 "a directory that is not a Git checkout is refused" --check --repo "$WORK" --out "$OUT" "$VERSION"

expect_exit 0 "a clean checkout passes the check" --check --repo "$REPO" --out "$OUT" "$VERSION"
expect_output "the check names the archive" "$ARCHIVE"
expect_output "the check names the source SHA" "$SHA"
expect_output "the check names the default ref" "hypr/base"
expect_output "the check names the pinned image" "ubuntu:22.04@sha256:0a9887931b0e38af2e3487ed39e29436b0535c4f3f52e57745baaf79d1898956"
if [ -e "$OUT" ]; then
  fail "the check writes nothing to the output directory"
else
  pass "the check writes nothing to the output directory"
fi

expect_exit 0 "an explicit ref resolves" --check --repo "$REPO" --out "$OUT" --ref main "$VERSION"
expect_output "the explicit ref names its SHA" "$SHA"

printf 'edit\n' >> "$REPO/THIRD_PARTY_NOTICES.md"
expect_exit 1 "a modified tracked file is refused" --check --repo "$REPO" --out "$OUT" "$VERSION"
expect_output "the refusal says the tree is dirty" "dirty"
git -C "$REPO" checkout -q -- THIRD_PARTY_NOTICES.md

printf 'new\n' > "$REPO/untracked.txt"
expect_exit 1 "an untracked file is refused" --check --repo "$REPO" --out "$OUT" "$VERSION"
rm -f "$REPO/untracked.txt"

printf 'staged\n' > "$REPO/staged.txt"
git -C "$REPO" add staged.txt
expect_exit 1 "a staged file is refused" --check --repo "$REPO" --out "$OUT" "$VERSION"
git -C "$REPO" rm -q --cached staged.txt
rm -f "$REPO/staged.txt"

mkdir -p "$OUT"
: > "$OUT/$ARCHIVE"
expect_exit 1 "an existing archive is never overwritten" --check --repo "$REPO" --out "$OUT" "$VERSION"
expect_output "the refusal names the existing archive" "$ARCHIVE"
rm -rf "$OUT"

expect_exit 0 "the clean checkout passes again after the cases" --check --repo "$REPO" --out "$OUT" "$VERSION"

if grep -nE '(^|[^[:alnum:]_-])(git[[:space:]]+(push|tag)|gh[[:space:]]+(release|api|workflow)|npm[[:space:]]+publish|cargo[[:space:]]+publish|curl[^|]*(-T|--upload-file|-X[[:space:]]*(PUT|POST)))' "$SCRIPT" >"$WORK/publish.txt"; then
  cat "$WORK/publish.txt" >&2
  fail "the script holds no push, tag, publish or upload command"
else
  pass "the script holds no push, tag, publish or upload command"
fi

if [ "$failures" -ne 0 ]; then
  echo "$failures case(s) failed" >&2
  exit 1
fi
echo "all cases passed"
