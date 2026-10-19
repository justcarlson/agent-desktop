#!/usr/bin/env bash
# Runs the ignored Wayland input live tests on the box, from the worktree root, as
#   scripts/remote-linux.sh bash -s < crates/linux/src/input/wayland/live_wayland.sh
# A headless gnome-shell hangs on about half of its starts, so the shell is started in a loop
# and kept only once repeated RemoteDesktop reads and a CreateSession answer in time.
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
export AD_FILTER_RUN=$filter AD_BIN=$bin
bash scripts/private-session.sh -- bash -c '
rd="--session --dest org.gnome.Mutter.RemoteDesktop --object-path /org/gnome/Mutter/RemoteDesktop"
healthy() {
  for _ in 1 2 3 4 5; do
    timeout 3 gdbus call $rd --method org.freedesktop.DBus.Properties.Get org.gnome.Mutter.RemoteDesktop Version >/dev/null 2>&1 || return 1
    sleep 0.4
  done
  reply=$(timeout 3 gdbus call $rd --method org.gnome.Mutter.RemoteDesktop.CreateSession 2>&1) || return 1
  path=$(echo "$reply" | grep -o "/org/gnome/Mutter/RemoteDesktop/Session/[a-z0-9]*")
  timeout 3 gdbus call --session --dest org.gnome.Mutter.RemoteDesktop --object-path "$path" --method org.gnome.Mutter.RemoteDesktop.Session.Stop >/dev/null 2>&1
  return 0
}
shell=""
for try in 1 2 3 4 5 6; do
  rm -f "$XDG_RUNTIME_DIR"/wayland-*
  ( unset DISPLAY; exec gnome-shell --headless --wayland --virtual-monitor 1280x800 > "$XDG_RUNTIME_DIR/shell.log" 2>&1 ) &
  shell=$!
  for _ in $(seq 1 100); do
    sock=$(ls "$XDG_RUNTIME_DIR" | grep -E "^wayland-[0-9]+$" | head -1)
    [ -n "$sock" ] && break
    sleep 0.2
  done
  sleep 3
  if healthy; then break; fi
  kill "$shell" 2>/dev/null; sleep 2; kill -KILL "$shell" 2>/dev/null
  shell=""
done
[ -n "$shell" ] || { echo "live_wayland: no healthy shell" >&2; exit 1; }
export WAYLAND_DISPLAY=$sock
status=0
"$AD_BIN" --ignored --test-threads=1 --nocapture "$AD_FILTER_RUN" || status=$?
kill "$shell" 2>/dev/null
exit $status
'
