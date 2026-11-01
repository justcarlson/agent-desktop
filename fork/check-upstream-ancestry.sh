#!/usr/bin/env bash
# Fails when a branch meant for upstream carries fork-only work.
#
# Usage: fork/check-upstream-ancestry.sh <branch> [<upstream base>]
#   <upstream base> defaults to upstream/main.
#
# It reads every commit between the merge-base and the branch tip, and the
# final tree diff. A finding is a fork-only path in any of those diffs, a
# fork-only hunk, a commit subject that starts with test(fork), or a commit
# that is, or was cherry-picked from, a fork-only commit.
#
# It reads local git only. Fetch upstream yourself before you run it.
# Exit codes: 0 clean, 1 findings (listed on stdout), 2 usage or bad ref.
set -euo pipefail

# Fork-only paths. An entry that ends in / matches everything under it.
FORK_ONLY_PATHS=(
  fork/
  test/
  .github/workflows/omarchy-vm.yml
  tests/e2e-linux/oracle/hyprland.py
  tests/e2e-linux/runner/attached.py
  tests/e2e-linux/runner/attached_guard.py
  tests/e2e-linux/runner/attached_session.py
  tests/e2e-linux/runner/second_instance.py
  tests/e2e-linux/runner/socket_fixture.py
  tests/e2e-linux/scenarios/hyprland_cancel.py
  tests/e2e-linux/scenarios/hyprland_common.py
  tests/e2e-linux/scenarios/hyprland_desktop.py
  tests/e2e-linux/test_attached_guard.py
  tests/e2e-linux/test_hyprland_fixtures.py
  tests/e2e-linux/test_hyprland_lifecycle.py
  tests/e2e-linux/test_hyprland_membership.py
  tests/e2e-linux/test_hyprland_oracle.py
  tests/e2e-linux/test_hyprland_registry.py
  tests/e2e-linux/test_hyprland_scenarios.py
)

# Fork-only hunks in shared files, as "<path> <extended regex on an added line>".
FORK_ONLY_HUNKS=(
  ".github/actionlint.yaml ubuntu-26\\.04"
)

# Fork-only commits on hypr/base, by abbreviated SHA.
FORK_ONLY_COMMITS=(
  82318792 8ddf0e26 24cde3a6 d67d5294 7d3a2579 b84174ba
  e7f265c1 2a3bf4da e59537ef aaf0077e
  7510d5f3 dd14f2a8 fd79204f 37be4372
  c17a710d c033f345 fdf7eaaa e51f5f77 ee625edc 8b957f56
  1a45a312 7c22c0c4 8f5c8564 ca21c93e
)

FORK_ONLY_SUBJECT_PREFIX='test(fork)'

usage() {
  echo "usage: $0 <branch> [<upstream base, default upstream/main>]" >&2
  exit 2
}

[ $# -ge 1 ] && [ $# -le 2 ] || usage
branch=$1
base=${2:-upstream/main}

resolve() {
  git rev-parse --verify --quiet "$1^{commit}" || {
    echo "error: '$1' is not a commit in this repository" >&2
    exit 2
  }
}

branch_sha=$(resolve "$branch")
base_sha=$(resolve "$base")
merge_base=$(git merge-base "$base_sha" "$branch_sha") || {
  echo "error: '$branch' and '$base' share no history" >&2
  exit 2
}

findings=()

is_fork_only_path() {
  local entry
  for entry in "${FORK_ONLY_PATHS[@]}"; do
    case $entry in
      */) [[ $1 == "$entry"* ]] && return 0 ;;
      *) [[ $1 == "$entry" ]] && return 0 ;;
    esac
  done
  return 1
}

check_paths() {
  local where=$1 path
  shift
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    if is_fork_only_path "$path"; then
      findings+=("fork-only path $path in $where")
    fi
  done < <("$@")
}

check_hunks() {
  local where=$1 entry path pattern added line
  shift
  for entry in "${FORK_ONLY_HUNKS[@]}"; do
    path=${entry%% *}
    pattern=${entry#* }
    added=$("$@" -- "$path" | sed -n '/^+++ /d; /^+/p')
    line=$(grep -m1 -E -- "$pattern" <<<"$added" || true)
    if [ -n "$line" ]; then
      findings+=("fork-only line '${line#+}' in $path in $where")
    fi
  done
}

is_fork_only_commit() {
  local sha
  for sha in "${FORK_ONLY_COMMITS[@]}"; do
    [[ $1 == "$sha"* ]] && return 0
  done
  return 1
}

while IFS= read -r commit; do
  subject=$(git log -1 --format=%s "$commit")
  where="commit ${commit:0:12} ($subject)"
  if [[ $subject == "$FORK_ONLY_SUBJECT_PREFIX"* ]]; then
    findings+=("fork-only subject in $where")
  fi
  if is_fork_only_commit "$commit"; then
    findings+=("fork-only $where")
  fi
  while IFS= read -r picked; do
    if is_fork_only_commit "$picked"; then
      findings+=("cherry-pick of fork-only commit $picked in $where")
    fi
  done < <(git log -1 --format=%B "$commit" | sed -nE 's/.*cherry picked from commit ([0-9a-f]+).*/\1/p')
  check_paths "$where" git diff-tree -r -m --root --no-renames --no-commit-id --name-only "$commit"
  check_hunks "$where" git diff-tree -p -m --root --no-renames --no-commit-id "$commit"
done < <(git rev-list --reverse "$merge_base..$branch_sha")

final="final diff ${merge_base:0:12}...${branch_sha:0:12}"
check_paths "$final" git diff --no-renames --name-only "$merge_base" "$branch_sha"
check_hunks "$final" git diff --no-renames "$merge_base" "$branch_sha"

if [ ${#findings[@]} -eq 0 ]; then
  echo "ok: $branch holds no fork-only work since $base (merge-base ${merge_base:0:12})"
  exit 0
fi

listed=$(printf '  %s\n' "${findings[@]}" | awk '!seen[$0]++')
echo "$branch holds fork-only work since $base (merge-base ${merge_base:0:12}):"
echo "$listed"
echo "$(wc -l <<<"$listed") finding(s). Rebuild the branch by cherry-picking onto $base without them."
exit 1
