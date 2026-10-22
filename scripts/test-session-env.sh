#!/usr/bin/env bash
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ad-session-env.XXXXXX")
cleanup() {
  python3 -c 'import shutil, sys; shutil.rmtree(sys.argv[1])' "$scratch"
}
trap cleanup EXIT

python3 - "$here/session-env.sh" "$scratch" <<'PY'
from pathlib import Path
import sys

source, root = map(Path, sys.argv[1:])
proc = root / "proc"
for pid in (11, 12, 13, 14):
    directory = proc / str(pid)
    directory.mkdir(parents=True)
    (directory / "comm").write_text("gnome-shell\n")

def environ(pid, entries):
    (proc / str(pid) / "environ").write_bytes(b"\0".join(s.encode() for s in entries) + b"\0")

environ(11, ["AGENT_DESKTOP_PRIVATE_SESSION=/private", "DISPLAY=:101",
             "XDG_RUNTIME_DIR=/private", "DBUS_SESSION_BUS_ADDRESS=unix:path=/private/bus",
             "NOISE=" + "x" * 100_000])
environ(13, ["DISPLAY=:102", "XDG_RUNTIME_DIR="])
environ(14, ["DISPLAY=:7", "XDG_RUNTIME_DIR=/real", "XAUTHORITY=/real/auth",
             "DBUS_SESSION_BUS_ADDRESS=unix:path=/real/bus", "PARENT_ONLY=do-not-import"])
(root / "session-env.sh").write_text(source.read_text().replace("/proc/", str(proc) + "/"))
PY

mkdir "$scratch/bin"
cat > "$scratch/bin/pgrep" <<'SH'
#!/usr/bin/env bash
for name do :; done
found=1
for directory in "$SESSION_ENV_FAKE_PROC"/*; do
  if [ "$(cat "$directory/comm")" = "$name" ]; then
    printf '%s\n' "${directory##*/}"
    found=0
  fi
done
exit "$found"
SH
chmod +x "$scratch/bin/pgrep"
export PATH="$scratch/bin:$PATH" SESSION_ENV_FAKE_PROC="$scratch/proc"
export DISPLAY=:stale WAYLAND_DISPLAY=stale XAUTHORITY=stale
export XDG_RUNTIME_DIR=/private DBUS_SESSION_BUS_ADDRESS=unix:path=/private/bus PARENT_ONLY=kept

# shellcheck source=/dev/null
observed=$(. "$scratch/session-env.sh"; printf '%s\n' "$DISPLAY|$XDG_RUNTIME_DIR|$DBUS_SESSION_BUS_ADDRESS|$XAUTHORITY|${WAYLAND_DISPLAY-unset}|$PARENT_ONLY")
expected=':7|/real|unix:path=/real/bus|/real/auth|unset|kept'
if [ "$observed" != "$expected" ]; then
  printf 'FAIL: private shell listed first: expected %s; got %s\n' "$expected" "$observed" >&2
  exit 1
fi

python3 -c 'import shutil, sys; shutil.rmtree(sys.argv[1])' "$scratch/proc/14"
# shellcheck source=/dev/null
if (. "$scratch/session-env.sh") 2>/dev/null; then
  echo 'FAIL: private or unreadable environments cannot establish a live desktop' >&2
  exit 1
fi
echo 'ok: private shells are skipped and absent live desktop evidence refuses'
