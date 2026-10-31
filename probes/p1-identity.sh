#!/usr/bin/env bash
# Probe 1 (R2): ext_foreign_toplevel identifiers mapped to Hyprland addresses, 2 same-title windows + 1 other.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
hyprctl version | head -1
p1=$(launch ADTest); p2=$(launch ADTest); p3=$(launch ADOther)
python3 $P/two_windows.py ADTwin --same-size > $OUT/twin.log 2>&1 & started+=("$!"); p4=$!
wait_clients 5 || { echo "apps did not map"; exit 1; }
sleep 0.5
echo "== hyprctl clients (address pid workspace size at title)"; clients
echo "== wlprobe list"; $W list
echo "== second wlprobe list (identifier stability across clients)"; $W list | cut -c1-60
echo "== agent-desktop list-windows"; $AD list-windows > $OUT/p1-list-windows.json 2>&1; python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(json.dumps(d["data"] if d.get("ok") else d)[:3000])' $OUT/p1-list-windows.json
echo "pids $p1 $p2 $p3 twin=$p4"
for id in $(python3 -c 'import json,sys; [print(w["id"]) for w in json.load(open(sys.argv[1]))["data"] if "Twin" in w.get("title","")]' $OUT/p1-list-windows.json 2>/dev/null); do echo "snapshot $id"; $AD snapshot --window-id "$id" -i 2>&1 | head -c 600; echo; done
for p in $p1 $p2; do echo "pid $p start=$(awk '{print $22}' /proc/$p/stat)"; done
