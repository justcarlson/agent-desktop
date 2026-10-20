#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) NAME=agent-desktop-darwin-arm64; HELPER_NAME=agent-desktop-macos-helper; EXPECTED_OS=macos ;;
  Darwin-x86_64) NAME=agent-desktop-darwin-x64; HELPER_NAME=agent-desktop-macos-helper; EXPECTED_OS=macos ;;
  Linux-x86_64) NAME=agent-desktop-linux-x64; HELPER_NAME=agent-desktop-linux-clipboard-owner; EXPECTED_OS=linux ;;
  Linux-aarch64) NAME=agent-desktop-linux-arm64; HELPER_NAME=agent-desktop-linux-clipboard-owner; EXPECTED_OS=linux ;;
  *)
    echo "Unsupported smoke-test platform: $(uname -s)-$(uname -m)"
    exit 1
    ;;
esac

digest() {
  if [ "$(uname -s)" = "Linux" ]; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

SOURCE=target/release/agent-desktop
HELPER_SOURCE="target/release/${HELPER_NAME}"
VERSION_JSON=$(mktemp "${TMPDIR:-/tmp}/agent-desktop-version.XXXXXX")
cleanup() {
  rm -f "$VERSION_JSON" "npm/bin/${NAME}" "npm/bin/${HELPER_NAME}"
}
trap cleanup EXIT

SOURCE_SHA=$(digest "$SOURCE")
HELPER_SOURCE_SHA=$(digest "$HELPER_SOURCE")
cp "$SOURCE" "npm/bin/${NAME}"
cp "$HELPER_SOURCE" "npm/bin/${HELPER_NAME}"
chmod +x "npm/bin/${NAME}"
chmod +x "npm/bin/${HELPER_NAME}"
COPY_SHA=$(digest "npm/bin/${NAME}")
HELPER_COPY_SHA=$(digest "npm/bin/${HELPER_NAME}")
if [ "$SOURCE_SHA" != "$COPY_SHA" ] || [ "$HELPER_SOURCE_SHA" != "$HELPER_COPY_SHA" ]; then
  echo "NPM smoke executable copy failed immutable identity verification" >&2
  exit 1
fi
if [ ! -x "npm/bin/${HELPER_NAME}" ]; then
  echo "NPM smoke helper ${HELPER_NAME} is not executable beside ${NAME}" >&2
  exit 1
fi
node npm/bin/agent-desktop.js version > "$VERSION_JSON"
EXPECTED_OS="$EXPECTED_OS" VERSION_JSON="$VERSION_JSON" node -e "
  const out = require('fs').readFileSync(process.env.VERSION_JSON, 'utf8');
  const json = JSON.parse(out);
  if (json.ok !== true || !json.data || typeof json.data.version !== 'string') {
    throw new Error('agent-desktop npm wrapper did not return version JSON');
  }
  if (json.data.os !== process.env.EXPECTED_OS) {
    throw new Error('agent-desktop npm wrapper reported os ' + json.data.os + ', expected ' + process.env.EXPECTED_OS);
  }
"
if [ "$SOURCE_SHA" != "$(digest "$SOURCE")" ] || \
   [ "$COPY_SHA" != "$(digest "npm/bin/${NAME}")" ] || \
   [ "$HELPER_SOURCE_SHA" != "$(digest "$HELPER_SOURCE")" ] || \
   [ "$HELPER_COPY_SHA" != "$(digest "npm/bin/${HELPER_NAME}")" ]; then
  echo "NPM smoke executable changed while it was executed" >&2
  exit 1
fi
