#!/bin/bash
# Host-safe checks for test/all. They never start a desktop: test/all must
# refuse before it touches anything when it is not inside an Omarchy VM.
#   bash test/test-all.sh
set -u
here=$(cd "$(dirname "$0")" && pwd)
failed=0
check() { if [[ $2 == ok ]]; then echo "ok   $1"; else echo "FAIL $1"; failed=1; fi; }

home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT

err=$(env -u OMARCHY_VM_CHANNEL HOME="$home" bash "$here/all" 2>&1 >/dev/null)
rc=$?
check "refuses outside a VM with exit 2" "$([[ $rc == 2 ]] && echo ok)"
check "names the VM in the refusal" "$([[ $err == *"Omarchy VM"* ]] && echo ok)"
check "writes nothing before it refuses" "$([[ -z $(ls -A "$home") ]] && echo ok)"

OMARCHY_VM_CHANNEL='' HOME="$home" bash "$here/all" >/dev/null 2>&1
check "an empty channel is not a VM" "$([[ $? == 2 ]] && echo ok)"

# The private session verdict, taken from test/all itself.
mutter_checks=()
eval "$(sed -n '/^mutter_checks=(/,/^)/p; /^private_session_ok() {/,/^}/p' "$here/all")"
log=$home/private-session.log
verdict() { printf '%s\n' "$@" >"$log"; private_session_ok "$log" && echo pass || echo fail; }
check "every check passing passes" "$([[ $(verdict 'ok   a' 'ok   b') == pass ]] && echo ok)"
check "only the mutter checks failing passes" \
  "$([[ $(verdict 'ok   a' "FAIL ${mutter_checks[0]}" "FAIL ${mutter_checks[2]}") == pass ]] && echo ok)"
check "another failed check fails" "$([[ $(verdict 'ok   a' "FAIL ${mutter_checks[0]}" 'FAIL b') == fail ]] && echo ok)"
check "a log with no passing check fails" "$([[ $(verdict 'private-session: crashed') == fail ]] && echo ok)"

exit "$failed"
