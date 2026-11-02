"""Which config dialect the window legs' legacy instance answered in, judged by the VM channel.

The legacy instance reads a `hyprland.conf`. Stable (Hyprland 0.56.2) still has the legacy config
manager, so the instance must answer in the legacy dialect there, or the legacy dispatch is never
tested. Edge (Hyprland 0.57.0) reads the same file as Lua (probes/linux/FINDINGS.md L46-4), so edge
runs the Lua cases twice and the run says so. `lua` comes from `eval return 1` on the instance.
"""

from __future__ import annotations

import os

CHANNEL = "OMARCHY_VM_CHANNEL"


def channel() -> str | None:
    """The Omarchy channel the VM booted, as `test/omarchy-vm` exports it."""
    return os.environ.get(CHANNEL) or None


def dialect_problems(name: str | None, lua: bool) -> list[str]:
    """Stable must answer a conf file in the legacy dialect; a run with no channel cannot be judged."""
    if not name:
        return [f"{CHANNEL} is not set, so the legacy instance's dialect (lua={lua}) cannot be judged"]
    if name == "stable" and lua:
        return ["on stable the legacy config instance answered in Lua, so no legacy dispatch was tested"]
    return []


def dialect_note(name: str | None, lua: bool) -> str:
    """The run-log evidence: the channel and the flag the legacy instance answered with."""
    if name == "edge" and lua:
        return f"legacy config instance on edge: lua={lua}; edge is Lua only, the legacy cases ran in Lua"
    dialect = "Lua" if lua else "the legacy dialect"
    return f"legacy config instance on {name or 'an unnamed channel'}: lua={lua}; it answered in {dialect}"
