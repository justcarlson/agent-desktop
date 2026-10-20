from __future__ import annotations

from oracle import atspi_raw, gsettings, x11
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import assert_ok, count_live, live_root, ref_for, window_of


def _semantic_click(ctx: LegContext, app, window: str) -> None:
    before_pointer = x11.pointer(ctx.tier)
    before_active = x11.active_window(ctx.tier)
    before_count = count_live(ctx.tier, app, "7")
    ctx.before("x11.pointer", before_pointer)
    ctx.before("x11.active_window", before_active)
    ctx.before("atspi.names_7", before_count)
    assert_ok(ctx.product(["click", ref_for(ctx, window, "7")]))
    after_count = atspi_raw.poll(
        lambda: count_live(ctx.tier, app, "7") if count_live(ctx.tier, app, "7") > before_count else None,
        6.0,
    )
    ctx.after("atspi.names_7", after_count)
    if after_count is None:
        raise AssertionError("the semantic click left no effect")


@implementation("non_interference.pointer_unchanged_by_semantic_click")
def pointer_unchanged_by_semantic_click(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = window_of(app)
        pointer = x11.pointer(ctx.tier)
        _semantic_click(ctx, app, window)
        ctx.after("x11.pointer", x11.pointer(ctx.tier))
        if x11.pointer(ctx.tier) != pointer:
            raise AssertionError("a headless semantic click moved the pointer")
    finally:
        apps.stop(app)


@implementation("non_interference.active_window_unchanged_by_semantic_click")
def active_window_unchanged_by_semantic_click(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    other = apps.launch(ctx.tier, "Text Editor")
    try:
        active = x11.active_window(ctx.tier)
        _semantic_click(ctx, app, window_of(app))
        ctx.after("x11.active_window", x11.active_window(ctx.tier))
        if x11.active_window(ctx.tier) != active:
            raise AssertionError("a headless semantic click changed the active window")
    finally:
        apps.stop(other)
        apps.stop(app)


@implementation("non_interference.pointer_unchanged_by_value_write")
def pointer_unchanged_by_value_write(ctx: LegContext) -> None:
    schema, key = "org.gnome.desktop.peripherals.mouse", "speed"
    app = apps.launch(ctx.tier, "Settings", args=("mouse",))
    try:
        window = window_of(app)
        pointer = x11.pointer(ctx.tier)
        before = gsettings.get(ctx.tier, schema, key)
        ctx.before("x11.pointer", pointer)
        ctx.before("gsettings.speed", before)
        target = "0.2" if before != "0.2" else "0.4"
        assert_ok(ctx.product(["set-value", ref_for(ctx, window, role="slider"), target]))
        changed = atspi_raw.poll(
            lambda: gsettings.get(ctx.tier, schema, key)
            if gsettings.get(ctx.tier, schema, key) != before
            else None,
            6.0,
        )
        ctx.after("gsettings.speed", changed)
        ctx.after("x11.pointer", x11.pointer(ctx.tier))
        if changed is None:
            raise AssertionError("the semantic value write did not commit")
        if x11.pointer(ctx.tier) != pointer:
            raise AssertionError("a semantic value write moved the pointer")
    finally:
        apps.stop(app)
