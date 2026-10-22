#!/bin/sh
# Runs "$@" while holding one of two box slots, waiting up to $1 seconds for one.
# Slot 0 is the lock file that older checkouts of remote-linux.sh take, so mixed
# checkouts still never exceed two concurrent runs. Physical input stays serialized
# by the product's own interaction lease; the slots only bound builds and sessions.
# A command that itself exits 251 is indistinguishable from a busy slot and is retried.
wait_s=$1
shift
end=$(( $(date +%s) + wait_s ))
while :; do
  for slot in .box.lock .box.lock.1; do
    flock -o -n -E 251 "$HOME/ad-wt/$slot" "$@"
    rc=$?
    [ "$rc" -ne 251 ] && exit "$rc"
  done
  if [ "$(date +%s)" -ge "$end" ]; then
    echo "remote-linux: no box slot free after ${wait_s}s" >&2
    exit 251
  fi
  sleep 1
done
