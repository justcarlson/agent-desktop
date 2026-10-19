#!/usr/bin/env bash
# Measures the snap and Flatpak accessibility legs on a GitHub Ubuntu runner and prints the
# results; nothing here reads or writes a file in the repository.
# Usage: scripts/sandbox-closure.sh PATH-TO-agent-desktop
set -uo pipefail

ad=${1:?path to the agent-desktop binary}
snap_plugs=(unity7 desktop-legacy)

read_app() {
  local label=$1 app=$2 apps listing
  apps=$("$ad" list-apps 2>&1 | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
    print([a["name"] for a in data["data"]["apps"]])
except Exception as error:
    print("unreadable:", error)
')
  listing=$("$ad" snapshot --app "$app" -i 2>&1 | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
except Exception as error:
    print("unreadable:", error)
    sys.exit()
if data.get("ok"):
    print("ok ref_count", data["data"].get("ref_count"))
else:
    error = data.get("error", {})
    print("error", error.get("code"), error.get("message"), error.get("details"))
')
  echo "RESULT $label apps=$apps snapshot=$listing"
}

wait_for_app() {
  local app=$1
  for _ in $(seq 1 40); do
    "$ad" list-apps 2>/dev/null | grep -qi "$app" && return 0
    sleep 1
  done
  return 1
}

stop_app() {
  pkill -f "$1" 2>/dev/null || true
  sleep 3
}

snap_leg() {
  local round connected plug
  echo "SNAP connections before:"
  snap connections firefox
  connected=()
  for plug in "${snap_plugs[@]}"; do
    if snap connections firefox | grep -q "firefox:$plug"; then
      connected+=("$plug")
    fi
  done
  echo "SNAP connected plugs: ${connected[*]}"
  for round in 1 2; do
    for mode in default restricted; do
      if [ "$mode" = restricted ]; then
        for plug in "${connected[@]}"; do sudo snap disconnect "firefox:$plug"; done
      fi
      snap run firefox --no-remote --profile "$HOME/ff-$mode-$round" about:blank >/dev/null 2>&1 &
      wait_for_app firefox || echo "SNAP round $round $mode: firefox never reached the registry"
      sleep 5
      read_app "snap round=$round mode=$mode" firefox
      stop_app firefox
      if [ "$mode" = restricted ]; then
        for plug in "${connected[@]}"; do sudo snap connect "firefox:$plug"; done
      fi
    done
  done
  echo "SNAP connections after:"
  snap connections firefox
}

flatpak_leg() {
  local round case_name flags
  flatpak remote-add --if-not-exists --system flathub https://flathub.org/repo/flathub.flatpakrepo
  flatpak install -y --system flathub org.gnome.TextEditor
  flatpak info --show-permissions org.gnome.TextEditor
  for round in 1 2; do
    for case_name in default denied granted; do
      case $case_name in
        default) flags=() ;;
        denied) flags=(--no-talk-name=org.a11y.Bus) ;;
        granted) flags=(--talk-name=org.a11y.Bus) ;;
      esac
      flatpak run "${flags[@]}" org.gnome.TextEditor >/dev/null 2>&1 &
      wait_for_app text-editor || echo "FLATPAK round $round $case_name: the editor never reached the registry"
      sleep 5
      read_app "flatpak round=$round case=$case_name flags=${flags[*]:-none}" gnome-text-editor
      flatpak kill org.gnome.TextEditor 2>/dev/null || true
      sleep 3
    done
  done
  echo "FLATPAK override file after:"
  flatpak override --system --show org.gnome.TextEditor || true
}

gsettings set org.gnome.desktop.interface toolkit-accessibility true
snap_leg
flatpak_leg
