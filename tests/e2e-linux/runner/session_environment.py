from __future__ import annotations

import os
import subprocess

_PAYLOAD = r"""
rt=$AGENT_DESKTOP_PRIVATE_SESSION
mkdir -p "$rt/ad-home"
env | grep -E '^(XDG_RUNTIME_DIR|DISPLAY|WAYLAND_DISPLAY|DBUS_SESSION_BUS_ADDRESS|XDG_CONFIG_HOME|XDG_DATA_HOME|XDG_CACHE_HOME|TMPDIR|LD_LIBRARY_PATH|GSETTINGS_BACKEND|XDG_CURRENT_DESKTOP|AGENT_DESKTOP_PRIVATE_SESSION|AGENT_DESKTOP_HOME)=' > "$AD_TIER_ENV_FILE.tmp"
mv "$AD_TIER_ENV_FILE.tmp" "$AD_TIER_ENV_FILE"
printf '%s %s\n' "$AD_RUNNER_PID" "$AD_RUNNER_START" > "$rt/harness-owner"
trap 'exit 0' TERM INT
mkfifo "$rt/release-pipe"
exec 9<>"$rt/release-pipe"
while [ ! -f "$AD_TIER_RELEASE_FILE" ]; do read -t 0.2 -u 9 || :; done
"""

_ENV_KEYS = (
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "TMPDIR",
    "LD_LIBRARY_PATH",
    "GSETTINGS_BACKEND",
    "XDG_CURRENT_DESKTOP",
    "AGENT_DESKTOP_PRIVATE_SESSION",
    "AGENT_DESKTOP_HOME",
    "DISPLAY",
    "WAYLAND_DISPLAY",
)


def _parse_env_file(text: str) -> dict[str, str]:
    env = {}
    for line in text.splitlines():
        key, sep, value = line.partition("=")
        if sep and key in _ENV_KEYS:
            env[key] = value
    return env


def _stderr_tail(proc: subprocess.Popen) -> str:
    if proc.stderr is None:
        return ""
    try:
        os.set_blocking(proc.stderr.fileno(), False)
        data = proc.stderr.read() or b""
    except (OSError, ValueError):
        return ""
    return data[-800:].decode("utf-8", "replace").strip()
