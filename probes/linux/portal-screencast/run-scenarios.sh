#!/usr/bin/env bash
# Drives the portal ScreenCast probe through the scenarios in FINDINGS.md. Throwaway VM only: it rewrites the
# VM user's xdph.conf, restarts xdg-desktop-portal-hyprland and opens windows. It refuses on a host session.
# Usage, inside the VM session: PORTAL_PROBE_VM=1 bash run-scenarios.sh OUT_DIR
set -euo pipefail

[ "${PORTAL_PROBE_VM:-}" = 1 ] || { echo "refused: set PORTAL_PROBE_VM=1, and only inside a throwaway VM" >&2; exit 2; }
systemd-detect-virt --vm > /dev/null || { echo "refused: not a VM" >&2; exit 2; }
[ $# -eq 1 ] || { echo "usage: $0 OUT_DIR" >&2; exit 2; }

HERE=$(cd "$(dirname "$0")" && pwd)
OUT=$1
PROBE_DIR=/tmp/xdph-portal-probe
TERM_CMD=${PROBE_TERM_CMD:-alacritty --class adprobe.term -e}
CONF="$HOME/.config/hypr/xdph.conf"
mkdir -p "$OUT" "$PROBE_DIR"
started=()

stop_started() {
  for pid in "${started[@]}"; do kill -TERM "$pid" 2> /dev/null || true; done
  if [ -f "$CONF.adprobe-backup" ]; then mv "$CONF.adprobe-backup" "$CONF"; else rm -f "$CONF"; fi
  systemctl --user restart xdg-desktop-portal-hyprland
}
trap stop_started EXIT

probe() { python3 -I "$HERE/portal_screencast_probe.py" "$@" | tee -a "$OUT/records.jsonl"; }
field() { python3 -I -c 'import json,sys; print(json.loads(sys.stdin.readlines()[-1]).get(sys.argv[1]) or "")' "$1"; }
address_of_pid() {
  hyprctl -j clients | python3 -I -c 'import json,sys; print(next(c["address"] for c in json.load(sys.stdin) if c["pid"]==int(sys.argv[1])))' "$1"
}
wait_for_pid_window() {
  for _ in $(seq 1 100); do address_of_pid "$1" 2> /dev/null && return 0; sleep 0.1; done
  return 1
}
ask_picker() { printf '%s\n' "$1" > "$PROBE_DIR/request"; }

[ -f "$CONF" ] && cp "$CONF" "$CONF.adprobe-backup"
mkdir -p "$(dirname "$CONF")"
printf 'screencopy {\n    allow_token_by_default = false\n    custom_picker_binary = %s\n}\n' "$HERE/stub-picker.sh" > "$CONF"
systemctl --user restart xdg-desktop-portal-hyprland
: > "$PROBE_DIR/picker.log"

# Two windows with one class, distinct sizes, and content that changes every 100 ms (the ticking counter).
# shellcheck disable=SC2086
$TERM_CMD sh -c 'while :; do date +%s%N; sleep 0.1; done' & started+=("$!")
# shellcheck disable=SC2086
$TERM_CMD sh -c 'while :; do date +%s%N; sleep 0.1; done' & started+=("$!")
A=$(wait_for_pid_window "${started[0]}")
B=$(wait_for_pid_window "${started[1]}")
for spec in "$A 600 400" "$B 900 700"; do
  read -r addr w h <<< "$spec"
  hyprctl dispatch setfloating "address:$addr" > /dev/null
  hyprctl dispatch resizewindowpixel "exact $w $h,address:$addr" > /dev/null
done
hyprctl -j clients > "$OUT/clients-initial.json"
echo "A=$A (600x400) B=$B (900x700)" | tee "$OUT/windows.txt"

# Q1 first run: no token. Each picker invocation is a dialog under stock xdph; the stub refuses.
ask_picker deny
probe run --label q1-first-run-deny

# Q2 grant: the picker selects B by Hyprland address; the restore token comes back.
ask_picker "$B"
token=$(probe run --label q2-grant-b | field restore_token_out)

# Q1 restore: any picker call is counted and refused, so a restore that needs one fails closed here.
ask_picker deny
token=$(probe run --label q1-restore-same-xdph --restore-token "$token" | field restore_token_out)

# Q2 identity across an xdph restart: the stored wl_resource pointer no longer matches; xdph falls back to
# the first toplevel with the stored class. The stream size says whether A or B came back.
systemctl --user restart xdg-desktop-portal-hyprland
token=$(probe run --label q2-restore-after-xdph-restart --restore-token "$token" | field restore_token_out)

# Q1 revoked: delete the token from the permission store, then restore. Stock xdph opens the picker here.
probe revoke --token "$token"
probe run --label q1-restore-after-revoke --restore-token "$token"

# Q3 + Q4: B on a hidden special workspace; first frame latency, pts against the request, counter changes.
hyprctl dispatch movetoworkspacesilent "special:adprobe,address:$B" > /dev/null
hyprctl -j clients > "$OUT/clients-hidden.json"
ask_picker "$B"
probe run --label q3-hidden-ticking --frames 6 --deadline-s 5
hyprctl keyword animations:enabled false > /dev/null
probe run --label q4-hidden-idle-output --frames 6 --deadline-s 5
tail -n 50 "$PROBE_DIR/picker.log" > "$OUT/picker.log"
