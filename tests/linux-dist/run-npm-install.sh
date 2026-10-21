#!/usr/bin/env bash
# Install the packed npm package the way a user does, then drive what it
# installed.
#
# Usage: run-npm-install.sh <archives-dir>
#
# The directory holds the dry-run CLI archive for this architecture
# (agent-desktop-v*-<target>.tar.gz). A PATH-first curl double serves it and a
# regenerated checksums.txt, so the run proves the postinstall downloaded and
# verified rather than finding a binary that was already there.
# AGENT_DESKTOP_BINARY_PATH is deliberately never set: it makes zero downloads.
set -euo pipefail

ARCHIVES=${1:?usage: run-npm-install.sh <archives-dir>}
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
SCRIPT_DIR="$ROOT/tests/linux-dist"
APP=${AD_APP:-gnome-calculator}
ATSPI_APP=${AD_ATSPI_APP:-calculator}

case "$(uname -m)" in
  x86_64)
    TARGET=x86_64-unknown-linux-gnu
    ARCH=x86_64
    ;;
  aarch64|arm64)
    TARGET=aarch64-unknown-linux-gnu
    ARCH=aarch64
    ;;
  *)
    echo "run-npm-install: unsupported architecture $(uname -m)" >&2
    exit 2
    ;;
esac

command -v node >/dev/null 2>&1 || { echo "run-npm-install: node is required" >&2; exit 2; }
command -v npm >/dev/null 2>&1 || { echo "run-npm-install: npm is required" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "run-npm-install: python3 is required" >&2; exit 2; }

VERSION=$(node -p "require('$ROOT/npm/package.json').version")

matches=("$ARCHIVES"/agent-desktop-v*-"$TARGET".tar.gz)
if [ "${#matches[@]}" -ne 1 ] || [ ! -f "${matches[0]}" ]; then
  echo "run-npm-install: expected one CLI archive for $TARGET in $ARCHIVES" >&2
  exit 2
fi
ARCHIVE=${matches[0]}

WORK=$(mktemp -d "${AGENT_DESKTOP_PRIVATE_SESSION:-${TMPDIR:-/tmp}}/ad-npm.XXXXXX")
export AGENT_DESKTOP_HOME="$WORK/state"
export npm_config_cache="$WORK/npm-cache"
export npm_config_update_notifier=false
PREFIX="$WORK/prefix"
SERVE="$WORK/serve"
DOUBLE="$WORK/double-bin"
LOG="$WORK/curl.log"
mkdir -m 700 "$AGENT_DESKTOP_HOME"
mkdir -p "$PREFIX" "$SERVE" "$DOUBLE"
APP_PID=

cleanup() {
  if [ -n "$APP_PID" ]; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
  trash -- "$WORK"
}
trap cleanup EXIT

SERVED="agent-desktop-v$VERSION-$TARGET.tar.gz"
cp "$ARCHIVE" "$SERVE/$SERVED"
(cd "$SERVE" && sha256sum "$SERVED" > checksums.txt)
ln -s "$SCRIPT_DIR/curl-double.sh" "$DOUBLE/curl"
chmod +x "$SCRIPT_DIR/curl-double.sh"

echo "run-npm-install: packing $VERSION"
TGZ=$(cd "$ROOT/npm" && npm pack --pack-destination "$WORK" --silent)
TGZ="$WORK/$TGZ"
[ -f "$TGZ" ] || { echo "run-npm-install: npm pack produced no tarball" >&2; exit 1; }

install_package() {
  PATH="$DOUBLE:$PATH" \
    AD_CURL_DOUBLE_DIR="$SERVE" \
    AD_CURL_DOUBLE_LOG="$LOG" \
    npm install -g "$TGZ" --prefix "$PREFIX" --no-audit --no-fund --loglevel=error
}

echo "run-npm-install: first install"
: > "$LOG"
install_package
requests=$(wc -l < "$LOG" | tr -d ' ')
if [ "$requests" -ne 2 ]; then
  echo "run-npm-install: FAIL code=DOWNLOAD_COUNT requests=$requests expected=2" >&2
  exit 1
fi
grep -q "agent-desktop-v$VERSION-$TARGET.tar.gz$" "$LOG" || {
  echo "run-npm-install: FAIL code=ARCHIVE_NOT_REQUESTED" >&2
  exit 1
}
grep -q "/checksums.txt$" "$LOG" || {
  echo "run-npm-install: FAIL code=CHECKSUMS_NOT_REQUESTED" >&2
  exit 1
}

echo "run-npm-install: second install reuses the recorded digests"
: > "$LOG"
PATH="$DOUBLE:$PATH" \
  AD_CURL_DOUBLE_DIR="$SERVE" \
  AD_CURL_DOUBLE_LOG="$LOG" \
  node "$PREFIX/lib/node_modules/agent-desktop/scripts/postinstall.js"
requests=$(wc -l < "$LOG" | tr -d ' ')
if [ "$requests" -ne 0 ]; then
  echo "run-npm-install: FAIL code=DIGEST_REUSE requests=$requests expected=0" >&2
  exit 1
fi

check_version() {
  local source=$1 json=$2
  printf '%s' "$json" > "$WORK/version.json"
  python3 - "$source" "$ARCH" "$WORK/version.json" <<'PY'
import json
import sys

source, arch, path = sys.argv[1], sys.argv[2], sys.argv[3]
document = json.load(open(path, encoding="utf-8"))
assert document.get("ok") is True, f"{source}: version did not succeed"
data = document["data"]
assert data.get("os") == "linux", f"{source}: os is {data.get('os')}"
assert data.get("target") == arch, f"{source}: target is {data.get('target')}"
PY
}

BIN="$PREFIX/bin/agent-desktop"
WRAPPER="$PREFIX/lib/node_modules/agent-desktop/bin/agent-desktop.js"
[ -x "$BIN" ] || { echo "run-npm-install: FAIL code=SYMLINK_MISSING $BIN" >&2; exit 1; }
[ -f "$WRAPPER" ] || { echo "run-npm-install: FAIL code=WRAPPER_MISSING $WRAPPER" >&2; exit 1; }

check_version symlink "$("$BIN" version)"
check_version wrapper "$(node "$WRAPPER" version)"
echo "run-npm-install: symlink and wrapper report os=linux target=$ARCH"

NODE_BIN=$(command -v node)
mkdir "$WORK/nopath"
python3 "$ROOT/tests/ffi-python/linux_helper.py" "$NODE_BIN" "$WRAPPER" "$WORK/nopath"

"$APP" >/dev/null 2>&1 &
APP_PID=$!
SNAP="$WORK/snapshot.json"
if ! python3 "$SCRIPT_DIR/atspi_read.py" --app "$ATSPI_APP" --timeout 30 >/dev/null; then
  echo "run-npm-install: FAIL code=APP_NOT_FOUND target=snapshot ($APP never registered)" >&2
  exit 1
fi
if ! "$BIN" snapshot --app "$APP" -i --max-depth 10 > "$SNAP" 2>/dev/null; then
  echo "run-npm-install: FAIL code=APP_NOT_FOUND target=snapshot ($APP did not snapshot)" >&2
  exit 1
fi
python3 - "$SNAP" <<'PY'
import json
import sys

document = json.load(open(sys.argv[1], encoding="utf-8"))
data = document["data"]
assert document["ok"] is True, "snapshot is not ok"
assert data["complete"] is True, "snapshot is incomplete"
assert data["ref_count"] > 0, "snapshot allocated no refs"
PY
echo "run-npm-install: installed snapshot ok"

find_ref() {
  python3 - "$1" "$2" "$3" <<'PY'
import json
import sys

path, role, name = sys.argv[1], sys.argv[2], sys.argv[3]

def walk(node):
    if isinstance(node, dict):
        if node.get("role") == role and node.get("name") == name and node.get("ref_id"):
            yield node["ref_id"]
        for value in node.values():
            if isinstance(value, (dict, list)):
                yield from walk(value)
    elif isinstance(node, list):
        for item in node:
            yield from walk(item)

with open(path, encoding="utf-8") as stream:
    document = json.load(stream)
refs = list(walk(document))
if len(refs) != 1:
    raise SystemExit(f"expected one {role} named {name!r}, found {refs}")
print(refs[0])
PY
}

read_display() {
  python3 "$SCRIPT_DIR/atspi_read.py" --app "$ATSPI_APP" --timeout "${1:-20}"
}

before=$(read_display 20) || before=""
ref=$(find_ref "$SNAP" button 7)
"$BIN" click "$ref" > "$WORK/click.json"
shown=""
for _ in $(seq 1 100); do
  shown=$(read_display 5) || shown=""
  [ "$shown" = "7" ] && break
  sleep 0.05
done
if [ "$shown" != "7" ]; then
  echo "run-npm-install: FAIL code=ACTION_NOT_DELIVERED display=$shown before=$before" >&2
  exit 1
fi
echo "run-npm-install: installed click ok display=$shown before=$before"

set +e
"$BIN" snapshot --app "agent-desktop-missing-$RANDOM" -i > /dev/null 2>&1
missing_rc=$?
set -e
if [ "$missing_rc" -ne 1 ]; then
  echo "run-npm-install: FAIL code=EXIT_CODE rc=$missing_rc expected=1" >&2
  exit 1
fi
echo "run-npm-install: a missing app exits 1"
echo "run-npm-install: all legs passed"
