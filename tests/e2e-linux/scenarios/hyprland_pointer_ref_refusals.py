"""Ref drag and headless hover by ref on the attached `hyprland` tier, on the nested second instance.

A drag with a ref at either end has no receiver proof on Hyprland, so a headed one refuses before
any event: `receiver_unproven` on an XWayland pad, `native_wayland_client` on a native one. Without
headed permission a ref drag or a ref hover refuses with `POLICY_DENIED` before it reads a receiver.
Each refusal is judged by the pad's own event log, `j/cursorpos` and `j/activewindow`; motion counts
as an event.
"""

from __future__ import annotations

from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_pointer import nested, refusal_problems
from scenarios.hyprland_pointer_pad import (NATIVE, PAD_SCROLL, XWAYLAND, cursor_at, events, global_point,
                                            instance_env, pad_ref, pads_left, pointer_pad, settled, since, xy)
from scenarios.hyprland_pointer_refs import (action_label, active_address, native_refusal_problems,
                                             unproven_problems)

QUIET_S = 0.5
BACKENDS = (XWAYLAND, NATIVE)
RECEIVER_JUDGES = {XWAYLAND: unproven_problems, NATIVE: native_refusal_problems}


def drag_argvs(button: str, pane: str, client: dict) -> list[list[str]]:
    """A ref drag from the button to the scroll pane, from the button to a point, and from a point to the button."""
    corner = xy(global_point(client, (0.5, 0.9)))
    return [["drag", "--from", button, "--to", pane], ["drag", "--from", button, "--to-xy", corner],
            ["drag", "--from-xy", corner, "--to", button]]


def hover_argvs(button: str, pane: str, client: dict) -> list[list[str]]:
    return [["hover", button]]


def refused_quietly(ctx: LegContext, env: dict, instance: dict, label: str, argv: list[str], judge, log) -> list[str]:
    """`judge` accepts the refusal, the pad logs nothing, and the cursor and active window stay put."""
    start, cursor, active = len(events(log)), cursor_at(ctx.tier, instance), active_address(ctx.tier, instance)
    problems = judge(label, ctx.product(argv, env=env))
    found = since(settled(log, lambda all_: len(all_) > start, QUIET_S), start)
    if found:
        problems.append(f"{label}: the pad logged {found[:5]}")
    moved = cursor_at(ctx.tier, instance)
    if moved != cursor:
        problems.append(f"{label}: the cursor moved from {cursor} to {moved}")
    now = active_address(ctx.tier, instance)
    if now != active:
        problems.append(f"{label}: the active window changed from {active} to {now}")
    return problems


def _refused_on_both_pads(ctx: LegContext, label: str, argvs_of, judge_of) -> None:
    tier = ctx.tier
    instance, _ = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    ctx.before("hyprland.cursor", cursor_at(tier, instance))
    problems, logged = [], {}
    for backend in BACKENDS:
        with pointer_pad(tier, instance, f"{label}-{backend}", backend) as (owned, client, log):
            button, pane = pad_ref(ctx, env, owned), pad_ref(ctx, env, owned, PAD_SCROLL)
            for argv in argvs_of(button, pane, client):
                name = f"{backend} pad: {ctx.mode} {action_label(argv)}"
                problems += refused_quietly(ctx, env, instance, name, argv, judge_of(backend), log)
            logged[backend] = len(events(log))
    ctx.after("applog.pad", logged)
    ctx.after("hyprland.cursor", cursor_at(tier, instance))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.cursor", cursor_at(tier, instance))
    ctx.restored("applog.pad", pads_left(tier))


def ref_drag_refused(ctx: LegContext) -> None:
    """Every headed ref drag refuses for its receiver before any event, on an XWayland and a native pad."""
    _refused_on_both_pads(ctx, "ref-drag", drag_argvs, RECEIVER_JUDGES.get)


def ref_drag_headless_refused(ctx: LegContext) -> None:
    """Every ref drag without headed permission refuses with POLICY_DENIED before any event."""
    _refused_on_both_pads(ctx, "ref-drag-headless", drag_argvs, lambda backend: refusal_problems)


def hover_by_ref_headless_refused(ctx: LegContext) -> None:
    """A ref hover without headed permission refuses with POLICY_DENIED before any event."""
    _refused_on_both_pads(ctx, "hover-headless", hover_argvs, lambda backend: refusal_problems)
