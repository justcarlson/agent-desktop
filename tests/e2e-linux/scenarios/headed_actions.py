from __future__ import annotations

import re

from oracle import atspi_raw, gsettings, x11
from runner import apps, helper as helper_module
from runner.legs import implementation
from runner.verdict import LegContext

REFUSAL_CODES = frozenset(
    {
        "ACTION_FAILED",
        "ACTION_NOT_SUPPORTED",
        "APP_UNRESPONSIVE",
        "ELEMENT_NOT_FOUND",
        "POLICY_DENIED",
        "STALE_REF",
        "TIMEOUT",
        "WINDOW_NOT_FOUND",
    }
)
FOCUS_REFUSAL_CODES = frozenset(
    {"activation_not_granted", "activation_refused", "frame_not_active", "overview_open"}
)
REFERENCE_RE = re.compile(r"^@[^:]+:e\d+$")


def walk(nodes) -> list[dict]:
    if isinstance(nodes, dict):
        nodes = [nodes]
    collected: list[dict] = []
    for node in nodes or ():
        collected.append(node)
        collected.extend(walk(node.get("children")))
    return collected


def find_ref(tree, name: str | None = None, role: str | None = None) -> str | None:
    for node in walk(tree):
        if name is not None and node.get("name") != name:
            continue
        if role is not None and node.get("role") != role:
            continue
        ref = node.get("ref_id")
        if isinstance(ref, str) and REFERENCE_RE.match(ref):
            return ref
    return None


def count_name(tree, name: str) -> int:
    return sum(1 for node in walk(tree) if node.get("name") == name)


def raw_walk(node) -> list:
    collected = [node]
    for child in node.children():
        collected.extend(raw_walk(child))
    return collected


def raw_names(root) -> list[str]:
    return [node.name for node in raw_walk(root)]


def checked_map(root) -> dict[str, bool]:
    states: dict[str, bool] = {}
    for node in raw_walk(root):
        if "CHECKABLE" in node.states and node.name:
            states[node.name] = "CHECKED" in node.states
    return states


def longest_text(root) -> str:
    texts = [node.text() for node in raw_walk(root)]
    return max((text for text in texts if text), key=len, default="")


def restore_text(ctx, app, before):
    target = ref_for(ctx, window_of(app), role="textfield")
    assert_ok(ctx.product(["clear", target]))
    if before:
        assert_ok(ctx.product(["type", target, before]))
    ctx.restored("atspi.document_text", helper_module.changed(
        lambda: longest_text(live_root(ctx.tier, app)), before))


def press_raw(root, name: str) -> bool:
    for node in raw_walk(root):
        if node.name != name:
            continue
        for action in ("click", "press"):
            if node.do_action(action):
                return True
    return False


def assert_ok(envelope) -> dict:
    if not envelope.ok:
        raise AssertionError(f"{envelope.argv} failed: {envelope.error}")
    return envelope.data or {}


def window_of(app) -> str:
    if not app.windows:
        raise AssertionError(f"{app.name} presented no window")
    return app.windows[0]


def snapshot(ctx: LegContext, window: str) -> dict:
    data = assert_ok(ctx.product(["snapshot", "--window-id", window, "-i"]))
    return data.get("tree") or {}


def ref_for(
    ctx: LegContext,
    window: str,
    name: str | None = None,
    role: str | None = None,
    deadline_s: float = 6.0,
) -> str:
    def resolve():
        return find_ref(snapshot(ctx, window), name=name, role=role)

    ref = atspi_raw.poll(resolve, deadline_s)
    if not ref:
        raise AssertionError(f"no ref for name={name!r} role={role!r} in window {window}")
    return ref


def live_root(tier, app):
    return atspi_raw.app_root(tier, app.owner_names[0])


def count_live(tier, app, name: str) -> int:
    return raw_names(live_root(tier, app)).count(name)


def wait_count(tier, app, name: str, target: int, deadline_s: float = 6.0) -> int | None:
    def observe():
        value = count_live(tier, app, name)
        return value if value >= target else None

    return atspi_raw.poll(observe, deadline_s)


def open_main_menu(ctx: LegContext, window: str) -> None:
    ref = ref_for(ctx, window, role="menubutton")
    assert_ok(ctx.product(["click", ref]))


def stop(app) -> None:
    apps.stop(app)


def _changed_checked(tier, app, before):
    current = checked_map(live_root(tier, app))
    return current if current != before else None


def _restored_checked(tier, app, before):
    current = checked_map(live_root(tier, app))
    return current if current == before else None


@implementation("headed_actions.focus_reads_active_window")
def focus_reads_active_window(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    other = apps.launch(ctx.tier, "Text Editor")
    try:
        window = window_of(app)
        native = x11.xid(ctx.tier, window)
        before = x11.active_window(ctx.tier)
        ctx.before("x11.active_window", before)
        assert_ok(ctx.product(["click", ref_for(ctx, window, "7")]))
        active = atspi_raw.poll(
            lambda: x11.active_window(ctx.tier) if x11.active_window(ctx.tier) == native else None, 5.0
        )
        ctx.after("x11.active_window", active)
        if active != native:
            ctx.finding("focus did not lift the target to the active window")
            raise AssertionError("the focus step did not make the target active")
        ctx.restored("x11.active_window", x11.restore_active(ctx.tier, before))
    finally:
        stop(other)
        stop(app)


def _minutes_of(value: str) -> str:
    digits = "".join(character for character in value if character.isdigit())
    seconds = int(digits or "0")
    minutes = max(1, seconds // 60)
    return f"{minutes} minutes"


def _expanded_count(root) -> int:
    return sum(1 for node in raw_walk(root) if "EXPANDED" in node.states)


@implementation("headed_actions.type_text")
def type_text(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Text Editor")
    try:
        window = window_of(app)
        before = longest_text(live_root(ctx.tier, app))
        ctx.before("atspi.document_text", before)
        target = ref_for(ctx, window, role="textfield")
        assert_ok(ctx.product(["type", target, "headed text"]))
        after = atspi_raw.poll(lambda: _grown_text(ctx.tier, app, before), 6.0)
        ctx.after("atspi.document_text", after)
        if not after or "headed text" not in after:
            raise AssertionError("typed text did not reach the document")
        restore_text(ctx, app, before)
    finally:
        stop(app)


def _grown_text(tier, app, before):
    current = longest_text(live_root(tier, app))
    return current if current and current != before else None
