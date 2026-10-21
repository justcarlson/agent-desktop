#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/ad-tools/root/usr/bin:$PATH"

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
exec python3 -I "$here/runner/prepare.py" "$@"
