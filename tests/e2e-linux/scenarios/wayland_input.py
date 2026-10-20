from __future__ import annotations

from runner.apps import launch, stop
from runner.legs import implementation
from runner.verdict import LegContext
from oracle import clipboard
from oracle.atspi_raw import app_root, poll

TEXT_EDITOR = "Text Editor"


def walk(node) -> list:
    nodes = [node]
    for child in node.children():
        nodes.extend(walk(child))
    return nodes


def document_text(root) -> str:
    parts = []
    for node in walk(root):
        text = node.text()
        if text:
            parts.append(text)
    return "\n".join(parts)


def text_ref(tree: dict | None) -> str | None:
    stack = [tree] if tree else []
    while stack:
        node = stack.pop()
        if node.get("ref") and node.get("role") in ("textbox", "text", "entry"):
            return node["ref"]
        stack.extend(node.get("children") or [])
    return None


def _editor(ctx: LegContext):
    app = launch(ctx.tier, TEXT_EDITOR)
    root = app_root(ctx.tier, "gnome-text-editor")
    snapshot = ctx.product(["snapshot", "--app", TEXT_EDITOR, "-i"])
    assert snapshot.ok, snapshot.raw
    ref = text_ref((snapshot.data or {}).get("tree"))
    assert ref, "Text Editor exposes no document ref"
    return app, root, ref


def _named(root, needle: str) -> list[str]:
    return [node.name for node in root.find() if needle.lower() in node.name.lower()]


@implementation("wayland_input.first_key_lands")
def first_key_lands(ctx: LegContext) -> None:
    app, root, ref = _editor(ctx)
    before = document_text(root)
    ctx.before("atspi.document_text", before)
    envelope = ctx.product(["type", ref, "h"])
    after = poll(lambda: document_text(app_root(ctx.tier, "gnome-text-editor")), 4.0)
    ctx.after("atspi.document_text", after)
    assert envelope.ok, envelope.raw
    assert "h" in after, f"the first key did not land: {after!r}"
    stop(app)


@implementation("wayland_input.modifiers_work")
def modifiers_work(ctx: LegContext) -> None:
    app, root, ref = _editor(ctx)
    ctx.product(["type", ref, "abc"])
    ctx.before("atspi.document_text", document_text(root))
    ctx.product(["press", "ctrl+a", "--window-id", _window_id(ctx)])
    ctx.product(["type", ref, "z"])
    after = poll(lambda: document_text(app_root(ctx.tier, "gnome-text-editor")), 4.0)
    ctx.after("atspi.document_text", after)
    assert "z" in after and "abc" not in after, after
    stop(app)


def _window_id(ctx: LegContext) -> str:
    envelope = ctx.product(["list-windows", "--app", TEXT_EDITOR])
    assert envelope.ok and envelope.data, envelope.raw
    return envelope.data[0]["id"]


@implementation("wayland_input.ctrl_f_two_keycodes")
def ctrl_f_two_keycodes(ctx: LegContext) -> None:
    app, root, _ref = _editor(ctx)
    ctx.before("atspi.find_bar", False)
    envelope = ctx.product(["press", "ctrl+f", "--window-id", _window_id(ctx)])
    assert envelope.ok, envelope.raw
    observed = poll(lambda: _named(app_root(ctx.tier, "gnome-text-editor"), "find"), 4.0)
    ctx.after("atspi.find_bar", bool(observed))
    assert observed, "Ctrl+F opened no find bar"
    stop(app)


@implementation("wayland_input.clipboard_paste_into_app")
def clipboard_paste_into_app(ctx: LegContext) -> None:
    app, root, _ref = _editor(ctx)
    owner = clipboard.foreign_set(ctx.tier, "text", b"harness-paste")
    ctx.before("clipboard.text", b"harness-paste")
    ctx.product(["press", "ctrl+v", "--window-id", _window_id(ctx)])
    after = poll(lambda: document_text(app_root(ctx.tier, "gnome-text-editor")), 4.0)
    ctx.after("atspi.document_text", after)
    assert "harness-paste" in after, after
    assert owner > 0
    stop(app)


@implementation("wayland_input.key_before_throwaway_not_rescued")
def key_before_throwaway_not_rescued(ctx: LegContext) -> None:
    app = launch(ctx.tier, TEXT_EDITOR)
    root = app_root(ctx.tier, "gnome-text-editor")
    before = document_text(root)
    ctx.before("atspi.document_text", before)
    envelope = ctx.product(["press", "x", "--timeout-ms", "500"])
    after = document_text(app_root(ctx.tier, "gnome-text-editor"))
    ctx.after("atspi.document_text", after)
    if not envelope.ok or "x" not in after:
        ctx.finding("a key sent inside the throwaway window did not land and the harness did not retry it")
    stop(app)
