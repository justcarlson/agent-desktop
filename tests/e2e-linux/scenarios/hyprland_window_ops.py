"""Window operations on the attached `hyprland` tier.

Each leg starts its own foot terminal and parks it alone on a spare workspace
that no monitor shows, so a tiled bystander never reflows. Every result is
read back from `j/clients`, never from the command's own `ok`. A sampler reads
the desktop and the bystander's toplevel while each operation runs: focus,
the cursor, the workspaces in view and the bystander must not change.

Each leg then runs the same operations on a sizer window in a nested instance
(`hyprland_window_nested`), where the output can move to a negative origin and a
fractional scale and the app's own log is read too, and once more in an instance
started from a legacy config (`hyprland_window_legacy`).
"""

from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw, hyprland
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance, shared
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_focus_controls import (active, client, control, cursor, focus_argv, listed_ids, move,
                                               restore_desktop, spare_workspaces, terminal, workspaces_in_use)
from scenarios.hyprland_pointer import nested
from scenarios.hyprland_window_checks import (MINIMIZED, SETTLE_S, float_argv, missing, operated, placement,
                                              refusal, satisfied, unchanged, window_argv)
from scenarios.hyprland_window_legacy import legacy_instance
from scenarios.hyprland_window_nested import geometry_cases, maximize_cases, minimize_cases

FOREIGN = "special:ad-e2e-foreign"
NEGATIVE_CORNER = ("-50", "-20")


def uses_lua(tier) -> bool:
    return hyprland.uses_lua(shared(tier), main_instance(tier)["signature"])


def watched(tier, holder: dict) -> dict:
    """What every operation must leave alone, read while it runs."""
    main = main_instance(tier)
    return {"desktop": lambda: desktop(tier, main),
            "bystander": lambda: placement(client(tier, holder["address"]))}


def located(tier, address: str):
    """Reads the toplevel's placement on the session instance."""
    return lambda: placement(client(tier, address))


@contextlib.contextmanager
def parked(ctx: LegContext, app_id: str, holder: dict, point: dict, *, floating: bool):
    """A foot terminal alone on a spare workspace, the desktop given back to the bystander.
    Yields its window id, address and workspace."""
    tier = ctx.tier
    settled = placement(client(tier, holder["address"]))
    with terminal(tier, app_id) as (app, found):
        ids = listed_ids(ctx.product(["list-windows"]), app.pid)
        require(len(ids) == 1, f"list-windows gives the terminal {ids}")
        address = found["address"]
        workspace = spare_workspaces(workspaces_in_use(tier), 1)[0]
        move(tier, address, workspace)
        if floating:
            control(tier, lambda lua: float_argv(address, lua))
            require(raw.poll(lambda: placement(client(tier, address))["floating"] is True, SETTLE_S),
                    f"{address} did not float")
        restore_desktop(tier, holder, point)
        require(raw.poll(lambda: placement(client(tier, holder["address"])) == settled, SETTLE_S),
                f"the bystander did not settle back to {settled}")
        yield ids[0], address, workspace


def _leg(ctx: LegContext, app_id: str, floating: bool, work, nested_work) -> None:
    """Record the desktop around `work`, which returns its problems, and give it back after. Then
    run `nested_work` in the nested Lua instance and in a legacy config instance."""
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with parked(ctx, app_id, holder, point, floating=floating) as target:
            problems = work(ctx, holder, *target)
            ctx.after("hyprland.desktop", read())
        instance, lua = nested(tier)
        problems += nested_work(ctx, instance, lua, "nested")
        with legacy_instance(tier) as (instance, lua):
            problems += nested_work(ctx, instance, lua, "legacy config" if not lua else "conf file read as lua")
        require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def closed_id(ctx: LegContext, app_id: str) -> str:
    """The window id of a terminal that has since closed and whose toplevel is gone."""
    with terminal(ctx.tier, app_id) as (app, _):
        ids = listed_ids(ctx.product(["list-windows"]), app.pid)
    require(len(ids) == 1, f"list-windows gave the closed terminal {ids}")
    return ids[0]


def _absent(ctx: LegContext, holder: dict, read, label: str) -> list[str]:
    """Every operation on a closed window's id is WINDOW_NOT_FOUND and touches neither the parked
    target nor the bystander."""
    gone = closed_id(ctx, f"ad-e2e-window-gone-{label}")
    reads = {"target": read, "bystander": located(ctx.tier, holder["address"])}
    with Sampler(watched(ctx.tier, holder)) as sampler:
        problems = missing(ctx, f"closed {label} window", gone, reads)
    return problems + sampler.changes


def _geometry(ctx: LegContext, holder: dict, window_id: str, address: str, workspace: str) -> list[str]:
    tier, read = ctx.tier, located(ctx.tier, address)
    argv = lambda verb, *extra: window_argv(verb, window_id, *extra)
    with Sampler(watched(tier, holder)) as sampler:
        problems = refusal(ctx, "tiled move", argv("move-window", "--x", "40", "--y", "60"), read, "tiled_window")
        problems += refusal(ctx, "tiled resize", argv("resize-window", "--width", "500", "--height", "400"), read,
                            "tiled_window")
    problems += sampler.changes
    control(tier, lambda lua: float_argv(address, lua))
    require(raw.poll(lambda: read()["floating"] is True, SETTLE_S), f"{address} did not float")
    first = read()
    with Sampler(watched(tier, holder)) as sampler:
        problems += operated(ctx, "move", argv("move-window", "--x", "40", "--y", "60"), read,
                             {"at": [40, 60], "workspace": workspace, "floating": True})
        problems += operated(ctx, "resize", argv("resize-window", "--width", "500", "--height", "400"), read,
                             {"at": [40, 60], "size": [500, 400], "workspace": workspace})
        problems += satisfied(ctx, "move again", argv("move-window", "--x", "40", "--y", "60"), read)
        problems += operated(ctx, "move to a negative corner", argv("move-window", "--x", NEGATIVE_CORNER[0],
                                                                    "--y", NEGATIVE_CORNER[1]), read,
                             {"at": [int(NEGATIVE_CORNER[0]), int(NEGATIVE_CORNER[1])], "size": [500, 400]})
        problems += operated(ctx, "resize back", argv("resize-window", "--width", str(first["size"][0]),
                                                      "--height", str(first["size"][1])), read,
                             {"size": first["size"]})
        problems += operated(ctx, "move back", argv("move-window", "--x", str(first["at"][0]),
                                                    "--y", str(first["at"][1])), read,
                             {"at": first["at"], "size": first["size"], "workspace": workspace})
    problems += sampler.changes + _absent(ctx, holder, read, "geometry")
    return problems + keeps_focus(tier, desktop(tier, main_instance(tier)))


def _foreign(ctx: LegContext, read, window_id: str, address: str, workspace: str) -> list[str]:
    """A window another tool hid on a special workspace is left to that tool."""
    move(ctx.tier, address, FOREIGN)
    try:
        return [problem for verb in ("minimize", "maximize", "restore")
                for problem in refusal(ctx, f"{verb} of a window another tool hid",
                                       window_argv(verb, window_id), read, "foreign_special_workspace")]
    finally:
        move(ctx.tier, address, workspace)


def _moved_away(ctx: LegContext, read, window_id: str, address: str, workspace: str) -> list[str]:
    """A minimized window someone moved elsewhere is no longer ours to restore: restore is
    satisfied and leaves it where it is."""
    problems = operated(ctx, "minimize before a move away", window_argv("minimize", window_id), read,
                        {"workspace": MINIMIZED + workspace})
    elsewhere = spare_workspaces(workspaces_in_use(ctx.tier), 1)[0]
    move(ctx.tier, address, elsewhere)
    try:
        problems += satisfied(ctx, "restore after a move away", window_argv("restore", window_id), read)
    finally:
        move(ctx.tier, address, workspace)
    return problems


def _restarted(ctx: LegContext, holder: dict) -> list[str]:
    """After a minimized app restarts, restore by the old id is WINDOW_NOT_FOUND and the new
    window is untouched."""
    tier, app_id = ctx.tier, "ad-e2e-window-restart"
    with terminal(tier, app_id) as (app, found):
        old = listed_ids(ctx.product(["list-windows"]), app.pid)
        require(len(old) == 1, f"list-windows gives the first terminal {old}")
        read = located(tier, found["address"])
        workspace = spare_workspaces(workspaces_in_use(tier), 1)[0]
        move(tier, found["address"], workspace)
        problems = operated(ctx, "minimize before a restart", window_argv("minimize", old[0]), read,
                            {"workspace": MINIMIZED + workspace})
    with terminal(tier, app_id) as (app, found):
        new = listed_ids(ctx.product(["list-windows"]), app.pid)
        if len(new) != 1 or new[0] == old[0]:
            problems.append(f"the restarted terminal is listed as {new}, the old id was {old[0]}")
        reads = {"restarted window": located(tier, found["address"]),
                 "bystander": located(tier, holder["address"])}
        with Sampler(watched(tier, holder)) as sampler:
            before = {name: each() for name, each in reads.items()}
            envelope = ctx.product(window_argv("restore", old[0]))
            problems += boundary_problems("restore by the id of the closed window", envelope, ("WINDOW_NOT_FOUND",))
            problems += [problem for name, each in reads.items()
                         for problem in unchanged(f"restore by the old id, {name}", each, before[name])]
        return problems + sampler.changes


def _minimize(ctx: LegContext, holder: dict, window_id: str, address: str, workspace: str) -> list[str]:
    tier, read = ctx.tier, located(ctx.tier, address)
    before = read()
    with Sampler(watched(tier, holder)) as sampler:
        problems = operated(ctx, "minimize", window_argv("minimize", window_id), read,
                            {"workspace": MINIMIZED + workspace})
        if workspace in workspaces_in_use(tier):
            problems.append(f"{workspace} still exists with its only window minimized")
        problems += satisfied(ctx, "minimize again", window_argv("minimize", window_id), read)
        problems += refusal(ctx, "move while minimized",
                            window_argv("move-window", window_id, "--x", "40", "--y", "60"), read,
                            "window_minimized")
        problems += operated(ctx, "restore to a workspace that no longer exists", window_argv("restore", window_id),
                             read, {"workspace": workspace, "fullscreen": 0, "size": before["size"]})
        problems += _foreign(ctx, read, window_id, address, workspace)
        problems += _moved_away(ctx, read, window_id, address, workspace)
    problems += sampler.changes + _restarted(ctx, holder)
    return problems + keeps_focus(tier, desktop(tier, main_instance(tier)))


def _maximize(ctx: LegContext, holder: dict, window_id: str, address: str, workspace: str) -> list[str]:
    tier, read = ctx.tier, located(ctx.tier, address)
    floating = read()
    if not uses_lua(tier):
        with Sampler(watched(tier, holder)) as sampler:
            problems = refusal(ctx, "legacy maximize", window_argv("maximize", window_id), read,
                               "legacy_config_focused_only")
        return problems + sampler.changes
    with Sampler(watched(tier, holder)) as sampler:
        problems = refusal(ctx, "maximize while the bystander holds focus", window_argv("maximize", window_id),
                           read, "maximize_takes_focus")
    problems += sampler.changes + _absent(ctx, holder, read, "maximize")
    control(tier, lambda lua: focus_argv(address, lua))
    require(raw.poll(lambda: active(tier).get("address") == address, SETTLE_S), f"{address} did not take focus")
    with Sampler(watched(tier, holder)) as sampler:
        problems += operated(ctx, "maximize", window_argv("maximize", window_id), read,
                             {"fullscreen": 1, "workspace": workspace})
        problems += satisfied(ctx, "maximize again", window_argv("maximize", window_id), read)
        problems += refusal(ctx, "resize while maximized",
                            window_argv("resize-window", window_id, "--width", "500", "--height", "400"),
                            read, "window_maximized")
        problems += operated(ctx, "restore", window_argv("restore", window_id), read,
                             {"fullscreen": 0, "at": floating["at"], "size": floating["size"],
                              "workspace": workspace})
    held = active(tier).get("address")
    return problems + sampler.changes + ([] if held == address else [f"focus moved from {address} to {held}"])


@implementation("lifecycle.hyprland_floating_move_resize")
def floating_move_resize(ctx: LegContext) -> None:
    _leg(ctx, "ad-e2e-window-geometry", False, _geometry, geometry_cases)


@implementation("lifecycle.hyprland_minimize_restore")
def minimize_restore(ctx: LegContext) -> None:
    _leg(ctx, "ad-e2e-window-minimize", False, _minimize, minimize_cases)


@implementation("lifecycle.hyprland_maximize_restore")
def maximize_restore(ctx: LegContext) -> None:
    _leg(ctx, "ad-e2e-window-maximize", True, _maximize, maximize_cases)
