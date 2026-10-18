#!/usr/bin/env bash
# Read the live desktop env from a running session process.
# Guessing DISPLAY=:0 breaks under AnyDesk/CRD, which run their own X display.
for proc in gnome-shell xfce4-session plasmashell mate-session cinnamon-session lxqt-session; do
  pid=$(pgrep -u "$(id -un)" -x "$proc" | head -1)
  [ -n "$pid" ] && break
done
if [ -z "${pid:-}" ]; then
  echo "session-env: no graphical session found for $(id -un)" >&2
  return 1
fi
eval "$(tr '\0' '\n' < "/proc/$pid/environ" \
  | grep -E '^(DISPLAY|WAYLAND_DISPLAY|XAUTHORITY|DBUS_SESSION_BUS_ADDRESS|XDG_RUNTIME_DIR)=' \
  | sed 's/^\([A-Z_]*\)=\(.*\)$/export \1="\2"/')"
