#!/usr/bin/env bash
# rsync + ssh. No devcontainer, no remote git remote, no CI runner.
# Usage: scripts/remote-linux.sh [cmd args...]    argv runs verbatim, quoted per argument
#        scripts/remote-linux.sh -c 'shell string'  the string runs under bash (pipes, globs)
#        default command: cargo test -p agent-desktop-linux
#        stdin reaches the remote command: scripts/remote-linux.sh python3 - < probe.py > capture.json
# Each worktree syncs to its own remote dir. All of them share one remote CARGO_TARGET_DIR:
# the box disk cannot hold a target dir per worktree, and cargo's build lock serializes them.
# Gitignored files (.env, .claude/, target/) never leave this machine.
# One lock serializes every run, because two runs driving the same X display corrupt
# each other's input, focus and hit-test results.
set -euo pipefail
HOST=${AGENT_DESKTOP_REMOTE:-linuxbox}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIR=${AGENT_DESKTOP_REMOTE_DIR:-ad-wt/$(basename "$ROOT")}
TIMEOUT=${AGENT_DESKTOP_REMOTE_TIMEOUT:-3600}

if [ "${1:-}" = "-c" ]; then
  [ $# -eq 2 ] || { echo "remote-linux: -c takes exactly one string" >&2; exit 2; }
  cmd="bash -c $(printf '%q' "$2")"
elif [ $# -gt 0 ]; then
  cmd=$(printf '%q ' "$@")
else
  cmd="cargo test -p agent-desktop-linux"
fi
case "$cmd" in *sudo*) echo "remote-linux: refusing sudo; the box is shared and runs unattended" >&2; exit 2 ;; esac

mkdir -p "$ROOT/.claude"
printf '%s\t%s\t%s\n' "$(date -u +%FT%TZ)" "$DIR" "$cmd" >> "$ROOT/.claude/remote-linux.log"

qdir=$(printf '%q' "$DIR")
ssh -n -o BatchMode=yes "$HOST" "mkdir -p $qdir"
rsync -az --delete --exclude .git/ --filter=':- .gitignore' "$ROOT/" "$HOST:$DIR/"
ssh -o BatchMode=yes "$HOST" "cd $qdir && { [ ! -f \$HOME/.cargo/env ] || . \$HOME/.cargo/env; } \
  && export CARGO_TARGET_DIR=\$HOME/ad-target && . ./scripts/session-env.sh \
  && flock \$HOME/ad-wt/.box.lock timeout $TIMEOUT $cmd"
