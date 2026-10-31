#!/usr/bin/env bash
# Runs a command against a private Hyprland session nested in the owner's Hyprland.
# Usage: AGENT_DESKTOP_HYPRLAND_PARENT=<parent wayland socket path> \
#        AGENT_DESKTOP_HYPRLAND_PARENT_SIGNATURE=<parent instance signature> \
#        scripts/private-session.sh -- scripts/private-hyprland-shell.sh [--] command [args...]
# Hyprland has no headless-only backend, and the packaged headless parents (weston 15, sway 1.12)
# offer older wl_compositor and xdg_wm_base versions than Aquamarine binds, so the parent is the
# owner's compositor. The parent starts the nested compositor on a silent special workspace, so
# its window never takes the owner's focus or workspace. The nested compositor carries the private
# session's runtime dir, bus and config dirs, and the private session's sweep reaches it.
# Arch's AT-SPI bus launcher activates the registry through systemd, which the private bus
# cannot reach, so the registry is started here when activation leaves it unowned.
# A private runtime dir is deep enough that the instance socket path passes the 108-byte
# sun_path limit; Hyprland and hyprctl truncate it the same way, so readiness asks hyprctl
# instead of testing for .socket.sock.
# WAYLAND_DISPLAY and HYPRLAND_INSTANCE_SIGNATURE name the nested session for the command.
set -euo pipefail

[ -n "${AGENT_DESKTOP_PRIVATE_SESSION:-}" ] || {
  echo "private-hyprland-shell: run it through scripts/private-session.sh" >&2
  exit 2
}
[ "${1:-}" = "--" ] && shift
[ $# -gt 0 ] || { echo "usage: private-hyprland-shell.sh [--] command [args...]" >&2; exit 2; }
parent=${AGENT_DESKTOP_HYPRLAND_PARENT:?set AGENT_DESKTOP_HYPRLAND_PARENT to the parent socket path}
parent_sig=${AGENT_DESKTOP_HYPRLAND_PARENT_SIGNATURE:?set AGENT_DESKTOP_HYPRLAND_PARENT_SIGNATURE}
case "$parent" in /*) ;; *) echo "private-hyprland-shell: the parent socket path must be absolute" >&2; exit 2 ;; esac
[ -S "$parent" ] || { echo "private-hyprland-shell: no parent socket at $parent" >&2; exit 2; }

rt=$XDG_RUNTIME_DIR
log=$AGENT_DESKTOP_PRIVATE_SESSION
parent_rt=$(dirname "$parent")
workspace=special:agent-desktop-${rt##*.}
attempts=3
shell_pid=

parent_ctl() { env XDG_RUNTIME_DIR="$parent_rt" HYPRLAND_INSTANCE_SIGNATURE="$parent_sig" hyprctl "$@"; }

process_start() {
  local stat fields
  stat=$(cat "/proc/$1/stat" 2>/dev/null) || return 1
  stat=${stat##*) }
  read -r -a fields <<< "$stat"
  printf '%s\n' "${fields[19]}"
}

stop_shell() {
  if [ -n "$shell_pid" ] && [ "$(process_start "$shell_pid" || true)" = "$shell_start" ] &&
     grep -qzxF "XDG_RUNTIME_DIR=$rt" "/proc/$shell_pid/environ" 2>/dev/null; then
    kill -TERM "$shell_pid" 2>/dev/null || true
    for _ in $(seq 1 50); do kill -0 "$shell_pid" 2>/dev/null || break; sleep 0.1; done
    kill -KILL "$shell_pid" 2>/dev/null || true
  fi
  shell_pid=
}
trap stop_shell EXIT

parent_exec() {
  local reply
  reply=$(parent_ctl dispatch "hl.dsp.exec_cmd(\"$1\", { workspace = \"$workspace silent\" })" 2>&1) || true
  [ "$reply" = ok ] && return 0
  reply=$(parent_ctl dispatch exec "[workspace $workspace silent] $1" 2>&1) || true
  [ "$reply" = ok ]
}

registry_ready() {
  local a11y
  a11y=$(gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus \
    --method org.a11y.Bus.GetAddress 2>/dev/null | sed -n "s/^('\(.*\)',)$/\1/p") || return 1
  [ -n "$a11y" ] || return 1
  gdbus call --address "$a11y" --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.NameHasOwner org.a11y.atspi.Registry 2>/dev/null | grep -q true
}

start_registry() {
  registry_ready && return 0
  gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus \
    --method org.a11y.Bus.GetAddress > /dev/null 2>&1 || return 1
  setsid /usr/lib/at-spi2-registryd > "$log/registryd.log" 2>&1 < /dev/null &
  for _ in $(seq 1 50); do registry_ready && return 0; sleep 0.1; done
  return 1
}

start_shell() {
  rm -rf -- "$rt/hypr"
  cat > "$rt/hyprland.conf" <<EOF
monitor = , 1280x800@60, 0x0, 1
misc {
  disable_hyprland_logo = true
  disable_splash_rendering = true
}
animations {
  enabled = false
}
EOF
  cat > "$rt/hyprland-start" <<EOF
#!/bin/sh
echo \$\$ > "$rt/hyprland.pid"
exec env -i HOME="$HOME" USER="${USER:-$(id -un)}" PATH="$PATH" LANG="${LANG:-C.UTF-8}" \\
  XDG_RUNTIME_DIR="$rt" WAYLAND_DISPLAY="$parent" DBUS_SESSION_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" \\
  XDG_CONFIG_HOME="$XDG_CONFIG_HOME" XDG_DATA_HOME="$XDG_DATA_HOME" XDG_CACHE_HOME="$XDG_CACHE_HOME" \\
  TMPDIR="$TMPDIR" GSETTINGS_BACKEND="$GSETTINGS_BACKEND" AGENT_DESKTOP_PRIVATE_SESSION="$log" \\
  Hyprland -c "$rt/hyprland.conf" > "$log/hyprland.log" 2>&1 < /dev/null
EOF
  chmod 700 "$rt/hyprland-start"
  rm -f -- "$rt/hyprland.pid"
  parent_exec "$rt/hyprland-start" || return 1
  for _ in $(seq 1 100); do [ -s "$rt/hyprland.pid" ] && break; sleep 0.1; done
  [ -s "$rt/hyprland.pid" ] || return 1
  shell_pid=$(cat "$rt/hyprland.pid")
  shell_start=$(process_start "$shell_pid") || return 1
  signature=
  socket=
  for _ in $(seq 1 100); do
    for dir in "$rt"/hypr/*/; do
      [ -f "${dir}hyprland.lock" ] || continue
      if HYPRLAND_INSTANCE_SIGNATURE=$(basename "$dir") hyprctl -j monitors 2>/dev/null | grep -q '"name"'; then
        signature=$(basename "$dir")
      fi
    done
    if [ -n "$signature" ]; then
      socket=$(find "$rt" -maxdepth 1 -name 'wayland-*' ! -name '*.lock' -printf '%f\n' | sort | head -n 1)
      [ -n "$socket" ] && break
    fi
    sleep 0.1
  done
  [ -n "$socket" ]
}

for _ in $(seq 1 "$attempts"); do
  if start_shell; then
    export WAYLAND_DISPLAY=$socket HYPRLAND_INSTANCE_SIGNATURE=$signature
    export XDG_SESSION_TYPE=wayland GDK_BACKEND=wayland QT_QPA_PLATFORM=wayland
    dbus-update-activation-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE \
      XDG_SESSION_TYPE GDK_BACKEND QT_QPA_PLATFORM
    start_registry || { echo "private-hyprland-shell: the AT-SPI registry did not start" >&2; exit 1; }
    "$@"
    exit $?
  fi
  stop_shell
done
echo "private-hyprland-shell: no nested Hyprland after $attempts starts; its log follows" >&2
tail -n 20 "$log/hyprland.log" >&2 2>/dev/null || true
exit 1
