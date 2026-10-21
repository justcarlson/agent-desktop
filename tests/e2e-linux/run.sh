#!/usr/bin/env bash
# Drive the live harness through the canonical interaction lease so headed input
# cannot interleave with another run on the same host. The release binary is
# built once here; runner/main.py stages it under a private suite directory.
set -euo pipefail
export PATH="$HOME/.local/ad-tools/root/usr/bin:$PATH"

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../.." && pwd)
e2e=$repo/tests/e2e

build=1
for arg in "$@"; do
  case "$arg" in
    --lying-actions) build=0 ;;
  esac
done
if [ -n "${AGENT_DESKTOP_E2E_BIN:-}" ] || [ -n "${AGENT_DESKTOP_E2E_BIN_SRC:-}" ]; then
  build=0
fi
if [ "$build" = 1 ]; then
  (cd "$repo" && export CARGO_INCREMENTAL=0 && cargo build --locked --release -p agent-desktop -p agent-desktop-linux --bins)
fi

exec python3 "$e2e/interaction_lock.py" run python3 -I "$here/runner/main.py" "$@"
