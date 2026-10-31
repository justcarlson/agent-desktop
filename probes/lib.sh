# Sourced by the probe payloads inside the nested private session.
P=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
W=$P/wlprobe/target/debug/wlprobe
AD=${AD:-agent-desktop}
APP=${APP:?set APP to the GTK3 test app script (button, entry, checkbox)}
OUT=$P/out
mkdir -p "$OUT"
started=()

j() { hyprctl -j "$@"; }
clients() { j clients | python3 -c 'import json,sys; [print(c["address"],c["pid"],c["workspace"]["name"],"%dx%d"%tuple(c["size"]),"%d,%d"%tuple(c["at"]),repr(c["title"])) for c in json.load(sys.stdin)]'; }
addr_of_pid() { j clients | python3 -c "import json,sys; print(next(c['address'] for c in json.load(sys.stdin) if c['pid']==$1))"; }
active() { j activewindow | python3 -c 'import json,sys; a=json.load(sys.stdin); print("activewindow", a.get("address"), a.get("workspace",{}).get("name"), repr(a.get("title")))'; }
wsinfo() { echo "activeworkspace $(j activeworkspace | python3 -c 'import json,sys; print(json.load(sys.stdin)["name"])') monitor_special=$(j monitors | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["specialWorkspace"]["name"] or "-")')"; }
wait_clients() { for _ in $(seq 1 100); do [ "$(j clients | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')" -ge "$1" ] && return 0; sleep 0.1; done; return 1; }
launch() { python3 -c "import sys; exec(open('$APP').read().replace('\"ADTest\"', repr(sys.argv[1])))" "$1" > "$OUT/app-$1-$RANDOM.log" 2>&1 & started+=("$!"); echo "$!"; }
stop_started() { for p in "${started[@]}"; do kill -TERM "$p" 2>/dev/null || true; done; }
trap stop_started EXIT
# The nested output lives on a hidden host special workspace and gets no frame callbacks, so it never renders.
# A nested headless output renders on its own timer; the Wayland output is then disabled.
use_headless() {
  hyprctl output create headless PROBE >/dev/null
  hyprctl keyword monitor "PROBE,1280x800@60,0x0,1" >/dev/null
  hyprctl keyword monitor "WAYLAND-1,disable" >/dev/null
  sleep 0.5
  j monitors | python3 -c 'import json,sys; print("monitors", [(m["name"], m["width"], m["height"], m["activeWorkspace"]["name"]) for m in json.load(sys.stdin)])'
}
