from __future__ import annotations

from oracle import atspi_raw
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import (
    REFUSAL_CODES,
    assert_ok,
    live_root,
    longest_text,
    raw_names,
    ref_for,
    window_of,
)


def _present(ctx: LegContext, app, title: str, deadline_s: float = 8.0):
    return atspi_raw.poll(lambda: title if title in raw_names(live_root(ctx.tier, app)) else None, deadline_s)




def _open_calculator_about(ctx: LegContext):
    app = apps.launch(ctx.tier, "Calculator")
    window = window_of(app)
    assert_ok(ctx.product(["click", ref_for(ctx, window, role="menubutton")]))
    about = ref_for(ctx, window, name="About", deadline_s=8.0)
    assert_ok(ctx.product(["click", about]))
    _present(ctx, app, "About")
    return app, window


def _open_terminal_preferences(ctx: LegContext):
    app = apps.launch(ctx.tier, "Terminal")
    window = window_of(app)
    assert_ok(ctx.product(["click", ref_for(ctx, window, role="menubutton")]))
    preferences = ref_for(ctx, window, name="Preferences", deadline_s=8.0)
    assert_ok(ctx.product(["click", preferences]))
    _present(ctx, app, "Preferences")
    return app, window


def _parent_action_blocked(ctx: LegContext, app, window: str, modal: str) -> None:
    before = raw_names(live_root(ctx.tier, app))
    ctx.before("atspi.names", before)
    target = ref_for(ctx, window, role="button")
    envelope = ctx.product(["click", target])
    if envelope.ok:
        raise AssertionError(f"{modal} did not block a parent action")
    code = (envelope.error or {}).get("code")
    if code not in REFUSAL_CODES:
        raise AssertionError(f"{modal} refusal carried {code!r}")
    after = atspi_raw.poll(lambda: raw_names(live_root(ctx.tier, app)), 4.0)
    ctx.after("atspi.names", after)
    if after != before:
        raise AssertionError(f"a parent action changed state while {modal} was open")




@implementation("modal.terminal_preferences_blocks_parent")
def terminal_preferences_blocks_parent(ctx: LegContext) -> None:
    app, window = _open_terminal_preferences(ctx)
    try:
        _parent_action_blocked(ctx, app, window, "Preferences")
    finally:
        apps.stop(app)


@implementation("modal.calculator_about_blocks_parent")
def calculator_about_blocks_parent(ctx: LegContext) -> None:
    app, window = _open_calculator_about(ctx)
    try:
        _parent_action_blocked(ctx, app, window, "About")
    finally:
        apps.stop(app)
