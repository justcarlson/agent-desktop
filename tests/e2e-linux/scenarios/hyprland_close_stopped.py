"""`close-app` on a stopped app on the attached `hyprland` tier.

A stopped process cannot answer a close request, so close-app must report
`not_delivered` and leave it running. `/proc` shows the process generation
still alive, `j/clients` still lists its toplevel, and the bystander keeps
focus, cursor and workspaces throughout. The process resumes before cleanup.
"""

from __future__ import annotations

import os
import signal

from runner.helper import managed, require
from runner.verdict import LegContext
from scenarios.hyprland_close import clients_of, exited, listed_name
from scenarios.hyprland_common import Sampler, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus


def stopped_close_problems(envelope, alive: bool, toplevels: tuple[list, list]) -> list[str]:
    """The report says not_delivered, the process lives, and its toplevels stay."""
    report = (envelope.data if envelope.ok else envelope.error) or {}
    delivery = (report.get("disposition") or {}).get("delivery")
    problems = []
    if delivery != "not_delivered":
        problems.append(f"close-app on a stopped app reports {delivery!r}: {envelope.raw[:400]}")
    if not alive:
        problems.append("the stopped process exited")
    before, after = toplevels
    if not after or before != after:
        problems.append(f"the stopped app's toplevels changed from {before} to {after}")
    return problems


def close_app_undelivered_report(ctx: LegContext) -> None:
    tier = ctx.tier
    read = lambda: desktop(tier, main_instance(tier))
    ctx.before("hyprland.desktop", read())
    with managed(ctx, "Calculator") as app:
        name = listed_name(ctx.product(["list-apps"]), app.pid)
        alive = lambda: not exited(app.pid, app.start_time)
        listed = lambda: sorted(item.get("address") for item in clients_of(tier, app.pid))
        os.kill(app.pid, signal.SIGSTOP)
        try:
            ctx.before("proc.alive", alive())
            before = listed()
            with Sampler({"desktop": read}) as sampler:
                envelope = ctx.product(["close-app", name])
            ctx.after("proc.alive", alive())
            ctx.after("hyprland.desktop", read())
            problems = stopped_close_problems(envelope, alive(), (before, listed()))
            problems += sampler.changes + keeps_focus(tier, read())
        finally:
            os.kill(app.pid, signal.SIGCONT)
        ctx.restored("proc.alive", alive())
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())
