#!/usr/bin/env bash
# rsync + ssh. No devcontainer, no remote git remote, no CI runner.
# Usage: scripts/remote-linux.sh [cmd args...]    argv runs verbatim, quoted per argument
#        scripts/remote-linux.sh -c 'shell string'  the string runs under bash (pipes, globs)
#        default command: cargo test -p agent-desktop-linux
#        stdin reaches the remote command: scripts/remote-linux.sh python3 - < probe.py > capture.json
# Each worktree syncs to its own remote dir. All of them share one remote CARGO_TARGET_DIR:
# the box disk cannot hold a target dir per worktree, and cargo's build lock serializes them.
# Only files git tracks or would add are synced (git ls-files), so gitignored files such as .env,
# .claude/ and target/ never leave this machine; rsync cannot read .gitignore itself, since a
# leading ! re-includes in git but clears the rule list in rsync.
# After every run it warns if the live AT-SPI bus socket is gone: a second bus launcher sharing
# XDG_RUNTIME_DIR removes it on exit, and every later AT-SPI client then fails. For that reason a
# command that starts dbus-run-session is refused unless it also sets XDG_RUNTIME_DIR.
# One lock serializes every run, because two runs driving the same X display corrupt
# each other's input, focus and hit-test results. AGENT_DESKTOP_REMOTE_LOCK=0 skips the lock;
# use it only for read-only commands that touch no display, input or app state. -o keeps the
# lock out of children that outlive the command, such as xclip serving a selection, and -w
# bounds the wait for the lock by the same timeout as the command.
# Two runs from the same worktree share one remote folder, and the sync happens before the lock,
# so runs from one worktree must not overlap.
set -euo pipefail
HOST=${AGENT_DESKTOP_REMOTE:-linuxbox}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIR=${AGENT_DESKTOP_REMOTE_DIR:-ad-wt/$(basename "$ROOT")}
TIMEOUT=${AGENT_DESKTOP_REMOTE_TIMEOUT:-3600}
case "$TIMEOUT" in ""|*[!0-9]*) echo "remote-linux: AGENT_DESKTOP_REMOTE_TIMEOUT must be whole seconds" >&2; exit 2 ;; esac

if [ "${1:-}" = "-c" ]; then
  [ $# -eq 2 ] || { echo "remote-linux: -c takes exactly one string" >&2; exit 2; }
  cmd="bash -c $(printf '%q' "$2")"
elif [ $# -gt 0 ]; then
  cmd=$(printf '%q ' "$@")
else
  cmd="cargo test -p agent-desktop-linux"
fi
case "$cmd" in *sudo*) echo "remote-linux: refusing sudo; the box is shared and runs unattended" >&2; exit 2 ;; esac
case "$cmd" in *dbus-run-session*)
  case "$cmd" in *XDG_RUNTIME_DIR=*) ;; *)
    echo "remote-linux: refusing dbus-run-session without a private XDG_RUNTIME_DIR=<0700 dir>; its bus launcher would delete the live AT-SPI socket" >&2; exit 2 ;;
  esac ;;
esac

mkdir -p "$ROOT/.claude"
printf '%s\t%s\t%s\n' "$(date -u +%FT%TZ)" "$DIR" "$cmd" >> "$ROOT/.claude/remote-linux.log"

qdir=$(printf '%q' "$DIR")
lock="flock -o -w $TIMEOUT \$HOME/ad-wt/.box.lock"
[ "${AGENT_DESKTOP_REMOTE_LOCK:-1}" = 0 ] && lock=""
ssh -n -o BatchMode=yes "$HOST" "mkdir -p \$HOME/ad-wt $qdir"
filter=$(mktemp)
trap 'rm -f "$filter"' EXIT
git -C "$ROOT" ls-files -co --exclude-standard | awk -F/ 'function esc(s){gsub(/[][*?\\]/, "\\\\&", s); return s} {p=""; for(i=1;i<NF;i++){p=p $i "/"; if(!(p in d)){d[p]=1; print "+ /" esc(p)}} print "+ /" esc($0)} END{print "- *"}' > "$filter"
rsync -az --delete --delete-excluded --filter="merge $filter" "$ROOT/" "$HOST:$DIR/"
ssh -o BatchMode=yes "$HOST" "cd $qdir && { [ ! -f \$HOME/.cargo/env ] || . \$HOME/.cargo/env; } \
  && export CARGO_TARGET_DIR=\$HOME/ad-target && . ./scripts/session-env.sh \
  && { $lock timeout -k 30 $TIMEOUT $cmd; rc=\$?; \
  ls \$XDG_RUNTIME_DIR/at-spi/bus* >/dev/null 2>&1 || echo 'remote-linux: WARNING the live session AT-SPI bus socket is missing' >&2; \
  exit \$rc; }"
