#!/usr/bin/env bash
# Runs the ignored Wayland input live tests on the box, from the worktree root, as
#   scripts/remote-linux.sh bash -s < crates/linux/src/input/wayland/live_wayland.sh
# The tests start and health-check their own headless gnome-shell; this wrapper only builds a
# private copy of the test binary (other worktrees overwrite the shared one) and runs it inside
# a private session.
set -uo pipefail
filter=${AD_FILTER:-input::wayland::live_tests}
bin=$HOME/ad-wt/lane-c-bin/linux-tests
mkdir -p "$(dirname "$bin")"
ready=""
for _ in $(seq 1 20); do
  touch crates/linux/src/lib.rs
  exe=$(cargo test -p agent-desktop-linux --lib --no-run --message-format=json 2>/dev/null \
    | grep -o '"executable":"[^"]*agent_desktop_linux[^"]*"' | head -1 | cut -d'"' -f4)
  [ -n "$exe" ] && cp "$exe" "$bin"
  if "$bin" --list 2>/dev/null | grep -q "wayland::live_tests::"; then ready=1; break; fi
done
[ -n "$ready" ] || { echo "live_wayland: no test binary with the live tests" >&2; exit 1; }
exec bash scripts/private-session.sh -- "$bin" --ignored --test-threads=1 --nocapture "$filter"
