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


@implementation("headed_actions.click_button")
def click_button(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = window_of(app)
        before = count_live(ctx.tier, app, "7")
        ctx.before("atspi.names_7", before)
        assert_ok(ctx.product(["click", ref_for(ctx, window, "7")]))
        after = wait_count(ctx.tier, app, "7", before + 1)
        ctx.after("atspi.names_7", after)
        if after != before + 1:
            raise AssertionError(f"display did not take the click: {before} -> {after}")
        clear = find_ref(snapshot(ctx, window), name="AC") or find_ref(snapshot(ctx, window), name="Clear")
        if clear:
            assert_ok(ctx.product(["click", clear]))
            ctx.restored("atspi.names_7", wait_count(ctx.tier, app, "7", before))
    finally:
        stop(app)


@implementation("headed_actions.toggle_switch")
def toggle_switch(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = window_of(app)
        open_main_menu(ctx, window)
        switch = ref_for(ctx, window, role="switch")
        root = live_root(ctx.tier, app)
        before = checked_map(root)
        ctx.before("atspi.checked", before)
        assert_ok(ctx.product(["toggle", switch]))
        after = atspi_raw.poll(lambda: _changed_checked(ctx.tier, app, before), 6.0)
        ctx.after("atspi.checked", after)
        if not after:
            raise AssertionError("no switch changed its checked state")
        open_main_menu(ctx, window)
        switch = ref_for(ctx, window, role="switch")
        assert_ok(ctx.product(["toggle", switch]))
        ctx.restored("atspi.checked", atspi_raw.poll(lambda: _restored_checked(ctx.tier, app, before), 6.0))
    finally:
        stop(app)


def _changed_checked(tier, app, before):
    current = checked_map(live_root(tier, app))
    return current if current != before else None


def _restored_checked(tier, app, before):
    current = checked_map(live_root(tier, app))
    return current if current == before else None


@implementation("headed_actions.delayed_enable")
def delayed_enable(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = window_of(app)
        root = live_root(ctx.tier, app)
        undo = ref_for(ctx, window, name="Undo")
        before = count_live(ctx.tier, app, "7")
        ctx.before("atspi.names_7", before)
        trigger = helper_module.delayed(2.0, lambda: _press_twice(ctx.tier, app))
        assert_ok(ctx.product(["click", undo, "--timeout-ms", "8000"]))
        trigger.join()
        after = wait_count(ctx.tier, app, "7", before + 1)
        ctx.after("atspi.names_7", after)
        if after != before + 1:
            raise AssertionError("one Undo click did not leave a single digit")
    finally:
        stop(app)


def _press_twice(tier, app) -> None:
    root = live_root(tier, app)
    press_raw(root, "7")
    press_raw(live_root(tier, app), "7")


@implementation("headed_actions.focus_reads_active_window")
def focus_reads_active_window(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    other = apps.launch(ctx.tier, "Text Editor")
    try:
        window = window_of(app)
        ctx.before("x11.active_window", x11.active_window(ctx.tier))
        assert_ok(ctx.product(["click", ref_for(ctx, window, "7")]))
        active = atspi_raw.poll(
            lambda: window if x11.active_window(ctx.tier) == window else None, 5.0
        )
        ctx.after("x11.active_window", active)
        if active != window:
            ctx.finding("focus did not lift the target to the active window")
            raise AssertionError("the focus step did not make the target active")
    finally:
        stop(other)
        stop(app)


@implementation("headed_actions.set_value_commit")
def set_value_commit(ctx: LegContext) -> None:
    schema, key = "org.gnome.desktop.peripherals.mouse", "speed"
    app = apps.launch(ctx.tier, "Settings", args=("mouse",))
    try:
        window = window_of(app)
        before = gsettings.get(ctx.tier, schema, key)
        ctx.before("gsettings.speed", before)
        slider = ref_for(ctx, window, role="slider")
        target = "0.3" if before != "0.3" else "0.6"
        assert_ok(ctx.product(["set-value", slider, target]))
        after = atspi_raw.poll(
            lambda: gsettings.get(ctx.tier, schema, key) if gsettings.get(ctx.tier, schema, key) != before else None,
            6.0,
        )
        ctx.after("gsettings.speed", after)
        if after is None:
            raise AssertionError("the pointer speed key did not change")
        assert_ok(ctx.product(["set-value", ref_for(ctx, window, role="slider"), before]))
        ctx.restored("gsettings.speed", atspi_raw.poll(lambda: gsettings.get(ctx.tier, schema, key) == before, 6.0))
    finally:
        stop(app)


@implementation("headed_actions.select_combo")
def select_combo(ctx: LegContext) -> None:
    schema, key = "org.gnome.desktop.session", "idle-delay"
    app = apps.launch(ctx.tier, "Settings", args=("power",))
    try:
        window = window_of(app)
        before = gsettings.get(ctx.tier, schema, key)
        ctx.before("gsettings.idle_delay", before)
        combo = ref_for(ctx, window, role="combobox")
        choice = "2 minutes" if "300" in before else "5 minutes"
        assert_ok(ctx.product(["select", combo, choice]))
        after = atspi_raw.poll(
            lambda: gsettings.get(ctx.tier, schema, key) if gsettings.get(ctx.tier, schema, key) != before else None,
            6.0,
        )
        ctx.after("gsettings.idle_delay", after)
        if after is None:
            raise AssertionError("Screen Blank did not commit through its semantic route")
        assert_ok(ctx.product(["select", ref_for(ctx, window, role="combobox"), _minutes_of(before)]))
        ctx.restored("gsettings.idle_delay", atspi_raw.poll(lambda: gsettings.get(ctx.tier, schema, key) == before, 6.0))
    finally:
        stop(app)


def _minutes_of(value: str) -> str:
    digits = "".join(character for character in value if character.isdigit())
    seconds = int(digits or "0")
    minutes = max(1, seconds // 60)
    return f"{minutes} minutes"


@implementation("headed_actions.select_combo_modifier_keys")
def select_combo_modifier_keys(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = window_of(app)
        for combo in ("super", "win"):
            assert_ok(ctx.product(["press", combo, "--window-id", window]))
            ctx.after(f"x11.overview.{combo}", x11.overview_active(ctx.tier))
            if x11.overview_active(ctx.tier):
                assert_ok(ctx.product(["press", "escape", "--window-id", window]))
                ctx.restored(
                    f"x11.overview.{combo}",
                    atspi_raw.poll(lambda: not x11.overview_active(ctx.tier), 6.0),
                )
    finally:
        stop(app)


@implementation("headed_actions.expand_collapse_row")
def expand_collapse_row(ctx: LegContext) -> None:
    settings = [
        ("org.gnome.nautilus.list-view", "use-tree-view", "true"),
        ("org.gnome.nautilus.preferences", "default-folder-viewer", "list-view"),
    ]
    saved = [(schema, key, gsettings.get(ctx.tier, schema, key)) for schema, key, _ in settings]
    for schema, key, value in settings:
        gsettings.set(ctx.tier, schema, key, value)
    app = apps.launch(ctx.tier, "Files")
    try:
        window = window_of(app)
        row = ref_for(ctx, window, role="treeitem")
        root = live_root(ctx.tier, app)
        before = _expanded_count(root)
        ctx.before("atspi.expanded", before)
        assert_ok(ctx.product(["expand", row]))
        after = atspi_raw.poll(lambda: _expanded_count(live_root(ctx.tier, app)), 6.0)
        if after is None or after <= before:
            raise AssertionError("the row did not read as expanded")
        ctx.after("atspi.expanded", after)
        row = ref_for(ctx, window, role="treeitem")
        assert_ok(ctx.product(["collapse", row]))
        ctx.restored("atspi.expanded", atspi_raw.poll(lambda: _expanded_count(live_root(ctx.tier, app)) == before, 6.0))
    finally:
        stop(app)
        for schema, key, value in saved:
            gsettings.set(ctx.tier, schema, key, value)


def _expanded_count(root) -> int:
    return sum(1 for node in raw_walk(root) if "EXPANDED" in node.states)


@implementation("headed_actions.type_text")
def type_text(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Text Editor")
    try:
        window = window_of(app)
        before = longest_text(live_root(ctx.tier, app))
        ctx.before("atspi.document_text", before)
        target = ref_for(ctx, window, role="text")
        assert_ok(ctx.product(["type", target, "headed text"]))
        after = atspi_raw.poll(lambda: _grown_text(ctx.tier, app, before), 6.0)
        ctx.after("atspi.document_text", after)
        if not after or "headed text" not in after:
            raise AssertionError("typed text did not reach the document")
    finally:
        stop(app)


def _grown_text(tier, app, before):
    current = longest_text(live_root(tier, app))
    return current if current and current != before else None


@implementation("headed_actions.never_focused_window_refusal")
def never_focused_window_refusal(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Text Editor")
    try:
        window = window_of(app)
        before = longest_text(live_root(ctx.tier, app))
        ctx.before("x11.active_window", x11.active_window(ctx.tier))
        target = ref_for(ctx, window, role="text")
        envelope = ctx.product(["type", target, "never focused"])
        if envelope.ok:
            raise AssertionError("a never-focused window accepted a headed action")
        details = (envelope.error or {}).get("details") or {}
        cause = details.get("kind") if details.get("kind") in FOCUS_REFUSAL_CODES else (envelope.error or {}).get("code")
        ctx.after("x11.active_window", x11.active_window(ctx.tier))
        after = longest_text(live_root(ctx.tier, app))
        ctx.after("atspi.document_text", after)
        if cause not in FOCUS_REFUSAL_CODES:
            raise AssertionError(f"the refusal carried {cause!r}, not a landed focus cause")
        if after != before:
            ctx.finding("a never-focused window took input despite the refusal")
            raise AssertionError("input reached a window the focus step refused")
    finally:
        stop(app)
