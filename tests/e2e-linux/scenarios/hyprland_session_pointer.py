"""Ref pointer refusals and `hover --xy` delivery on the session instance of the attached `hyprland` tier.

Both legs drive one pointer pad the leg owns, shown on the focused monitor beside the bystander. A
sampler reads the bystander's `j/clients` box and keeps the cursor on the pad while the leg acts.

`pointer.session_ref_pointer_refusals` sends every ref pointer verb to an XWayland pad. Headed, each
ref drag, scroll, double-click and triple-click refuses as `receiver_unproven` (L45-2) and a hover by
ref as `hover_unsupported`. Headless, each ref drag and a hover by ref refuse with `POLICY_DENIED` and
no `details.cause`; `pointer.headless_or_cancelled_input` covers headless ref scroll, double-click
and triple-click on the session. Every refusal is not_delivered with safe retry, and after each one
the pad log, `j/cursorpos` and `j/activewindow` are unchanged.

`pointer.session_hover_coordinates` hovers a native pad by coordinates. `j/cursorpos` reads the point,
and the pad's own log shows the motion arrive there with no press, release or scroll.
"""

from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw
from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance
from scenarios.hyprland_cursor import app_shown
from scenarios.hyprland_hover import _cursor_on, ref_refusal_problems, xy_hover_problems
from scenarios.hyprland_pointer import (beside_bystander, held_still, refusal_problems, session_client, session_pad,
                                        session_reads)
from scenarios.hyprland_pointer_pad import (PAD_SCROLL, XWAYLAND, cursor_at, events, global_point, instance_env,
                                            lands_at, pad_ref, pointer_pad, settled, since, window_point, xy)
from scenarios.hyprland_pointer_ref_refusals import drag_argvs, refused_quietly
from scenarios.hyprland_pointer_refs import action_label, unproven_problems

REFUSED = {"delivery": "not_delivered", "retry": "safe"}
HOVER_AIM = (0.5, 0.75)
CURSOR_SETTLE_S = 3.0
DELIVERED = ("press", "release", "scroll")


def disposition_problems(label: str, envelope) -> list[str]:
    """A refusal reports exactly not_delivered with safe retry."""
    found = (envelope.error or {}).get("disposition")
    return [] if found == REFUSED else [f"{label}: the disposition is {found}, not {REFUSED}"]


def headless_problems(label: str, envelope) -> list[str]:
    """A headless ref pointer action refuses with POLICY_DENIED before delivery, and names no cause."""
    problems = refusal_problems(label, envelope)
    cause = ((envelope.error or {}).get("details") or {}).get("cause")
    if cause is not None:
        problems.append(f"{label}: the refusal names cause {cause!r}, expected none")
    return problems


def exact(judge):
    """`judge`, plus the exact disposition."""
    return lambda label, envelope: judge(label, envelope) + disposition_problems(label, envelope)


def headed_actions(button: str, pane: str, client: dict) -> list[tuple[list[str], object]]:
    """Every headed ref pointer verb on the pad, with the judge of its refusal."""
    gestures = [["scroll", pane, "--direction", "down", "--amount", "1"], ["double-click", button],
                ["triple-click", button]]
    return ([(argv, unproven_problems) for argv in drag_argvs(button, pane, client) + gestures]
            + [(["hover", button], ref_refusal_problems)])


def headless_actions(button: str, pane: str, client: dict) -> list[tuple[list[str], object]]:
    """The headless ref drags and hover by ref; `pointer.headless_or_cancelled_input` sends the rest."""
    argvs = drag_argvs(button, pane, client) + [["hover", button]]
    return [(argv, headless_problems) for argv in argvs]


@contextlib.contextmanager
def xwayland_session_pad(ctx: LegContext, label: str):
    """An XWayland pad on the session instance, shown on the focused monitor beside the bystander.
    Yields the app, its settled toplevel and its event log."""
    tier = ctx.tier
    main = main_instance(tier)
    with pointer_pad(tier, main, label, XWAYLAND) as (owned, _, log), app_shown(tier, owned.pid):
        yield owned, session_client(tier, main, owned.pid), log


def session_ref_pointer_refusals(ctx: LegContext) -> None:
    tier = ctx.tier
    main = main_instance(tier)
    session_reads(ctx, "before", main)
    with xwayland_session_pad(ctx, f"session-refs-{ctx.mode}") as (owned, client, log):
        env = instance_env(tier, main)
        button, pane = pad_ref(ctx, env, owned), pad_ref(ctx, env, owned, PAD_SCROLL)
        actions = (headed_actions if ctx.mode == "headed" else headless_actions)(button, pane, client)
        with beside_bystander(tier, main, client) as found:
            held_still(log, len(events(log)), len)
            problems = []
            for argv, judge in actions:
                name = f"session pad: {ctx.mode} {action_label(argv)}"
                problems += refused_quietly(ctx, env, main, name, argv, exact(judge), log)
        session_reads(ctx, "after", main, log)
        require(not problems and not found, "; ".join(problems + found))
    session_reads(ctx, "restored", main)


def hovered_at(found: list[dict], expected: tuple[float, float]) -> bool:
    motions = [item for item in found if item.get("ev") == "motion" and item.get("widget") == "area"]
    return bool(motions) and lands_at(motions[-1], expected)


def pad_hover_problems(found: list[dict], client: dict, aim: tuple[int, int]) -> list[str]:
    """The pad logged motion in its area that ends on the aim, and no press, release or scroll."""
    expected = window_point(aim, client)
    problems = []
    if not hovered_at(found, expected):
        motions = [(item.get("wx"), item.get("wy")) for item in found if item.get("ev") == "motion"]
        problems.append(f"hover --xy={xy(aim)}: the pad's last motions are {motions[-3:]}, not {expected}")
    clicked = [(item.get("ev"), item.get("widget")) for item in found if item.get("ev") in DELIVERED]
    if clicked:
        problems.append(f"hover --xy={xy(aim)}: the pad logged {clicked}")
    return problems


def session_hover_coordinates(ctx: LegContext) -> None:
    tier = ctx.tier
    main = main_instance(tier)
    session_reads(ctx, "before", main)
    with session_pad(ctx, "session-hover") as (_, client, log):
        env = instance_env(tier, main)
        aim = global_point(client, HOVER_AIM)
        expected = window_point(aim, client)
        with beside_bystander(tier, main, client) as found:
            start = len(events(log))
            start += len(held_still(log, start, len))
            envelope = ctx.product(["hover", f"--xy={xy(aim)}"], env=env)
            cursor = raw.poll(lambda: _cursor_on(tier, main, aim), CURSOR_SETTLE_S) or cursor_at(tier, main)
            settled(log, lambda all_: hovered_at(since(all_, start), expected))
            seen = held_still(log, start, len)
            problems = xy_hover_problems(envelope, cursor, aim) + pad_hover_problems(seen, client, aim)
        session_reads(ctx, "after", main, log)
        require(not problems and not found, "; ".join(problems + found))
    session_reads(ctx, "restored", main)
