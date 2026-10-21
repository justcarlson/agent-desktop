from __future__ import annotations

from scenarios.headed_actions import find_ref as qualified_ref

from runner.apps import launch, stop, window_id
from runner.helper import changed
from scenarios.launch import _close_added_windows, _window_ids
from scenarios.wayland_input import document_text
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root, poll

TEXT_EDITOR = "Text Editor"


def walk(node) -> list:
    nodes = [node]
    for child in node.children():
        nodes.extend(walk(child))
    return nodes


def active_names(root) -> list[str]:
    return [
        node.name
        for node in walk(root)
        if "ACTIVE" in getattr(node, "states", frozenset()) and node.name
    ]


def code_of(envelope) -> str:
    return str((envelope.error or {}).get("code", ""))


def search(value, needle: str) -> bool:
    target = needle.lower()
    if isinstance(value, str):
        return target in value.lower()
    if isinstance(value, dict):
        return any(search(key, needle) or search(item, needle) for key, item in value.items())
    if isinstance(value, (list, tuple)):
        return any(search(item, needle) for item in value)
    return False


def first_ref(tree: dict | None) -> str | None:
    return qualified_ref(tree)


def _editor(ctx: LegContext):
    app = launch(ctx.tier, TEXT_EDITOR)
    return app, app_root(ctx.tier, "gnome-text-editor")


def _press(ctx: LegContext, combo: str) -> None:
    envelope = ctx.product(["press", combo])
    assert envelope.ok, envelope.raw


def seat_keyboard_makes_active(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    before = active_names(root)
    ctx.before("atspi.active", before)
    _press(ctx, "escape")
    observed = poll(lambda: active_names(app_root(ctx.tier, "gnome-text-editor")), 5.0)
    ctx.after("atspi.active", observed)
    assert observed, "no window carried ACTIVE after the seat keyboard arrived"
    stop(app)
    def read():
        try:
            return active_names(app_root(ctx.tier, "gnome-text-editor"))
        except LookupError:
            return []
    if before != observed:
        ctx.restored("atspi.active", changed(read, before))


def _active_ids(ctx):
    return sorted(window_id(node) for node in app_root(ctx.tier, "gnome-text-editor").children()
                  if node.role in (16, 23, 69) and "ACTIVE" in node.states)


def later_mapped_window_takes_focus(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    _press(ctx, "escape")
    before = poll(lambda: _active_ids(ctx), 5.0)
    existing = _window_ids(ctx.tier)
    ctx.before("atspi.active", before)
    later = launch(ctx.tier, TEXT_EDITOR, ("--new-window",))
    observed = poll(lambda: _active_ids(ctx) if _active_ids(ctx) != before else None, 6.0)
    ctx.after("atspi.active", observed)
    assert observed and observed != before, f"focus stayed on {before!r}"
    _close_added_windows(ctx.tier, existing)
    ctx.restored("atspi.active", changed(lambda: _active_ids(ctx), before))
    stop(app)


def focus_changed_times_out_with_baseline(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    ctx.before("atspi.active", active_names(root))
    envelope = ctx.product(["wait", "--event", "focus-changed", "--timeout", "1500"])
    ctx.after("atspi.active", active_names(root))
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "TIMEOUT", envelope.raw
    details = (envelope.error or {}).get("details") or {}
    assert details.get("baseline_counts"), envelope.raw
    stop(app)


def headed_ref_action_refusal(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    snapshot = ctx.product(["snapshot", "--app", TEXT_EDITOR, "-i"])
    assert snapshot.ok, snapshot.raw
    ref = first_ref((snapshot.data or {}).get("tree"))
    assert ref, "the snapshot allocated no ref"
    ctx.before("atspi.document_text", document_text(root))
    envelope = ctx.product(["click", ref], headed=True)
    ctx.after("atspi.document_text", document_text(app_root(ctx.tier, "gnome-text-editor")))
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "ACTION_NOT_SUPPORTED", envelope.raw
    error = {"message": (envelope.error or {}).get("message", ""), "details": (envelope.error or {}).get("details")}
    assert search(error, "wayland_session") or search(error, "focus"), envelope.raw
    stop(app)
