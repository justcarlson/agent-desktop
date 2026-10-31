"""Toplevels the Hyprland capture legs start: a foot terminal that runs a given command, or any program that maps
one window of a known class, each started in the run's slice and stopped when the leg ends."""

from __future__ import annotations

import contextlib
from pathlib import Path

from oracle import atspi_raw as raw, hyprland, proc
from runner import apps, attached_session
from runner.apps import OwnedApp
from runner.helper import require
from scenarios.hyprland_windows import clients


def terminal_argv(app_id: str, command: list[str]) -> list[str]:
    """A foot toplevel titled `app_id` that runs `command`."""
    return ["foot", "--app-id", app_id, "--title", app_id, *command]


def _new_client(tier, app_id: str, known: set[str]) -> dict | None:
    found = [item for item in clients(tier)
             if item.get("class") == app_id and item.get("address") not in known]
    require(len(found) <= 1, f"several new {app_id} clients appeared: {found}")
    return found[0] if found and found[0].get("mapped") else None


@contextlib.contextmanager
def terminal(tier, app_id: str, command: list[str]):
    """A foot toplevel with no accessibility tree, started in the run's slice, running `command`."""
    with toplevel(tier, app_id, terminal_argv(app_id, command)) as started:
        yield started


@contextlib.contextmanager
def toplevel(tier, app_id: str, argv: list[str], env: dict[str, str] | None = None):
    """The one toplevel of class `app_id` that `argv` maps, started in the run's slice."""
    hyprland.lua_string(app_id)
    known = {item.get("address") for item in clients(tier)}
    attached_session.spawn(tier, argv, {**tier.env, **(env or {})}, tier.runtime_dir / "terminal-launch.log")
    found = raw.poll(lambda: _new_client(tier, app_id, known), 10.0)
    require(found, f"the {app_id} window never mapped")
    pid = found["pid"]
    start = proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]
    require(proc.owned(tier, pid, start, {}), f"the {app_id} window is not in the run's slice")
    owned = OwnedApp(app_id, pid, start, [], [], "")
    apps._TIERS[pid, start] = tier
    try:
        yield owned, found
    finally:
        apps.stop(owned)
        apps._TIERS.pop((pid, start), None)
