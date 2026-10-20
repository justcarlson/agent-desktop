from __future__ import annotations

from runner.apps import launch, stop
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
    stack = [tree] if tree else []
    while stack:
        node = stack.pop()
        if node.get("ref"):
            return node["ref"]
        stack.extend(node.get("children") or [])
    return None


def _editor(ctx: LegContext):
    app = launch(ctx.tier, TEXT_EDITOR)
    return app, app_root(ctx.tier, "gnome-text-editor")


def _press(ctx: LegContext, combo: str) -> None:
    envelope = ctx.product(["press", combo])
    assert envelope.ok, envelope.raw


@implementation("wayland_focus.seat_keyboard_makes_active")
def seat_keyboard_makes_active(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    ctx.before("atspi.active", [])
    _press(ctx, "a")
    observed = poll(lambda: active_names(app_root(ctx.tier, "gnome-text-editor")), 5.0)
    ctx.after("atspi.active", observed)
    assert observed, "no window carried ACTIVE after the seat keyboard arrived"
    stop(app)


@implementation("wayland_focus.later_mapped_window_takes_focus")
def later_mapped_window_takes_focus(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    _press(ctx, "a")
    before = poll(lambda: active_names(app_root(ctx.tier, "gnome-text-editor")), 5.0)
    ctx.before("atspi.active", before)
    later = launch(ctx.tier, TEXT_EDITOR, ("--new-window",))
    observed = poll(lambda: active_names(app_root(ctx.tier, "gnome-text-editor")), 6.0)
    ctx.after("atspi.active", observed)
    assert observed and observed != before, f"focus stayed on {before!r}"
    stop(later)
    stop(app)


@implementation("wayland_focus.focus_changed_times_out_with_baseline")
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


@implementation("wayland_focus.headed_ref_action_refusal")
def headed_ref_action_refusal(ctx: LegContext) -> None:
    app, root = _editor(ctx)
    snapshot = ctx.product(["snapshot", "--app", TEXT_EDITOR, "-i"])
    assert snapshot.ok, snapshot.raw
    ref = first_ref((snapshot.data or {}).get("tree"))
    assert ref, "the snapshot allocated no ref"
    ctx.before("atspi.document_text", "")
    envelope = ctx.product(["click", ref], headed=True)
    ctx.after("atspi.document_text", "")
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "ACTION_NOT_SUPPORTED", envelope.raw
    error = {"message": (envelope.error or {}).get("message", ""), "details": (envelope.error or {}).get("details")}
    assert search(error, "wayland_session") or search(error, "focus"), envelope.raw
    stop(app)
