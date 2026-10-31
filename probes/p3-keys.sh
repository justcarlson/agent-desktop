#!/usr/bin/env bash
# Probe 3 (R4): zwp_virtual_keyboard_v1 delivery into the focused Entry; nested layout set to de to detect layout changes.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
kbd() { j devices | python3 -c 'import json,sys; [print("  kbd", repr(k["name"]), "layout="+k["layout"], "active_keymap="+repr(k["active_keymap"]), "main="+str(k.get("main"))) for k in json.load(sys.stdin)["keyboards"]]'; }
xkbcli compile-keymap --layout us > "$OUT/us.xkb"
hyprctl keyword input:kb_layout de >/dev/null
WAYLAND_DEBUG=1 python3 $P/keylog_app.py > "$OUT/p3-app.log" 2> "$OUT/p3-app-wl.log" & started+=("$!")
wait_clients 1 || exit 1
sleep 0.8
wid=$($AD list-windows | python3 -c 'import json,sys; print(next(w["id"] for w in json.load(sys.stdin)["data"] if w["title"]=="ADTest"))')
$AD snapshot --window-id "$wid" -i > "$OUT/p3-snap.json"
entry=$(python3 -c 'import json,sys
def walk(n):
    yield n
    for c in n.get("children",[]): yield from walk(c)
print(next(n["ref_id"] for n in walk(json.load(open(sys.argv[1]))["data"]["tree"]) if n.get("role")=="textfield"))' "$OUT/p3-snap.json")
states() { $AD get --property states "$entry" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["value"])'; }
echo "entry $entry states=$(states)"
echo "== A: Tab with a minimal generated keymap (wtype style)"; $W type "$(printf '\t')" 5; sleep 0.3; echo "entry states=$(states)"
echo "== B: Tab with a full us keymap, evdev KEY_TAB"; $W rawkeys "$OUT/us.xkb" 15; sleep 0.3; echo "entry states=$(states)"
echo "== devices before"; kbd
echo "== C: type 'Hello é' with a minimal generated keymap, 300 ms per key"
$W type "Hello é" 300 & tp=$!
sleep 1.0; echo "== devices during"; kbd
wait $tp; sleep 0.3
echo "== devices after"; kbd
echo "AT-SPI value: $($AD get --property value "$entry")"
echo "== D: same text, 5 ms per key, after clearing"
$AD set-value "$entry" "" > /dev/null
$W type "Hello é" 5; sleep 0.3
echo "AT-SPI value: $($AD get --property value "$entry")"
echo "== devices end"; kbd
echo "== app key log"; grep -E 'press|entry' "$OUT/p3-app.log" | tr '\n' ';'; echo
echo "== client keymap events (size): $(grep -oE 'wl_keyboard#[0-9]+\.keymap\(1, fd [0-9]+, [0-9]+\)' "$OUT/p3-app-wl.log" | grep -oE '[0-9]+\)$' | tr -d ')' | tr '\n' ' ')"
