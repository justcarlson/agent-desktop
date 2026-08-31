#!/usr/bin/env bash
# ponytail: rsync + ssh. No devcontainer, no remote git remote, no CI runner.
# Usage: scripts/remote-linux.sh [command...]   default: cargo test -p agent-desktop-linux
set -euo pipefail
HOST=${AGENT_DESKTOP_REMOTE:-linuxbox}
DIR=${AGENT_DESKTOP_REMOTE_DIR:-agent-desktop}
cd "$(dirname "$0")/.."
rsync -az --delete --exclude target/ --exclude .git/ ./ "$HOST:$DIR/"
ssh "$HOST" "cd '$DIR' && [ -f \$HOME/.cargo/env ] && . \$HOME/.cargo/env; . ./scripts/session-env.sh && ${*:-cargo test -p agent-desktop-linux}"
