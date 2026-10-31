#!/usr/bin/env bash
# Probe 2 (R3): zwlr_foreign_toplevel_handle_v1.activate on a normal non-active workspace and on a hidden special workspace.
# Also R2: two same-pid same-title windows forced to the same floating size.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
pa=$(launch ADTest); pb=$(launch ADOther); pc=$(launch ADSpecial)
python3 $P/two_windows.py ADTwin --same-size > $OUT/twin2.log 2>&1 & started+=("$!"); pt=$!
wait_clients 5 || { echo "apps did not map"; exit 1; }
sleep 0.5
A=$(addr_of_pid $pa); B=$(addr_of_pid $pb); C=$(addr_of_pid $pc)
twins=$(j clients | python3 -c "import json,sys; print(' '.join(c['address'] for c in json.load(sys.stdin) if c['pid']==$pt))")
echo "A=$A B=$B C=$C twins=$twins"
echo "== R2 twins floating, same size, same position"
for t in $twins; do
  hyprctl dispatch setfloating "address:$t" >/dev/null
  hyprctl dispatch resizewindowpixel "exact 400 300,address:$t" >/dev/null
  hyprctl dispatch movewindowpixel "exact 700 400,address:$t" >/dev/null
  hyprctl dispatch movetoworkspacesilent "3,address:$t"
done
sleep 0.3
clients | grep ADTwin
$AD list-windows | python3 -c 'import json,sys; [print("atspi", w["id"], w["pid"], w["bounds"], w["is_focused"], w["title"]) for w in json.load(sys.stdin)["data"] if w["title"]=="ADTwin"]'
echo "== setup: B to workspace 2, C to special:hid, focus A"
hyprctl dispatch movetoworkspacesilent "2,address:$B"
hyprctl dispatch movetoworkspacesilent "special:hid,address:$C"
hyprctl dispatch focuswindow "address:$A"
sleep 0.3
clients; active; wsinfo
probe() {
  echo "== activate $1 ($2)"
  $W activate "$2"
  sleep 0.3
  active; wsinfo
  $AD list-windows | python3 -c 'import json,sys; print("atspi_focused", [w["title"] for w in json.load(sys.stdin)["data"] if w["is_focused"]])'
  $W list | python3 -c 'import json,sys; [print("wlr", d["address"], d["title"], d["wlr_states"]) for d in map(json.loads, sys.stdin)]'
}
probe "B on non-active workspace 2" "$B"
probe "A back on workspace 1" "$A"
probe "C on hidden special:hid" "$C"
probe "A again (does special close?)" "$A"
