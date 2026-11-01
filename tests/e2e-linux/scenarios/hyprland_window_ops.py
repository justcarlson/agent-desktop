"""Window operations on the attached `hyprland` tier.

Each leg starts its own foot terminal and parks it alone on a spare workspace
that no monitor shows, so a tiled bystander never reflows. Every result is
read back from `j/clients`, never from the command's own `ok`. A sampler reads
the desktop and the bystander's toplevel while each operation runs: focus,
the cursor, the workspaces in view and the bystander must not change.
"""

from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw, hyprland
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance, shared
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_focus_controls import (active, client, control, cursor, focus_argv, listed_ids, lua_call,
                                               move, restore_desktop, spare_workspaces, terminal,
                                               workspaces_in_use)

MINIMIZED = "special:agent-desktop-minimized-"
SETTLE_S = 3.0
TOLERANCE = 2
VERIFIED = "delivered_verified"


def placement(item: dict | None) -> dict:
    """The fields of a toplevel that a window operation may change."""
    item = item or {}
    return {"at": list(item.get("at") or ()), "size": list(item.get("size") or ()),
            "floating": item.get("floating"), "fullscreen": item.get("fullscreen"),
            "workspace": (item.get("workspace") or {}).get("name")}


def holds(now: dict, expected: dict) -> bool:
    """Every expected field is as asked; a position or size within the tolerance."""
    for key, value in expected.items():
        if key in ("at", "size"):
            seen = now.get(key) or []
            if len(seen) != len(value) or any(abs(a - b) > TOLERANCE for a, b in zip(seen, value)):
                return False
        elif now.get(key) != value:
            return False
    return True


def float_argv(address: str, lua: bool) -> list[str]:
    target = f"address:{address}"
    if lua:
        return ["dispatch", lua_call("hl.dsp.window.float", {"action": "enable", "window": target})]
    hyprland.lua_string(target)
    return ["dispatch", "setfloating", target]


def uses_lua(tier) -> bool:
    return hyprland.uses_lua(shared(tier), main_instance(tier)["signature"])


def watched(tier, holder: dict) -> dict:
    """What every operation must leave alone, read while it runs."""
    main = main_instance(tier)
    return {"desktop": lambda: desktop(tier, main),
            "bystander": lambda: placement(client(tier, holder["address"]))}


def operated(ctx: LegContext, label: str, argv: list[str], address: str, expected: dict) -> list[str]:
    """Run one operation, then read the toplevel back from j/clients."""
    envelope = ctx.product(argv)
    raw.poll(lambda: holds(placement(client(ctx.tier, address)), expected), SETTLE_S)
    now = placement(client(ctx.tier, address))
    problems = [] if holds(now, expected) else [f"{label}: j/clients shows {now}, not {expected}"]
    delivery = ((envelope.data or {}).get("disposition") or {}).get("delivery")
    if not envelope.ok or delivery != VERIFIED:
        problems.append(f"{label}: expected {VERIFIED}, got {envelope.raw[:400]}")
    return problems


def satisfied(ctx: LegContext, label: str, argv: list[str], address: str) -> list[str]:
    """An operation already satisfied answers not_delivered and changes nothing."""
    before = placement(client(ctx.tier, address))
    envelope = ctx.product(argv)
    delivery = ((envelope.data or {}).get("disposition") or {}).get("delivery")
    problems = [] if envelope.ok and delivery == "not_delivered" else [
        f"{label}: expected ok and not_delivered, got {envelope.raw[:400]}"]
    return problems + unchanged(ctx.tier, label, address, before)


def refusal(ctx: LegContext, label: str, argv: list[str], address: str, cause: str) -> list[str]:
    """A refused operation sends nothing, so the target is as it was."""
    before = placement(client(ctx.tier, address))
    envelope = ctx.product(argv)
    problems = boundary_problems(label, envelope, ("ACTION_NOT_SUPPORTED",), physical=True, cause=cause)
    return problems + unchanged(ctx.tier, label, address, before)


def unchanged(tier, label: str, address: str, before: dict) -> list[str]:
    after = placement(client(tier, address))
    return [] if after == before else [f"{label}: the target changed from {before} to {after}"]


def window_argv(verb: str, window_id: str, *extra: str) -> list[str]:
    return [verb, "--window-id", window_id, *extra]


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


def _leg(ctx: LegContext, app_id: str, floating: bool, work) -> None:
    """Record the desktop around `work`, which returns its problems, and give it back after."""
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with parked(ctx, app_id, holder, point, floating=floating) as target:
            problems = work(ctx, holder, *target)
            ctx.after("hyprland.desktop", read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _geometry(ctx: LegContext, holder: dict, window_id: str, address: str, workspace: str) -> list[str]:
    tier = ctx.tier
    with Sampler(watched(tier, holder)) as sampler:
        problems = refusal(ctx, "tiled move", window_argv("move-window", window_id, "--x", "40", "--y", "60"),
                           address, "tiled_window")
    problems += sampler.changes
    control(tier, lambda lua: float_argv(address, lua))
    require(raw.poll(lambda: placement(client(tier, address))["floating"] is True, SETTLE_S),
            f"{address} did not float")
    with Sampler(watched(tier, holder)) as sampler:
        problems += operated(ctx, "move", window_argv("move-window", window_id, "--x", "40", "--y", "60"),
                             address, {"at": [40, 60], "workspace": workspace, "floating": True})
        problems += operated(ctx, "resize",
                             window_argv("resize-window", window_id, "--width", "500", "--height", "400"),
                             address, {"at": [40, 60], "size": [500, 400], "workspace": workspace})
        problems += satisfied(ctx, "move again", window_argv("move-window", window_id, "--x", "40", "--y", "60"),
                              address)
    return problems + sampler.changes + keeps_focus(tier, desktop(tier, main_instance(tier)))


def _minimize(ctx: LegContext, holder: dict, window_id: str, address: str, workspace: str) -> list[str]:
    tier = ctx.tier
    before = placement(client(tier, address))
    with Sampler(watched(tier, holder)) as sampler:
        problems = operated(ctx, "minimize", window_argv("minimize", window_id), address,
                            {"workspace": MINIMIZED + workspace})
        problems += satisfied(ctx, "minimize again", window_argv("minimize", window_id), address)
        problems += refusal(ctx, "move while minimized",
                            window_argv("move-window", window_id, "--x", "40", "--y", "60"), address,
                            "window_minimized")
        problems += operated(ctx, "restore", window_argv("restore", window_id), address,
                             {"workspace": workspace, "fullscreen": 0, "size": before["size"]})
    return problems + sampler.changes + keeps_focus(tier, desktop(tier, main_instance(tier)))


def _maximize(ctx: LegContext, holder: dict, window_id: str, address: str, workspace: str) -> list[str]:
    tier = ctx.tier
    floating = placement(client(tier, address))
    if not uses_lua(tier):
        with Sampler(watched(tier, holder)) as sampler:
            problems = refusal(ctx, "legacy maximize", window_argv("maximize", window_id), address,
                               "legacy_config_focused_only")
        return problems + sampler.changes
    with Sampler(watched(tier, holder)) as sampler:
        problems = refusal(ctx, "maximize while the bystander holds focus", window_argv("maximize", window_id),
                           address, "maximize_takes_focus")
    problems += sampler.changes
    control(tier, lambda lua: focus_argv(address, lua))
    require(raw.poll(lambda: active(tier).get("address") == address, SETTLE_S), f"{address} did not take focus")
    with Sampler(watched(tier, holder)) as sampler:
        problems += operated(ctx, "maximize", window_argv("maximize", window_id), address,
                             {"fullscreen": 1, "workspace": workspace})
        problems += satisfied(ctx, "maximize again", window_argv("maximize", window_id), address)
        problems += refusal(ctx, "resize while maximized",
                            window_argv("resize-window", window_id, "--width", "500", "--height", "400"),
                            address, "window_maximized")
        problems += operated(ctx, "restore", window_argv("restore", window_id), address,
                             {"fullscreen": 0, "at": floating["at"], "size": floating["size"],
                              "workspace": workspace})
    held = active(tier).get("address")
    return problems + sampler.changes + ([] if held == address else [f"focus moved from {address} to {held}"])


@implementation("lifecycle.hyprland_floating_move_resize")
def floating_move_resize(ctx: LegContext) -> None:
    _leg(ctx, "ad-e2e-window-geometry", False, _geometry)


@implementation("lifecycle.hyprland_minimize_restore")
def minimize_restore(ctx: LegContext) -> None:
    _leg(ctx, "ad-e2e-window-minimize", False, _minimize)


@implementation("lifecycle.hyprland_maximize_restore")
def maximize_restore(ctx: LegContext) -> None:
    _leg(ctx, "ad-e2e-window-maximize", True, _maximize)
