from __future__ import annotations

from oracle import atspi_raw, x11
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import (
    REFUSAL_CODES,
    assert_ok,
    live_root,
    longest_text,
    ref_for,
    window_of,
)


def _document_ref(ctx: LegContext, window: str) -> str:
    for role in ("text", "textbox", "textfield"):
        try:
            return ref_for(ctx, window, role=role, deadline_s=2.0)
        except AssertionError:
            continue
    raise AssertionError("the editor presented no document node")


def _document_text(ctx: LegContext, app) -> str:
    return longest_text(live_root(ctx.tier, app))


def _move_over(ctx: LegContext, cover, target_window: str) -> None:
    x, y, _, _ = x11.geometry(ctx.tier, target_window)
    assert_ok(ctx.product(["move-window", "--window-id", window_of(cover), "--x", str(x + 40), "--y", str(y + 40)]))
    atspi_raw.poll(lambda: x11.geometry(ctx.tier, window_of(cover))[0] == x + 40, 6.0)


def _above(ctx: LegContext, cover) -> bool:
    order = x11.stacking(ctx.tier)
    window = window_of(cover)
    target = [item for item in order if item == window]
    return bool(target) and order.index(window) > len(order) // 2


def _setup(ctx: LegContext):
    editor = apps.launch(ctx.tier, "Text Editor")
    window = window_of(editor)
    assert_ok(ctx.product(["type", _document_ref(ctx, window), "baseline"]))
    atspi_raw.poll(lambda: "baseline" in _document_text(ctx, editor), 6.0)
    cover = apps.launch(ctx.tier, "Calculator")
    _move_over(ctx, cover, window)
    return editor, window, cover


@implementation("occlusion.ordinary_cover_removed_click_lands")
def ordinary_cover_removed_click_lands(ctx: LegContext) -> None:
    editor, window, cover = _setup(ctx)
    try:
        before = _document_text(ctx, editor)
        ctx.before("atspi.document_text", before)
        assert_ok(ctx.product(["type", _document_ref(ctx, window), "ordinary"]))
        after = atspi_raw.poll(lambda: _grown(ctx, editor, before), 6.0)
        ctx.after("atspi.document_text", after)
        if not after:
            raise AssertionError("the headed action did not reach the covered editor")
    finally:
        apps.stop(cover)
        apps.stop(editor)


def _grown(ctx, app, before):
    current = _document_text(ctx, app)
    return current if current != before else None


@implementation("occlusion.above_cover_refused_no_effect")
def above_cover_refused_no_effect(ctx: LegContext) -> None:
    editor, window, cover = _setup(ctx)
    x11.set_above(ctx.tier, window_of(cover), True)
    try:
        before = _document_text(ctx, editor)
        ctx.before("atspi.document_text", before)
        ctx.before("x11.stacking", x11.stacking(ctx.tier))
        envelope = ctx.product(["type", _document_ref(ctx, window), "covered"])
        if envelope.ok:
            raise AssertionError("an above-layer cover did not block the action")
        code = (envelope.error or {}).get("code")
        if code not in REFUSAL_CODES:
            raise AssertionError(f"the refusal carried {code!r}")
        ctx.after("atspi.document_text", _document_text(ctx, editor))
        ctx.after("x11.stacking", x11.stacking(ctx.tier))
        if _document_text(ctx, editor) != before:
            raise AssertionError("the covered document changed")
        if not _above(ctx, cover):
            raise AssertionError("the cover was not left above the target")
    finally:
        x11.set_above(ctx.tier, window_of(cover), False)
        apps.stop(cover)
        apps.stop(editor)


@implementation("occlusion.above_flag_dropped_click_lands")
def above_flag_dropped_click_lands(ctx: LegContext) -> None:
    editor, window, cover = _setup(ctx)
    x11.set_above(ctx.tier, window_of(cover), True)
    x11.set_above(ctx.tier, window_of(cover), False)
    try:
        before = _document_text(ctx, editor)
        ctx.before("atspi.document_text", before)
        assert_ok(ctx.product(["type", _document_ref(ctx, window), "dropped"]))
        after = atspi_raw.poll(lambda: _grown(ctx, editor, before), 6.0)
        ctx.after("atspi.document_text", after)
        if not after:
            raise AssertionError("the click did not land once the above flag was dropped")
    finally:
        apps.stop(cover)
        apps.stop(editor)
