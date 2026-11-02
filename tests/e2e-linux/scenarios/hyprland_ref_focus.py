"""Ref focus on the attached `hyprland` tier.

Ref focus is the accessibility GrabFocus action, not a pointer action. Headed,
the product first proves the Qt window holds focus, which needs no activation
here because the leg gave it focus, and then moves keyboard focus inside it.
The raw FOCUSED states of the entry and the button show the move, and
`j/activewindow` shows the Qt toplevel still active. Headless, the product
refuses with POLICY_DENIED and nothing changes: once while the Qt window waits
on its hidden special workspace, which stays hidden, and once while it holds
focus. Focus, cursor and workspaces go back to the bystander.

The Qt window runs on native Wayland: the attached session gives apps no X
display, so the X11 tiers' xcb launch never maps here.
"""

from __future__ import annotations

from pathlib import Path

from runner.helper import changed, one, ref, require, root
from scenarios.headed_actions import assert_ok
from scenarios.hyprland_common import Sampler, desktop, main_instance
from scenarios.hyprland_focus_controls import active, move, restore_desktop, shown
from scenarios.hyprland_focus_proof import _bystander_desktop, frame_active, spawned
from scenarios.hyprland_keys_common import give_focus, toplevel
from scenarios.ref_effects import policy_refusal
from scenarios.ref_focus import grab


QT_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "qt_window.py"
QT_NAMES = ("qt_window.py",)
QT_ENV = {"QT_QPA_PLATFORM": "wayland", "QT_ACCESSIBILITY": "1", "QT_LINUX_ACCESSIBILITY_ALWAYS_ON": "1"}


def qt_window(tier):
    """The Qt toolkit window on native Wayland, registered to the run."""
    log = tier.runtime_dir / "qt-window.log"
    log.unlink(missing_ok=True)
    return spawned(tier, [str(QT_SCRIPT), str(log)], QT_NAMES, "ad-qt-focus", QT_ENV)


def _hidden_refusal(ctx, target: str, read_focus, workspace: str) -> list[str]:
    tier = ctx.tier
    read = lambda: desktop(tier, main_instance(tier))
    with Sampler({"desktop": read, "shown": lambda: shown(tier)}) as sampler:
        policy_refusal(ctx, ["focus", target], read_focus, read_focus())
    problems = [f"hidden: {change}" for change in sampler.changes]
    if any(special == workspace for _, special in shown(tier).values()):
        problems.append(f"the hidden {workspace} was shown")
    return problems


def focus(ctx) -> None:
    tier = ctx.tier
    with _bystander_desktop(ctx) as (read, holder, point), qt_window(tier) as app:
        tree = root(ctx, app)
        target = ref(ctx, app, name="Text entry", role="textfield")
        field = one(tree, name="Text entry")
        other = one(tree, role=43, name="Press me")
        read_focus = lambda: ("FOCUSED" in field.states, "FOCUSED" in other.states)
        client = toplevel(tier, app)
        problems = []
        hidden = (client.get("workspace") or {}).get("name") or ""
        if ctx.mode == "headless" and hidden.startswith("special:"):
            problems += _hidden_refusal(ctx, target, read_focus, hidden)
        move(tier, client["address"], read()["workspace"])
        restore_desktop(tier, holder, point)
        give_focus(tier, client["address"])
        grab(other)
        before = changed(read_focus, (False, True))
        ctx.before("atspi.focus", before)
        try:
            if ctx.mode == "headless":
                with Sampler({"desktop": read}) as sampler:
                    policy_refusal(ctx, ["focus", target], read_focus, before)
                problems += [f"focused: {change}" for change in sampler.changes]
            else:
                assert_ok(ctx.product(["focus", target]))
                changed(read_focus, (True, False))
                if active(tier).get("address") != client["address"]:
                    problems.append(f"focus moved to another frame: j/activewindow is {active(tier)}")
                if not frame_active(tier, app):
                    problems.append("the Qt frame lost ACTIVE")
            ctx.after("atspi.focus", read_focus())
        finally:
            grab(other)
            ctx.restored("atspi.focus", changed(read_focus, before))
        require(not problems, "; ".join(problems))
