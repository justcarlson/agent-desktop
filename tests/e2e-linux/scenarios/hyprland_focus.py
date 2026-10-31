"""`focus-window` on the attached `hyprland` tier.

A headed `focus-window` binds the window to one toplevel and activates it.
Every result is read back from the compositor: `j/activewindow` names the
window that holds focus, `j/clients` names each toplevel's workspace and size,
and `j/monitors` names the workspaces in view. The command's own `ok` is never
the evidence. Each leg gives focus, the cursor and the workspaces back to the
bystander and reads them back. Only the desktop is a recorded oracle; a
window a leg starts is gone by the restore point, so each leg checks its own
windows before and after it acts.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_focus_controls import (active, bound_by_size, client, clients_of, cursor, focus_twins,
                                               frame_sizes, listed_ids, move, restore_desktop, same_size, shown,
                                               spare_workspaces, terminal, workspaces_in_use)

FOCUS_SETTLE_S = 3.0


def focused_view(tier, address: str) -> dict:
    """What the compositor says about one toplevel and focus, from three reads."""
    held = active(tier)
    toplevel = client(tier, address) or {}
    workspace = (toplevel.get("workspace") or {}).get("name")
    return {
        "active": held.get("address"),
        "active_pid": held.get("pid"),
        "pid": toplevel.get("pid"),
        "workspace": workspace,
        "in_view": any(regular == workspace and not special for regular, special in shown(tier).values()),
    }


def focus_problems(label: str, view: dict, address: str, workspace: str) -> list[str]:
    """The toplevel holds focus, is the one active window, stayed on its workspace and is in view."""
    problems = []
    if view.get("active") != address or view.get("active_pid") != view.get("pid"):
        problems.append(f"{label}: the compositor's active window is {view.get('active')}, not {address}")
    if view.get("workspace") != workspace:
        problems.append(f"{label}: the toplevel is on {view.get('workspace')!r}, not {workspace!r}")
    if not view.get("in_view"):
        problems.append(f"{label}: no monitor shows {workspace!r}")
    return problems


def focused(ctx: LegContext, label: str, window_id: str, address: str, workspace: str) -> list[str]:
    """Run a headed focus-window and read the result from the compositor, never from the envelope."""
    envelope = ctx.product(["focus-window", "--window-id", window_id])
    raw.poll(lambda: focus_problems(label, focused_view(ctx.tier, address), address, workspace) == [],
             FOCUS_SETTLE_S)
    view = focused_view(ctx.tier, address)
    problems = focus_problems(label, view, address, workspace)
    if problems:
        problems.append(f"{label}: focus-window answered {envelope.raw[:400]}")
    return problems


def candidate_problems(label: str, envelope, expected: dict[str, str]) -> list[str]:
    """AMBIGUOUS_TARGET names every twin toplevel with its workspace, and nothing was sent."""
    problems = boundary_problems(label, envelope, ("AMBIGUOUS_TARGET",))
    candidates = ((envelope.error or {}).get("details") or {}).get("candidates")
    if not isinstance(candidates, list):
        return problems + [f"{label}: details.candidates is {candidates!r}"]
    named = {str(item.get("address")): item.get("workspace") for item in candidates if isinstance(item, dict)}
    if len(candidates) != len(expected) or named != expected:
        problems.append(f"{label}: candidates {candidates} are not the twin toplevels {expected}")
    return problems


def unfocused_problems(label: str, view: dict, address: str, workspace: str) -> list[str]:
    """Before the leg acts, the toplevel waits unfocused on its workspace and no monitor shows it."""
    problems = []
    if view.get("active") == address:
        problems.append(f"{label}: {address} already holds focus before focus-window")
    if view.get("workspace") != workspace:
        problems.append(f"{label}: the toplevel is on {view.get('workspace')!r}, not {workspace!r}")
    if view.get("in_view"):
        problems.append(f"{label}: {workspace!r} is in view before the leg focuses a window there")
    return problems


def _unfocused(tier, label: str, address: str, workspace: str) -> None:
    problems = unfocused_problems(label, focused_view(tier, address), address, workspace)
    require(not problems, "; ".join(problems))


@implementation("focus.hyprland_other_workspace")
def other_workspace(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", desktop(tier, main))
    try:
        with managed(ctx, "Calculator") as calculator, terminal(tier, "ad-e2e-focus-far") as (foot_app, foot):
            toplevels = clients_of(tier, calculator.pid)
            require(len(toplevels) == 1, f"Calculator has {len(toplevels)} toplevels")
            foot_ids = listed_ids(ctx.product(["list-windows"]), foot_app.pid)
            require(len(foot_ids) == 1, f"list-windows gives the terminal {foot_ids}")
            far = spare_workspaces(workspaces_in_use(tier), 2)
            targets = (("calculator", calculator.windows[0], toplevels[0]["address"], far[0]),
                       ("terminal", foot_ids[0], foot["address"], far[1]))
            for _, _, address, workspace in targets:
                move(tier, address, workspace)
            problems = []
            for label, window_id, address, workspace in targets:
                _unfocused(tier, label, address, workspace)
                problems += focused(ctx, label, window_id, address, workspace)
            ctx.after("hyprland.desktop", desktop(tier, main))
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", desktop(tier, main))


def _set_apart(tier, found: list[dict]) -> dict[str, str]:
    """Each twin alone on its own hidden regular workspace, so both have one title and one size."""
    addresses = sorted(item["address"] for item in found)
    places = dict(zip(addresses, spare_workspaces(workspaces_in_use(tier), 2)))
    for address, workspace in places.items():
        move(tier, address, workspace)
    return places


@implementation("focus.hyprland_twin_after_sizes_differ")
def twin_after_sizes_differ(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", desktop(tier, main))
    try:
        with focus_twins(tier) as (twins, found):
            places = _set_apart(tier, found)
            shared_place = places[sorted(places)[1]]
            with terminal(tier, "ad-e2e-focus-split") as (_, foot):
                move(tier, foot["address"], shared_place)
                bound = raw.poll(lambda: bound_by_size(clients_of(tier, twins.pid), frame_sizes(tier, twins)), 5.0)
                require(bound, f"the twins never took distinct sizes: {clients_of(tier, twins.pid)}")
                problems = []
                for label, window_id in zip(("first twin", "second twin"), twins.windows):
                    address = bound[window_id]
                    _unfocused(tier, label, address, places[address])
                    problems += focused(ctx, label, window_id, address, places[address])
                ctx.after("hyprland.desktop", desktop(tier, main))
                require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", desktop(tier, main))


@implementation("focus.hyprland_same_size_twins_ambiguous")
def same_size_twins_ambiguous(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with focus_twins(tier) as (twins, found):
            places = _set_apart(tier, found)
            restore_desktop(tier, holder, point)
            require(raw.poll(lambda: same_size(clients_of(tier, twins.pid), frame_sizes(tier, twins)), 5.0),
                    f"the twins never shared one size: {clients_of(tier, twins.pid)}")
            before = sorted(clients_of(tier, twins.pid), key=lambda item: item["address"])
            with Sampler({"desktop": read}) as sampler:
                problems = []
                for window_id in twins.windows:
                    envelope = ctx.product(["focus-window", "--window-id", window_id])
                    problems += candidate_problems(f"same-size twin {window_id}", envelope, places)
            after = sorted(clients_of(tier, twins.pid), key=lambda item: item["address"])
            ctx.after("hyprland.desktop", read())
            problems += [] if after == before else [f"the twins changed: {before} became {after}"]
            problems += sampler.changes + keeps_focus(tier, read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _closed(ctx: LegContext, label: str, window_id: str) -> list[str]:
    envelope = ctx.product(["focus-window", "--window-id", window_id])
    return boundary_problems(f"closed {label}", envelope, ("WINDOW_NOT_FOUND",))


@implementation("focus.hyprland_closed_window_not_found")
def closed_window_not_found(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    ctx.before("hyprland.desktop", read())
    with managed(ctx, "Calculator") as calculator:
        frame, pid = calculator.windows[0], calculator.pid
    require(raw.poll(lambda: not clients_of(tier, pid), 5.0), "the closed Calculator still has a toplevel")
    with terminal(tier, "ad-e2e-focus-gone") as (foot_app, _):
        ids = listed_ids(ctx.product(["list-windows"]), foot_app.pid)
    require(len(ids) == 1, f"list-windows gave the terminal {ids}")
    left = {"calculator": clients_of(tier, pid), "terminal": clients_of(tier, foot_app.pid)}
    require(not any(left.values()), f"a closed window still has a toplevel: {left}")
    with Sampler({"desktop": read}) as sampler:
        problems = _closed(ctx, "frame", frame) + _closed(ctx, "toplevel", ids[0])
    ctx.after("hyprland.desktop", read())
    problems += sampler.changes + keeps_focus(tier, read())
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())
