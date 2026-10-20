from __future__ import annotations

import subprocess

from oracle import atspi_raw, x11
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import (
    FOCUS_REFUSAL_CODES,
    REFUSAL_CODES,
    assert_ok,
    live_root,
    longest_text,
    raw_walk,
    ref_for,
    window_of,
)

SEARCH_NAMES = ("Find", "Search", "Replace", "Find and Replace")


def held_modifiers(text: str) -> list[str]:
    held = []
    for line in text.splitlines():
        name, _, state = line.partition(":")
        if state.strip().lower() not in ("off", "up", ""):
            held.append(name.strip())
    return held


def _raw_node(root, name: str):
    for node in raw_walk(root):
        if node.name == name:
            return node
    return None


def _search_present(tier, app) -> bool:
    root = live_root(tier, app)
    return any(_raw_node(root, name) for name in SEARCH_NAMES)


def _enabled(tier, app, name: str) -> bool | None:
    node = _raw_node(live_root(tier, app), name)
    if node is None:
        return None
    return "SENSITIVE" in node.states


def _textfield_ref(ctx: LegContext, window: str) -> str:
    return ref_for(ctx, window, role="textfield", deadline_s=6.0)


def _launch(ctx: LegContext):
    app = apps.launch(ctx.tier, "Text Editor")
    return app, window_of(app)


@implementation("text_entry.typed_text_reaches_document")
def typed_text_reaches_document(ctx: LegContext) -> None:
    app, window = _launch(ctx)
    try:
        before = longest_text(live_root(ctx.tier, app))
        ctx.before("atspi.document_text", before)
        assert_ok(ctx.product(["type", ref_for(ctx, window, role="text"), "typed words"]))
        after = atspi_raw.poll(
            lambda: longest_text(live_root(ctx.tier, app))
            if "typed words" in longest_text(live_root(ctx.tier, app))
            else None,
            6.0,
        )
        ctx.after("atspi.document_text", after)
        if not after:
            raise AssertionError("typed text did not reach the document")
    finally:
        apps.stop(app)


@implementation("text_entry.find_bar_ctrl_f")
def find_bar_ctrl_f(ctx: LegContext) -> None:
    app, window = _launch(ctx)
    try:
        ctx.before("atspi.search_bar", _search_present(ctx.tier, app))
        assert_ok(ctx.product(["press", "ctrl+f", "--window-id", window]))
        found = atspi_raw.poll(lambda: True if _search_present(ctx.tier, app) else None, 6.0)
        ctx.after("atspi.search_bar", found)
        if not found:
            raise AssertionError("Ctrl+F did not open the find bar")
        assert_ok(ctx.product(["press", "escape", "--window-id", window]))
        ctx.restored(
            "atspi.search_bar",
            atspi_raw.poll(lambda: not _search_present(ctx.tier, app), 6.0),
        )
    finally:
        apps.stop(app)


@implementation("text_entry.replace_bar_ctrl_h")
def replace_bar_ctrl_h(ctx: LegContext) -> None:
    app, window = _launch(ctx)
    try:
        ctx.before("atspi.replace_all", _raw_node(live_root(ctx.tier, app), "Replace All") is not None)
        assert_ok(ctx.product(["press", "ctrl+h", "--window-id", window]))
        found = atspi_raw.poll(
            lambda: True if _raw_node(live_root(ctx.tier, app), "Replace All") is not None else None,
            6.0,
        )
        ctx.after("atspi.replace_all", found)
        if not found:
            raise AssertionError("Ctrl+H did not open the replace bar")
    finally:
        apps.stop(app)


@implementation("text_entry.replace_all_gating")
def replace_all_gating(ctx: LegContext) -> None:
    app, window = _launch(ctx)
    try:
        assert_ok(ctx.product(["press", "ctrl+h", "--window-id", window]))
        atspi_raw.poll(lambda: _raw_node(live_root(ctx.tier, app), "Replace All") is not None, 6.0)
        ctx.before("atspi.replace_all_sensitive", _enabled(ctx.tier, app, "Replace All"))
        entry = _textfield_ref(ctx, window)
        assert_ok(ctx.product(["type", entry, "needle"]))
        enabled = atspi_raw.poll(
            lambda: True if _enabled(ctx.tier, app, "Replace All") else None, 6.0
        )
        ctx.after("atspi.replace_all_sensitive", enabled)
        if not enabled:
            raise AssertionError("Replace All did not enable on typed search text")
        assert_ok(ctx.product(["clear", _textfield_ref(ctx, window)]))
        ctx.restored(
            "atspi.replace_all_sensitive",
            atspi_raw.poll(lambda: not _enabled(ctx.tier, app, "Replace All"), 6.0),
        )
    finally:
        apps.stop(app)


def _modifier_state(tier) -> str:
    process = subprocess.run(
        ["xdotool", "getmodifierstate"],
        capture_output=True,
        text=True,
        env=tier.env,
        timeout=5,
        check=False,
    )
    return process.stdout


@implementation("text_entry.held_key_released")
def held_key_released(ctx: LegContext) -> None:
    app, window = _launch(ctx)
    try:
        ctx.before("x11.modifiers", held_modifiers(_modifier_state(ctx.tier)))
        assert_ok(ctx.product(["press", "shift+a", "--window-id", window]))
        after = atspi_raw.poll(lambda: held_modifiers(_modifier_state(ctx.tier)), 3.0)
        ctx.after("x11.modifiers", after)
        if held_modifiers(_modifier_state(ctx.tier)):
            raise AssertionError("a modifier was left pressed after the key combo")
    finally:
        apps.stop(app)


@implementation("text_entry.unfocused_window_refusal")
def unfocused_window_refusal(ctx: LegContext) -> None:
    editor, window = _launch(ctx)
    other = apps.launch(ctx.tier, "Calculator")
    try:
        assert_ok(ctx.product(["click", ref_for(ctx, window_of(other), "7")]))
        before = longest_text(live_root(ctx.tier, editor))
        ctx.before("x11.active_window", active_or_none(ctx))
        envelope = ctx.product(["type", ref_for(ctx, window, role="text"), "unfocused"])
        after = longest_text(live_root(ctx.tier, editor))
        ctx.after("atspi.document_text", after)
        if ctx.tier.name == "x11-mutter" and envelope.ok:
            raise AssertionError("mutter accepted a headed action on a never-focused window")
        if envelope.ok and after == before:
            raise AssertionError("the action succeeded without reaching the document")
        if not envelope.ok:
            code = (envelope.error or {}).get("code")
            details = (envelope.error or {}).get("details") or {}
            if code not in REFUSAL_CODES and details.get("kind") not in FOCUS_REFUSAL_CODES:
                raise AssertionError(f"the refusal carried {code!r}")
            if after != before:
                raise AssertionError("input reached a window the product did not focus")
    finally:
        apps.stop(other)
        apps.stop(editor)


def active_or_none(ctx: LegContext):
    return x11.active_window(ctx.tier)
