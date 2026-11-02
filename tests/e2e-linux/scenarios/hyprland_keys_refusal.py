"""Headless physical keys refused before delivery on the attached `hyprland` tier.

The key log app sits unfocused, first on the shown workspace and then on a
spare one, while the bystander holds focus. A headless scoped `press`, by
window id or by app name, must refuse with the documented code and check,
`not_delivered`, before any key: the app logs no key, the entry stays empty,
and focus, cursor and workspaces stay with the bystander. Headless `type`
then writes the entry through `EditableText` with no key event and no focus
change, the negative control the plan row asks for.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import bystander, desktop, main_instance
from scenarios.hyprland_focus_controls import cursor, move, restore_desktop, spare_workspaces, workspaces_in_use
from scenarios.hyprland_keys_common import entry_ref, entry_text, key_log, keyboards, presses, toplevel

SEMANTIC = "semantic"
WINDOW_REFUSAL = ("STALE_REF", "exact_window_focus")
APP_REFUSAL = ("ACTION_FAILED", "window_active")


def press_refusal_problems(label: str, envelope, code: str, check: str) -> list[str]:
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or envelope.exit_code != 1 or error.get("code") != code:
        problems.append(f"{label} answered exit {envelope.exit_code}: {envelope.raw[:300]}")
    if details.get("check") != check:
        problems.append(f"{label} names check {details.get('check')!r}, not {check}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label} is not not_delivered: {error.get('disposition')}")
    if details.get("physical_delivery_started") not in (None, False):
        problems.append(f"{label} started physical delivery")
    return problems


def _refusals(ctx: LegContext, app, where: str) -> list[str]:
    tier = ctx.tier
    seen = desktop(tier, main_instance(tier))
    problems = []
    for label, argv, (code, check) in (
            (f"press --window-id ({where})", ["press", "x", "--window-id", app.windows[0]], WINDOW_REFUSAL),
            (f"press --app ({where})", ["press", "x", "--app", app.name], APP_REFUSAL)):
        problems += press_refusal_problems(label, ctx.product(argv), code, check)
        now = desktop(tier, main_instance(tier))
        if now != seen:
            problems.append(f"{label} changed the desktop: {seen} to {now}")
    return problems


@implementation("input.hyprland_headless_physical_keys_refused")
def headless_physical_keys_refused(ctx: LegContext) -> None:
    tier = ctx.tier
    read = lambda: desktop(tier, main_instance(tier))
    holder, point = bystander(tier), cursor(tier)
    shown = read()["workspace"]
    before = keyboards(tier)
    ctx.before("hyprland.desktop", read())
    ctx.before("hyprland.keyboards", before)
    try:
        with key_log(tier, "refusal") as (app, log):
            address = toplevel(tier, app)["address"]
            move(tier, address, shown)
            restore_desktop(tier, holder, point)
            ref = entry_ref(ctx, app)
            ctx.before("applog.presses", len(presses(log)))
            problems = _refusals(ctx, app, "shown")
            spare = spare_workspaces(workspaces_in_use(tier), 1)[0]
            move(tier, address, spare)
            problems += _refusals(ctx, app, f"hidden on {spare}")
            move(tier, address, shown)
            restore_desktop(tier, holder, point)
            if presses(log):
                problems.append(f"the refused presses reached the app: {presses(log)}")
            if entry_text(tier, app):
                problems.append(f"the refused presses changed the entry to {entry_text(tier, app)!r}")
            seen = read()
            typed = ctx.product(["type", ref, SEMANTIC])
            if not typed.ok:
                problems.append(f"headless type failed: {typed.raw[:300]}")
            if not raw.poll(lambda: entry_text(tier, app) == SEMANTIC, 4.0):
                problems.append(f"headless type left {entry_text(tier, app)!r}, not {SEMANTIC!r}")
            if presses(log):
                problems.append(f"headless type sent keys: {presses(log)}")
            if read() != seen:
                problems.append(f"headless type changed the desktop: {seen} to {read()}")
            ctx.after("applog.presses", len(presses(log)))
            ctx.after("hyprland.desktop", read())
            ctx.after("hyprland.keyboards", keyboards(tier))
            if keyboards(tier) != before:
                problems.append(f"the keyboards changed: {before} to {keyboards(tier)}")
            ctx.restored("applog.presses", len(presses(log)))
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())
    ctx.restored("hyprland.keyboards", keyboards(tier))
