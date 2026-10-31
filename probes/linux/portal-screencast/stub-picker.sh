#!/usr/bin/env bash
# Stand-in for hyprland-share-picker, set as xdph `screencopy:custom_picker_binary` inside a throwaway VM only.
# Every invocation is one place where stock xdph would have opened a dialog, so the probe counts them.
# It never draws anything. It answers from $PROBE_DIR/request:
#   deny          print no selection; xdph treats that as a refusal
#   0x<address>   select the toplevel whose Hyprland address matches, with the restore-token flag `r`
set -euo pipefail

PROBE_DIR=/tmp/xdph-portal-probe
mkdir -p "$PROBE_DIR"
list=${XDPH_WINDOW_SHARING_LIST:-}
request=$(cat "$PROBE_DIR/request" 2>/dev/null || echo deny)

{
  printf 'invocation mono_ns=%s args=%s request=%s\n' "$(date +%s%N)" "$*" "$request"
  printf 'list=%s\n' "$list"
} >> "$PROBE_DIR/picker.log"

[ "$request" = deny ] && exit 0

want=$((request))
rest=$list
while [ -n "$rest" ]; do
  entry=${rest%%"[HA>]"*}
  rest=${rest#*"[HA>]"}
  handle_lo=${entry%%"[HC>]"*}
  address=${entry##*"[HE>]"}
  if [ "$address" = 0 ]; then
    printf 'unmapped handle_lo=%s\n' "$handle_lo" >> "$PROBE_DIR/picker.log"
  fi
  if [ "$address" = "$want" ]; then
    printf 'selected handle_lo=%s address=%s\n' "$handle_lo" "$address" >> "$PROBE_DIR/picker.log"
    printf '[SELECTION]r/window:%s\n' "$handle_lo"
    exit 0
  fi
done
printf 'no_match address=%s\n' "$want" >> "$PROBE_DIR/picker.log"
