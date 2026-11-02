"""`duplicates.two_processes_distinct_window_ids` on the attached `hyprland` tier.

The X11 route proves that each window id addresses its own process by focusing
it and reading `_NET_WM_PID` of the X11 active window. Neither step exists
here: a headless `focus-window` on Hyprland refuses instead of activating a
window, and a native Wayland Calculator has no X11 window. This route proves
the same promise with independent reads: the accessibility registry gives each
process its own frame ids, `j/clients` maps one toplevel per process, each
snapshot answers for the id it was given, and `list-windows` names each id
with its own pid. The bystander keeps focus throughout.
"""

from __future__ import annotations

from runner import apps
from runner.helper import command, managed, require, snapshot
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_focus_controls import clients_of


def owner_evidence(tier) -> list[tuple]:
    return sorted((item.pid, item.start_time, item.windows) for item in apps.owners_named(tier, "Calculator"))


def toplevel_evidence(tier, pids: tuple[int, ...]) -> list[tuple]:
    return sorted((pid, sorted(item.get("address") for item in clients_of(tier, pid))) for pid in pids)


def listed_pids(ctx: LegContext) -> dict[str, object]:
    listed = command(ctx, ["list-windows", "--app", "Calculator"]).data or []
    return {item.get("id"): item.get("pid") for item in listed}


def two_processes_distinct_window_ids(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    read = lambda: desktop(ctx.tier, main)
    with managed(ctx, "Calculator") as first, managed(ctx, "Calculator") as second:
        pids = (first.pid, second.pid)
        evidence = owner_evidence(ctx.tier)
        require(len(evidence) == 2 and first.pid != second.pid, f"expected two Calculator processes: {evidence}")
        require(set(first.windows).isdisjoint(second.windows), "the two processes share a window id")
        toplevels = toplevel_evidence(ctx.tier, pids)
        require(all(len(addresses) == 1 for _, addresses in toplevels),
                f"j/clients does not map one toplevel per process: {toplevels}")
        require(toplevels[0][1] != toplevels[1][1], f"the processes share a toplevel: {toplevels}")
        state = read()
        require(not keeps_focus(ctx.tier, state), "; ".join(keeps_focus(ctx.tier, state)))
        ctx.before("atspi", evidence)
        ctx.before("hyprland.clients", toplevels)
        ctx.before("hyprland.desktop", state)
        with Sampler({"desktop": read}) as sampler:
            listed = listed_pids(ctx)
            for app in (first, second):
                data = snapshot(ctx, app)
                require(data["window"]["id"] == app.windows[0],
                        f"the snapshot for {app.windows[0]} answered {data['window'].get('id')}")
                require(listed.get(app.windows[0]) == app.pid,
                        f"list-windows names {app.windows[0]} with pid {listed.get(app.windows[0])}, not {app.pid}")
        ctx.after("atspi", owner_evidence(ctx.tier))
        ctx.after("hyprland.clients", toplevel_evidence(ctx.tier, pids))
        ctx.after("hyprland.desktop", read())
        problems = sampler.changes + keeps_focus(ctx.tier, read())
        require(owner_evidence(ctx.tier) == evidence, "the Calculator processes or their window ids changed")
        require(toplevel_evidence(ctx.tier, pids) == toplevels, "the Calculator toplevels changed")
        require(not problems, "the desktop moved: " + "; ".join(problems))
        ctx.restored("atspi", owner_evidence(ctx.tier))
        ctx.restored("hyprland.clients", toplevel_evidence(ctx.tier, pids))
        ctx.restored("hyprland.desktop", read())
