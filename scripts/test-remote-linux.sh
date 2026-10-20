#!/usr/bin/env bash
# Behaviour checks for scripts/remote-linux.sh against a stubbed ssh and rsync, so no box is needed:
#   bash scripts/test-remote-linux.sh
set -u
here=$(cd "$(dirname "$0")" && pwd)
failed=0
stubs=$(mktemp -d)
log="$stubs/ssh.log"
trap 'rm -rf "$stubs"' EXIT
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> "%s"\n' "$log" > "$stubs/ssh"
printf '#!/bin/sh\nexit 0\n' > "$stubs/rsync"
chmod +x "$stubs/ssh" "$stubs/rsync"

check() {
  if [ "$2" = ok ]; then echo "ok   $1"; else echo "FAIL $1"; failed=1; fi
}

run() {
  : > "$log"
  PATH="$stubs:$PATH" AGENT_DESKTOP_REMOTE_LOCK=0 bash "$here/remote-linux.sh" "$@" > /dev/null 2>&1
  tail -n 1 "$log"
}

sent=$(run cargo test -p agent-desktop-linux)
verdict=ok
case "$sent" in *"touch {}"*"exec"*) ;; *) verdict=bad ;; esac
check "a cargo command touches the synced sources before it runs" "$verdict"

sent=$(run -c 'cd x && cargo build')
verdict=ok
case "$sent" in *"touch {}"*) ;; *) verdict=bad ;; esac
check "a cargo command inside a shell string touches the sources too" "$verdict"

# shellcheck disable=SC2116
sent=$(run echo hello)
verdict=ok
case "$sent" in *touch*) verdict=bad ;; esac
check "a command without cargo is not delayed by a touch" "$verdict"

sent=$(run cargo test -p agent-desktop-linux)
verdict=ok
case "$sent" in *CARGO_INCREMENTAL=0*) ;; *) verdict=bad ;; esac
check "every remote command turns incremental builds off" "$verdict"

refused() {
  : > "$log"
  if PATH="$stubs:$PATH" AGENT_DESKTOP_REMOTE_LOCK=0 bash "$here/remote-linux.sh" "$@" > /dev/null 2>&1; then
    return 1
  fi
  [ ! -s "$log" ]
}

passes() {
  : > "$log"
  PATH="$stubs:$PATH" AGENT_DESKTOP_REMOTE_LOCK=0 bash "$here/remote-linux.sh" "$@" > /dev/null 2>&1 && [ -s "$log" ]
}

verdict=bad; refused -c 'pkill -f gnome-calculator' && verdict=ok
check "pkill is refused before anything is sent" "$verdict"
verdict=bad; refused -c 'echo a; killall nautilus' && verdict=ok
check "killall is refused before anything is sent" "$verdict"
verdict=bad; refused -c 'kill -f gnome-calculator' && verdict=ok
check "a kill with a name is refused" "$verdict"
verdict=bad; refused pkill -f x && verdict=ok
check "pkill in an argv command is refused" "$verdict"
verdict=bad; passes -c 'kill 1234' && verdict=ok
check "kill by a number passes" "$verdict"
verdict=bad
# shellcheck disable=SC2016
passes -c 'kill -TERM "$pid"' && verdict=ok
check "kill with a signal and a variable passes" "$verdict"
verdict=bad
# shellcheck disable=SC2016
passes -c 'kill -0 $p' && verdict=ok
check "kill -0 with a variable passes" "$verdict"
verdict=bad
# shellcheck disable=SC2016
passes -c 'kill -KILL -- -$pid 2>/dev/null; kill -KILL $pid' && verdict=ok
check "a group kill and a pid kill with a redirect pass" "$verdict"

exit "$failed"
