#!/usr/bin/env bash
#
# Decides which CI jobs a change needs, and whether the ci-ok gate passes.
#
#   ci-scope.sh paths BASE HEAD
#     Prints every path changed between two commits, with renames split into
#     their source and destination.
#   ci-scope.sh scope ROOT < changed-paths
#     Prints code=false when the change can skip the Rust jobs, else code=true.
#   ci-scope.sh verdict CODE ALWAYS... < needs-json
#     Exits 0 when every job in the needs JSON succeeded, or was skipped while
#     CODE is false and the job is not one of the ALWAYS jobs.
#
# A change skips the Rust jobs only when every path is documentation that no
# build, test or script reads: a file under apps/docs/, or a file under docs/
# whose path appears nowhere in crates/, src/, tests/, scripts/ or .github/.
# An empty list runs everything. scripts/ci-scope-selftest.sh proves both rules.

set -euo pipefail

is_read_by_checks() {
  local root=$1 path=$2 dir
  for dir in crates src tests scripts .github; do
    if [ -d "$root/$dir" ] && grep -rqF -- "$path" "$root/$dir"; then
      return 0
    fi
  done
  return 1
}

scope() {
  local root=$1 paths path seen=0
  paths=$(cat)
  while IFS= read -r path || [ -n "$path" ]; do
    [ -n "$path" ] || continue
    seen=1
    case "$path" in
      apps/docs/*) ;;
      docs/*)
        if is_read_by_checks "$root" "$path"; then
          echo "code=true"
          return 0
        fi
        ;;
      *)
        echo "code=true"
        return 0
        ;;
    esac
  done <<<"$paths"
  if [ "$seen" -eq 1 ]; then
    echo "code=false"
  else
    echo "code=true"
  fi
}

verdict() {
  local code=$1 needs always
  shift
  needs=$(cat)
  always=$(printf '%s\n' "$@" | jq -R . | jq -s .)
  printf '%s' "$needs" | jq -r 'to_entries[] | "\(.key): \(.value.result)"'
  printf '%s' "$needs" | jq -e --arg code "$code" --argjson always "$always" '
    (to_entries | length) > 0 and (to_entries | all(
      .value.result == "success"
      or (.value.result == "skipped" and $code == "false"
          and (.key as $job | $always | index($job) | not))
    ))' >/dev/null
}

case "${1:-}" in
  paths)
    [ $# -eq 3 ] || { echo "usage: $0 paths BASE HEAD" >&2; exit 2; }
    git diff --no-renames --name-only "$2" "$3"
    ;;
  scope)
    [ $# -eq 2 ] || { echo "usage: $0 scope ROOT < paths" >&2; exit 2; }
    scope "$2"
    ;;
  verdict)
    [ $# -ge 2 ] || { echo "usage: $0 verdict CODE ALWAYS... < needs-json" >&2; exit 2; }
    verdict "${@:2}"
    ;;
  *)
    echo "usage: $0 paths BASE HEAD | scope ROOT | verdict CODE ALWAYS..." >&2
    exit 2
    ;;
esac
