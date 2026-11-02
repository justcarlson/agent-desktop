"""Two processes with one window title and one size, each bound to its own toplevel.

Two Zoo GTK3 processes each show one frame titled "Zoo GTK3". Each sits alone
on its own spare workspace, so `j/clients` gives both one size. Title and size
then match both toplevels, and only the process tells them apart. A minimize
and restore by each frame id must move that process's toplevel and leave the
other's as it was, read back from `j/clients`, while the desktop is polled.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_focus_controls import (client, clients_of, cursor, listed_ids, move, restore_desktop,
                                               spare_workspaces, workspaces_in_use)
from scenarios.hyprland_window_checks import MINIMIZED, SETTLE_S, TOLERANCE, operated, placement, unchanged, window_argv

APP = "Zoo GTK3"


def one_title_one_size(toplevels: list[dict]) -> bool:
    """Two toplevels of two processes with one title and one size, within the tolerance."""
    if len(toplevels) != 2 or None in toplevels or len({item.get("pid") for item in toplevels}) != 2:
        return False
    if len({item.get("title") for item in toplevels}) != 1:
        return False
    first, second = (item.get("size") or [] for item in toplevels)
    return len(first) == len(second) == 2 and all(abs(a - b) <= TOLERANCE for a, b in zip(first, second))


def _toplevel(tier, app) -> dict:
    found = clients_of(tier, app.pid)
    require(len(found) == 1, f"{APP} pid {app.pid} has {len(found)} toplevels")
    require(len(app.windows) == 1, f"{APP} pid {app.pid} has frames {app.windows}")
    return found[0]


def _placed(ctx: LegContext, pair, holder: dict, point: dict) -> list[dict]:
    tier = ctx.tier
    toplevels = [_toplevel(tier, app) for app in pair]
    for item, workspace in zip(toplevels, spare_workspaces(workspaces_in_use(tier), 2)):
        move(tier, item["address"], workspace)
    restore_desktop(tier, holder, point)
    require(raw.poll(lambda: one_title_one_size([client(tier, item["address"]) for item in toplevels]), SETTLE_S),
            f"the two processes never showed one title and one size: "
            f"{[client(tier, item['address']) for item in toplevels]}")
    listing = ctx.product(["list-windows"])
    for app in pair:
        ids = listed_ids(listing, app.pid)
        require(ids == app.windows, f"list-windows gives pid {app.pid} {ids}, AT-SPI {app.windows}")
    return [client(ctx.tier, item["address"]) for item in toplevels]


def _own_toplevel(ctx: LegContext, app, own: dict, other: dict) -> list[str]:
    """Minimize and restore by the frame id move this process's toplevel only."""
    tier, window_id = ctx.tier, app.windows[0]
    read_own = lambda: placement(client(tier, own["address"]))
    read_other = lambda: placement(client(tier, other["address"]))
    first, other_before = read_own(), read_other()
    label = f"pid {app.pid}"
    problems = operated(ctx, f"{label} minimize", window_argv("minimize", window_id), read_own,
                        {"workspace": MINIMIZED + first["workspace"]})
    problems += unchanged(f"{label} minimize, the other process", read_other, other_before)
    problems += operated(ctx, f"{label} restore", window_argv("restore", window_id), read_own,
                         {"workspace": first["workspace"], "at": first["at"], "size": first["size"]})
    return problems + unchanged(f"{label} restore, the other process", read_other, other_before)


@implementation("identity.hyprland_two_processes_one_title")
def two_processes_one_title(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with managed(ctx, APP, (str(tier.runtime_dir / "zoo-gtk3-title-a.log"),)) as first, \
                managed(ctx, APP, (str(tier.runtime_dir / "zoo-gtk3-title-b.log"),)) as second:
            require(first.pid != second.pid, "both launches attributed one process")
            toplevels = _placed(ctx, (first, second), holder, point)
            with Sampler({"desktop": read}) as sampler:
                problems = _own_toplevel(ctx, first, toplevels[0], toplevels[1])
                problems += _own_toplevel(ctx, second, toplevels[1], toplevels[0])
            problems += sampler.changes + keeps_focus(tier, read())
            ctx.after("hyprland.desktop", read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())
