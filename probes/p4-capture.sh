#!/usr/bin/env bash
# Probe 4 (R7): ext_image_copy_capture of one toplevel, visible and on a hidden special workspace.
# Per run: f1 first frame, f2 a second frame with no change, then a headless AT-SPI click on Increment, then f3.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
[ "${HEADLESS:-1}" = 1 ] && use_headless
pa=$(launch ADTest); pb=$(launch ADOther)
wait_clients 2 || exit 1
sleep 0.8
A=$(addr_of_pid $pa)
wid=$($AD list-windows | python3 -c "import json,sys; print(next(w['id'] for w in json.load(sys.stdin)['data'] if w['pid']==$pa))")
btn() { $AD snapshot --window-id "$wid" -i | python3 -c 'import json,sys
def walk(n):
    yield n
    for c in n.get("children",[]): yield from walk(c)
print(next(n["ref_id"] for n in walk(json.load(sys.stdin)["data"]["tree"]) if n.get("role")=="button"))'; }
count() { $AD snapshot --window-id "$wid" | grep -oE 'count [0-9]+' | head -1; }
echo "A=$A wid=$wid"
for mode in visible hidden; do
  if [ $mode = hidden ]; then hyprctl dispatch movetoworkspacesilent "special:hid,address:$A"; sleep 0.3; fi
  clients | grep -E "^$A"; active; wsinfo
  echo "== capture $mode ($(count))"
  $W capture "$A" "$OUT/p4-$mode" "$AD click $(btn) > /dev/null; sleep 0.2" 2000
  echo "after: $(count)"
done
echo "== hidden again, static window, 3 frames in a row with no change"
$W capture "$A" "$OUT/p4-hidden-static" "" 3000
