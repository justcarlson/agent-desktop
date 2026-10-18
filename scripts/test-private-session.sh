#!/usr/bin/env bash
# Behaviour checks for scripts/private-session.sh. Run on the Linux box:
#   scripts/remote-linux.sh bash scripts/test-private-session.sh
set -u
helper=$(cd "$(dirname "$0")" && pwd)/private-session.sh
failed=0
marker=$(mktemp -u)

check() {
  if [ "$2" = ok ]; then echo "ok   $1"; else echo "FAIL $1"; failed=1; fi
}

AGENT_DESKTOP_WM_ROOT=/nonexistent "$helper" --wm -- touch "$marker" 2> /dev/null
status=$?
verdict=ok
{ [ "$status" -ne 0 ] && [ ! -e "$marker" ]; } || verdict=bad
check "a missing openbox fails and never runs the payload" "$verdict"
case "$marker" in
  "${TMPDIR:-/tmp}"/tmp.?*) rm -f -- "$marker" ;;
esac

# shellcheck disable=SC2016
seen=$(AT_SPI_BUS_ADDRESS=unix:path=/nonexistent/sentinel NO_AT_BRIDGE=1 GTK_A11Y=none \
  "$helper" -- sh -c 'echo "${AT_SPI_BUS_ADDRESS-unset}/${NO_AT_BRIDGE-unset}/${GTK_A11Y-unset}"')
verdict=ok
[ "$seen" = "unset/unset/unset" ] || verdict=bad
check "inherited accessibility overrides never reach the payload" "$verdict"

exit "$failed"
