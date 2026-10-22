#!/usr/bin/env bash
# Read the live desktop env from a running session process.
# Guessing DISPLAY=:0 breaks under AnyDesk/CRD, which run their own X display.
for proc in gnome-shell xfce4-session plasmashell mate-session cinnamon-session lxqt-session; do
  pid=
  for candidate in $(pgrep -u "$(id -un)" -x "$proc"); do
    candidate_env=$(tr '\0' '\n' 2>/dev/null < "/proc/$candidate/environ") || continue
    grep -q '^AGENT_DESKTOP_PRIVATE_SESSION=' <<< "$candidate_env" && continue
    grep -q '^XDG_RUNTIME_DIR=.' <<< "$candidate_env" || continue
    pid=$candidate
    break
  done
  [ -n "$pid" ] && break
done
if [ -z "${pid:-}" ]; then
  echo "session-env: no graphical session found for $(id -un)" >&2
  return 1
fi
unset DISPLAY WAYLAND_DISPLAY XAUTHORITY DBUS_SESSION_BUS_ADDRESS XDG_RUNTIME_DIR
while IFS= read -r entry; do
  case "$entry" in
    DISPLAY=*|WAYLAND_DISPLAY=*|XAUTHORITY=*|DBUS_SESSION_BUS_ADDRESS=*|XDG_RUNTIME_DIR=*)
      export "$entry" ;;
  esac
done <<< "$candidate_env"
