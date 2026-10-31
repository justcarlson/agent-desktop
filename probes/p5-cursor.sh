#!/usr/bin/env bash
# Probe 5 (R8): a long-lived client draws 2 overlay layer surfaces (namespace agent-desktop-cursor, empty input region).
set -uo pipefail
. "$(dirname "$0")/lib.sh"
use_headless
xkbcli compile-keymap --layout us > "$OUT/us.xkb"
pa=$(launch ADTest)
wait_clients 1 || exit 1
sleep 0.8
A=$(addr_of_pid $pa)
wid=$($AD list-windows | python3 -c "import json,sys; print(next(w['id'] for w in json.load(sys.stdin)['data'] if w['pid']==$pa))")
entry=$($AD snapshot --window-id "$wid" -i | python3 -c 'import json,sys
def walk(n):
    yield n
    for c in n.get("children",[]): yield from walk(c)
print(next(n["ref_id"] for n in walk(json.load(sys.stdin)["data"]["tree"]) if n.get("role")=="textfield"))')
layers() { j layers | python3 -c 'import json,sys
d=json.load(sys.stdin)
for mon,v in d.items():
    for lvl,ls in v["levels"].items():
        for l in ls: print("  layer", mon, "level="+lvl, l["namespace"], "pid=%s"%l["pid"], "at=%d,%d"%(l["x"],l["y"]), "size=%dx%d"%(l["w"],l["h"]))'; }
colors() { magick "$1" -format %c histogram:info:- | grep -iE '#FF0000|#0000FF' | sed 's/^ */  /' || echo "  no cursor colours"; }
clients | grep "^$A"; active
echo "== layers before"; layers
$W cursor 60 200,10 400,40 > "$OUT/p5-cursor.log" 2>&1 & cp=$!
for _ in $(seq 1 50); do grep -q 'cursor ready' "$OUT/p5-cursor.log" && break; sleep 0.1; done
cat "$OUT/p5-cursor.log"; echo "cursor client pid=$cp start=$(awk '{print $22}' /proc/$cp/stat)"
echo "== layers with cursor client"; layers
echo "== focus with cursor client"; active
echo "== keyboard still reaches the app: Tab with a full us keymap"
$W rawkeys "$OUT/us.xkb" 15; sleep 0.3
echo "entry states: $($AD get --property states "$entry" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["value"])')"
active
echo "== window capture (ext_image_copy_capture) with cursors over the window"
$W capture "$A" "$OUT/p5-window" "" 1000 | grep -E 'session|f1_'
colors "$OUT/p5-window-f1_first.png"
echo "== output capture (zwlr_screencopy, overlay_cursor=0) with cursors"
$W screencopy "$OUT/p5-screen-with.png"
colors "$OUT/p5-screen-with.png"
echo "== stop cursor client (exact pid $cp)"
kill -TERM "$cp"; wait "$cp" 2>/dev/null; sleep 0.3
echo "== layers after exit"; layers
$W screencopy "$OUT/p5-screen-after.png"
colors "$OUT/p5-screen-after.png"
