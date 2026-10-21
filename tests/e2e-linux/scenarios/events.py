from __future__ import annotations

from runner.apps import launch, stop, window_id
from runner.helper import changed, delayed
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root, registry, poll
from oracle.x11 import active_window, restore_active
from scenarios.launch import _close_added_windows, _window_ids
from scenarios.menus import _click_named

TEXT_EDITOR = "Text Editor"
CALCULATOR = "Calculator"


def event_kind(data: dict | None) -> str:
    event = (data or {}).get("event") or {}
    return str(event.get("kind", "")).replace("_", "-")


def timed_out(envelope) -> bool:
    error = envelope.error or {}
    return (not envelope.ok) and error.get("code") == "TIMEOUT"


def root_windows(tier, owner: str) -> int:
    try:
        return len(app_root(tier, owner).children())
    except LookupError:
        return 0


def _names(tier, owner):
    return sorted(node.name for node in registry(tier) if node.name == owner)


def _restore_count(ctx, owner, before, app_name):
    replacement = launch(ctx.tier, app_name)
    try:
        ctx.restored("atspi.root_children", changed(lambda: root_windows(ctx.tier, owner), before))
    finally:
        stop(replacement)


def _editor(tier):
    app = launch(tier, TEXT_EDITOR)
    return app, app_root(tier, "gnome-text-editor")


def _new_editor_window(tier):
    return launch(tier, TEXT_EDITOR, ("--new-window",))


def _calculator(tier):
    app = launch(tier, CALCULATOR)
    return app, app_root(tier, "gnome-calculator")


def _wait(ctx: LegContext, token: str, *extra: str):
    return ctx.product(["wait", "--event", token, "--timeout", "8000", *extra])


def _assert_transition(ctx: LegContext, token: str, before: int, after: int) -> None:
    ctx.after("atspi.root_children", after)
    assert after > before, f"{token} fired but the root children did not grow ({before} -> {after})"


def window_opened_transition(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    existing = _window_ids(ctx.tier)
    ctx.before("atspi.root_children", before)
    trigger = delayed(0.8, lambda: _new_editor_window(ctx.tier))
    envelope = _wait(ctx, "window-opened")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "window-opened", envelope.raw
    _assert_transition(ctx, "window-opened", before, root_windows(ctx.tier, "gnome-text-editor"))
    _close_added_windows(ctx.tier, existing)
    ctx.restored("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.window_opened_quiet")
def window_opened_quiet(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    envelope = ctx.product(["wait", "--event", "window-opened", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    after = root_windows(ctx.tier, "gnome-text-editor")
    ctx.after("atspi.root_children", after)
    assert after == before
    ctx.restored("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.window_closed_transition")
def window_closed_transition(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    trigger = delayed(0.8, lambda: stop(app))
    envelope = _wait(ctx, "window-closed")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "window-closed", envelope.raw
    remaining = changed(lambda: root_windows(ctx.tier, "gnome-text-editor"), 0)
    ctx.after("atspi.root_children", remaining)
    _restore_count(ctx, "gnome-text-editor", before, TEXT_EDITOR)


@implementation("events.window_closed_quiet")
def window_closed_quiet(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    envelope = ctx.product(["wait", "--event", "window-closed", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    ctx.restored("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.app_launched_transition")
def app_launched_transition(ctx: LegContext) -> None:
    ctx.before("proc.apps", _names(ctx.tier, "gnome-calculator"))
    launched = []
    trigger = delayed(0.5, lambda: launched.append(launch(ctx.tier, CALCULATOR)))
    envelope = _wait(ctx, "app-launched")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "app-launched", envelope.raw
    ctx.after("proc.apps", changed(lambda: _names(ctx.tier, "gnome-calculator"), ["gnome-calculator"]))
    stop(launched[0])
    ctx.restored("proc.apps", changed(lambda: _names(ctx.tier, "gnome-calculator"), []))


@implementation("events.app_launched_quiet")
def app_launched_quiet(ctx: LegContext) -> None:
    ctx.before("proc.apps", _names(ctx.tier, "gnome-calculator"))
    envelope = ctx.product(["wait", "--event", "app-launched", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("proc.apps", _names(ctx.tier, "gnome-calculator"))
    ctx.restored("proc.apps", _names(ctx.tier, "gnome-calculator"))


@implementation("events.app_terminated_transition")
def app_terminated_transition(ctx: LegContext) -> None:
    app, _root = _calculator(ctx.tier)
    ctx.before("proc.apps", _names(ctx.tier, "gnome-calculator"))
    trigger = delayed(0.8, lambda: stop(app))
    envelope = _wait(ctx, "app-terminated")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "app-terminated", envelope.raw
    ctx.after("proc.apps", changed(lambda: _names(ctx.tier, "gnome-calculator"), []))
    replacement = launch(ctx.tier, CALCULATOR)
    try:
        ctx.restored("proc.apps", changed(lambda: _names(ctx.tier, "gnome-calculator"), ["gnome-calculator"]))
    finally:
        stop(replacement)


@implementation("events.app_terminated_quiet")
def app_terminated_quiet(ctx: LegContext) -> None:
    app, _root = _calculator(ctx.tier)
    ctx.before("proc.apps", _names(ctx.tier, "gnome-calculator"))
    envelope = ctx.product(["wait", "--event", "app-terminated", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("proc.apps", _names(ctx.tier, "gnome-calculator"))
    ctx.restored("proc.apps", _names(ctx.tier, "gnome-calculator"))
    stop(app)


def _open_about(tier) -> None:
    assert _click_named(tier, "gnome-calculator", "Main Menu"), "Calculator Main Menu missing"
    previous = [-1]
    def settled():
        items = app_root(tier, "gnome-calculator").find(role=35)
        count = len(items)
        ready = count > 0 and previous[0] == count
        previous[0] = count
        return items if ready else None
    items = poll(settled, 4)
    assert items, "Calculator popover items did not settle"
    assert items[-1].do_action("click"), "Calculator About item did not activate"
    assert poll(lambda: any(node.role == 16 and "SHOWING" in node.states
                           for node in app_root(tier, "gnome-calculator").walk()), 4), "About dialog did not show"


def _appeared_about(ctx: LegContext, token: str) -> None:
    app, root = _calculator(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    trigger = delayed(0.6, lambda: _open_about(ctx.tier))
    envelope = _wait(ctx, token)
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == token, envelope.raw
    _assert_transition(ctx, token, before, root_windows(ctx.tier, "gnome-calculator"))
    _dismiss_about(ctx.tier)
    ctx.restored("atspi.root_children", changed(lambda: root_windows(ctx.tier, "gnome-calculator"), before))
    stop(app)


def surface_appeared_transition(ctx: LegContext) -> None:
    _appeared_about(ctx, "surface-appeared")


def surface_appeared_quiet(ctx: LegContext) -> None:
    app, root = _calculator(ctx.tier)
    ctx.before("atspi.root_children", len(root.children()))
    envelope = ctx.product(["wait", "--event", "surface-appeared", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("atspi.root_children", len(root.children()))
    stop(app)


def _dismiss_about(tier) -> None:
    dialogs = [node for node in app_root(tier, "gnome-calculator").walk()
               if node.role == 16 and "SHOWING" in node.states]
    assert len(dialogs) == 1, "one showing About dialog required"
    close = dialogs[0].find(role=43, name="Close")
    assert len(close) == 1 and close[0].do_action("click"), "About did not close"


def surface_dismissed_transition(ctx: LegContext) -> None:
    app, root = _calculator(ctx.tier)
    _open_about(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    trigger = delayed(0.6, lambda: _dismiss_about(ctx.tier))
    envelope = _wait(ctx, "surface-dismissed")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "surface-dismissed", envelope.raw
    after = changed(lambda: root_windows(ctx.tier, "gnome-calculator"), before - 1)
    ctx.after("atspi.root_children", after)
    _open_about(ctx.tier)
    ctx.restored("atspi.root_children", changed(lambda: root_windows(ctx.tier, "gnome-calculator"), before))
    stop(app)


def surface_dismissed_quiet(ctx: LegContext) -> None:
    app, root = _calculator(ctx.tier)
    ctx.before("atspi.root_children", len(root.children()))
    envelope = ctx.product(["wait", "--event", "surface-dismissed", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("atspi.root_children", len(root.children()))
    stop(app)


def _window_id(ctx: LegContext, app: str) -> str:
    envelope = ctx.product(["list-windows", "--app", app])
    assert envelope.ok and envelope.data, envelope.raw
    return envelope.data[0]["id"]


@implementation("events.focus_changed_quiet")
def focus_changed_quiet(ctx: LegContext) -> None:
    editor, _root = _editor(ctx.tier)
    before = active_window(ctx.tier)
    ctx.before("x11.active_window", before)
    envelope = ctx.product(["wait", "--event", "focus-changed", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("x11.active_window", active_window(ctx.tier))
    ctx.restored("x11.active_window", active_window(ctx.tier))
    stop(editor)


@implementation("events.scoped_and_unscoped_forms")
def scoped_and_unscoped_forms(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    existing = _window_ids(ctx.tier)
    ctx.before("atspi.root_children", before)
    scoped = delayed(0.6, lambda: _new_editor_window(ctx.tier))
    first = _wait(ctx, "window-opened", "--app", TEXT_EDITOR)
    scoped.join()
    assert first.ok, first.raw
    unscoped = delayed(0.6, lambda: _new_editor_window(ctx.tier))
    second = _wait(ctx, "window-opened")
    unscoped.join()
    assert second.ok, second.raw
    ctx.after("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    _close_added_windows(ctx.tier, existing)
    ctx.restored("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.open_and_close_inside_one_wait")
def open_and_close_inside_one_wait(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    ctx.before("atspi.root_children", len(root.children()))

    existing = _window_ids(ctx.tier)
    def cycle() -> None:
        _new_editor_window(ctx.tier)
        _close_added_windows(ctx.tier, existing)

    trigger = delayed(0.6, cycle)
    envelope = _wait(ctx, "window-closed")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "window-closed", envelope.raw
    ctx.after("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    ctx.restored("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.target_killed_mid_wait")
def target_killed_mid_wait(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    read = lambda: _names(ctx.tier, "gnome-text-editor")
    before = read()
    ctx.before("proc.text_editor", before)
    trigger = delayed(0.5, lambda: stop(app))
    envelope = ctx.product(
        ["wait", "--event", "window-closed", "--app", TEXT_EDITOR, "--timeout", "8000"]
    )
    trigger.join()
    if envelope.ok:
        assert envelope.data, envelope.raw
        assert envelope.data.get("target_unresolved") is True or envelope.data.get("found") is True
    else:
        assert timed_out(envelope), envelope.raw
    ctx.after("proc.text_editor", changed(read, []))
    replacement = launch(ctx.tier, TEXT_EDITOR)
    try:
        ctx.restored("proc.text_editor", changed(read, before))
    finally:
        stop(replacement)
