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
# Two box slots bound concurrent runs (scripts/box-slot.sh). Runs never share an X display:
# each drives its own private session, and physical input on any display is serialized by
# the product's interaction lease. AGENT_DESKTOP_REMOTE_LOCK=0 skips the slots; use it only
# for read-only commands that touch no display, input or app state. -o keeps the slot lock
# out of children that outlive the command, such as xclip serving a selection, and the wait
# for a slot is bounded by the same timeout as the command.
# AGENT_DESKTOP_REMOTE_SYNC=0 reuses the existing checkout for reading receipts or recovering
# a full disk, where even rsync cannot finish. Builds use the default sync.
# Two runs from the same worktree share one remote folder, and the sync happens before the lock,
# so runs from one worktree must not overlap.
set -euo pipefail
HOST=${AGENT_DESKTOP_REMOTE:-linuxbox}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIR=${AGENT_DESKTOP_REMOTE_DIR:-ad-wt/$(basename "$ROOT")}
TIMEOUT=${AGENT_DESKTOP_REMOTE_TIMEOUT:-3600}
case "$TIMEOUT" in ""|*[!0-9]*) echo "remote-linux: AGENT_DESKTOP_REMOTE_TIMEOUT must be whole seconds" >&2; exit 2 ;; esac
LOCK_WAIT=${AGENT_DESKTOP_REMOTE_LOCK_WAIT:-${AGENT_DESKTOP_REMOTE_LOCK_TIMEOUT:-$TIMEOUT}}
case "$LOCK_WAIT" in ""|*[!0-9]*) echo "remote-linux: AGENT_DESKTOP_REMOTE_LOCK_WAIT must be whole seconds" >&2; exit 2 ;; esac

# A kill by process name reaches every session of the user on the shared box, so only a kill by pid
# is sent: after `kill`, every word is a number, a signal option or a variable.
names_a_process_to_kill() {
  local chunk word
  if printf '%s' "$1" | grep -Eq '(^|[^[:alnum:]_.-])(pkill|killall)([^[:alnum:]_.-]|$)'; then
    return 0
  fi
  while IFS= read -r chunk; do
    chunk=${chunk#kill}
    for word in $chunk; do
      case "$word" in
        --|-s|-n|-[0-9]*|-[A-Z]*|[0-9]*|[A-Z][A-Z0-9]*|\$*|\"\$*|\"-\$*|-\$*|\'\$*|\>*) ;;
        *) return 0 ;;
      esac
    done
  done < <(printf '%s\n' "$1" | grep -oE '(^|[^[:alnum:]_.-])kill([[:space:]][^;&|)]*)?' | sed -E 's/^[^k]*//')
  return 1
}
to_check=$*
[ "${1:-}" = "-c" ] && to_check=${2:-}
if names_a_process_to_kill "$to_check"; then
  echo "remote-linux: kill by pid only; name-based kills reach every session on the box" >&2
  exit 2
fi

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
# Every box dir shares one CARGO_TARGET_DIR, so cargo can take another worktree's build for fresh;
# touching the sources inside the lock makes it rebuild this worktree's own.
wrap=
case "$cmd" in *cargo*|*linux-live-gate*) wrap='sh -c '"'"'find crates src -name \*.rs -exec touch {} + ; exec "$@"'"'"' sh' ;; esac
lock="sh ./scripts/box-slot.sh $LOCK_WAIT"
[ "${AGENT_DESKTOP_REMOTE_LOCK:-1}" = 0 ] && lock=""
ssh -n -o BatchMode=yes "$HOST" "mkdir -p \$HOME/ad-wt $qdir"
if [ "${AGENT_DESKTOP_REMOTE_SYNC:-1}" != 0 ]; then
  filter=$(mktemp)
  trap 'rm -f -- "$filter"' EXIT
  git -C "$ROOT" ls-files -co --exclude-standard | awk -F/ 'function esc(s){gsub(/[][*?\\]/, "\\\\&", s); return s} {p=""; for(i=1;i<NF;i++){p=p $i "/"; if(!(p in d)){d[p]=1; print "+ /" esc(p)}} print "+ /" esc($0)} END{print "- *"}' > "$filter"
  rsync -az --delete --delete-excluded --filter="merge $filter" "$ROOT/" "$HOST:$DIR/"
fi
ssh -o BatchMode=yes "$HOST" "cd $qdir && { [ ! -f \$HOME/.cargo/env ] || . \$HOME/.cargo/env; } \
  && export CARGO_TARGET_DIR=\$HOME/ad-target CARGO_INCREMENTAL=0 && . ./scripts/session-env.sh \
  && { timeout -k 30 $TIMEOUT $lock $wrap $cmd; rc=\$?; \
  ls \$XDG_RUNTIME_DIR/at-spi/bus* >/dev/null 2>&1 || echo 'remote-linux: WARNING the live session AT-SPI bus socket is missing' >&2; \
  exit \$rc; }"
