"""Hyprland window operations that must refuse, in the leg's mode, headless or headed.

The target is the window sizer, parked alone and tiled on a spare workspace no monitor shows.
Each refusal is read three ways: `j/clients` shows the target where it was, its process is the
same generation, and its own log shows no size or state. A sampler reads the desktop and the
bystander throughout: no refusal and no headed run may focus, show a workspace or move the
cursor. The supported counterpart of each request then works in the same mode, so the refusals
are not a broken route.
"""

from __future__ import annotations

import contextlib
from pathlib import Path

from oracle import atspi_raw as raw, proc
from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_focus_controls import (control, cursor, listed_ids, move, restore_desktop,
                                               spare_workspaces, workspaces_in_use)
from scenarios.hyprland_window_app import quiet_problems, settled_count, sizer
from scenarios.hyprland_window_checks import MINIMIZED, SETTLE_S, float_argv, operated, point, refusal, window_argv
from scenarios.hyprland_window_ops import _absent, _foreign, located, watched

LABEL = "refusals"


def generation(pid: int) -> list[int]:
    """The process and its start time, which a restart changes."""
    return [pid, proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]]


@contextlib.contextmanager
def parked_sizer(ctx: LegContext, holder: dict, pointer: dict):
    """The sizer alone and tiled on a spare workspace, the desktop given back to the bystander.
    Yields its window id, address, workspace, pid and log."""
    tier = ctx.tier
    settled = located(tier, holder["address"])()
    with sizer(tier, main_instance(tier), LABEL) as (owned, found, log):
        ids = listed_ids(ctx.product(["list-windows"], headed=False), owned.pid)
        require(len(ids) == 1, f"list-windows gives the sizer {ids}")
        workspace = spare_workspaces(workspaces_in_use(tier), 1)[0]
        move(tier, found["address"], workspace)
        restore_desktop(tier, holder, pointer)
        require(raw.poll(lambda: located(tier, holder["address"])() == settled, SETTLE_S),
                f"the bystander did not settle back to {settled}")
        yield ids[0], found["address"], workspace, owned.pid, log


def _refused(ctx: LegContext, log: Path, label: str, argv: list[str], read, cause: str) -> list[str]:
    count = settled_count(log)
    return refusal(ctx, label, argv, read, cause) + quiet_problems(label, log, count)


def _unsupported(ctx: LegContext, target: tuple, read) -> list[str]:
    window_id, address, workspace, _, log = target
    argv = lambda verb, *extra: window_argv(verb, window_id, *extra)
    problems = _refused(ctx, log, "tiled move", argv("move-window", "--x", "40", "--y", "60"), read, "tiled_window")
    problems += _refused(ctx, log, "tiled resize", argv("resize-window", "--width", "500", "--height", "400"), read,
                         "tiled_window")
    problems += _refused(ctx, log, "maximize while the bystander holds focus", argv("maximize"), read,
                         "maximize_takes_focus")
    problems += _foreign(ctx, read, window_id, address, workspace)
    problems += operated(ctx, "minimize", argv("minimize"), read, {"workspace": MINIMIZED + workspace})
    problems += _refused(ctx, log, "maximize while minimized", argv("maximize"), read, "window_minimized")
    problems += _refused(ctx, log, "move while minimized", argv("move-window", "--x", "40", "--y", "60"), read,
                         "window_minimized")
    return problems + operated(ctx, "restore", argv("restore"), read, {"workspace": workspace, "fullscreen": 0})


def _supported(ctx: LegContext, target: tuple, read) -> list[str]:
    """The supported counterpart of the refused move works in the same mode, then is undone."""
    window_id, address, workspace, _, _ = target
    control(ctx.tier, lambda lua: float_argv(address, lua))
    require(raw.poll(lambda: read()["floating"] is True, SETTLE_S), f"{address} did not float")
    first = read()
    argv = lambda x, y: window_argv("move-window", window_id, *point(x, y))
    problems = operated(ctx, "floating move", argv(40, 60), read, {"at": [40, 60], "workspace": workspace})
    return problems + operated(ctx, "floating move back", argv(*first["at"]), read,
                               {"at": first["at"], "size": first["size"], "workspace": workspace})


def window_operation_refusals(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read_desktop = lambda: desktop(tier, main)
    holder, pointer = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read_desktop())
    try:
        with parked_sizer(ctx, holder, pointer) as target:
            read = located(tier, target[1])
            started = generation(target[3])
            ctx.before("proc.target", started)
            with Sampler(watched(tier, holder)) as sampler:
                problems = _unsupported(ctx, target, read)
            problems += sampler.changes + _absent(ctx, holder, read, LABEL)
            ended = generation(target[3])
            if ended != started:
                problems.append(f"the sizer's process changed from {started} to {ended}")
            ctx.after("proc.target", ended)
            with Sampler(watched(tier, holder)) as sampler:
                problems += _supported(ctx, target, read)
            problems += sampler.changes + keeps_focus(tier, read_desktop())
            ctx.after("hyprland.desktop", read_desktop())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, pointer)
    ctx.restored("hyprland.desktop", read_desktop())
