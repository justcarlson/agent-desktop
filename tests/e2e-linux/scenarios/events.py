from __future__ import annotations

from runner.apps import launch, stop
from runner.helper import delayed
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root
from oracle.x11 import active_window

TEXT_EDITOR = "Text Editor"
CALCULATOR = "Calculator"


def event_kind(data: dict | None) -> str:
    event = (data or {}).get("event") or {}
    return str(event.get("kind", "")).replace("_", "-")


def timed_out(envelope) -> bool:
    error = envelope.error or {}
    return (not envelope.ok) and error.get("code") == "TIMEOUT"


def root_windows(tier, owner: str) -> int:
    return len(app_root(tier, owner).children())


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


@implementation("events.window_opened_transition")
def window_opened_transition(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    trigger = delayed(0.8, lambda: _new_editor_window(ctx.tier))
    envelope = _wait(ctx, "window-opened")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "window-opened", envelope.raw
    _assert_transition(ctx, "window-opened", before, root_windows(ctx.tier, "gnome-text-editor"))
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
    remaining = [item for item in app_root(ctx.tier, "gnome-text-editor").children() if item.name]
    ctx.after("atspi.root_children", len(remaining))
    assert not remaining


@implementation("events.window_closed_quiet")
def window_closed_quiet(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    envelope = ctx.product(["wait", "--event", "window-closed", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.app_launched_transition")
def app_launched_transition(ctx: LegContext) -> None:
    ctx.before("proc.apps", [])
    trigger = delayed(0.5, lambda: _calculator(ctx.tier))
    envelope = _wait(ctx, "app-launched")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "app-launched", envelope.raw
    ctx.after("proc.apps", ["gnome-calculator"])


@implementation("events.app_launched_quiet")
def app_launched_quiet(ctx: LegContext) -> None:
    ctx.before("proc.apps", [])
    envelope = ctx.product(["wait", "--event", "app-launched", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("proc.apps", [])


@implementation("events.app_terminated_transition")
def app_terminated_transition(ctx: LegContext) -> None:
    app, _root = _calculator(ctx.tier)
    ctx.before("proc.apps", ["gnome-calculator"])
    trigger = delayed(0.8, lambda: stop(app))
    envelope = _wait(ctx, "app-terminated")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "app-terminated", envelope.raw
    ctx.after("proc.apps", [])


@implementation("events.app_terminated_quiet")
def app_terminated_quiet(ctx: LegContext) -> None:
    app, _root = _calculator(ctx.tier)
    ctx.before("proc.apps", ["gnome-calculator"])
    envelope = ctx.product(["wait", "--event", "app-terminated", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("proc.apps", ["gnome-calculator"])
    stop(app)


def _open_about(tier) -> None:
    button = app_root(tier, "gnome-calculator").find(name="Main Menu")
    assert button, "Calculator Main Menu missing from the raw tree"
    button[0].do_action("click")


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
    stop(app)


@implementation("events.surface_appeared_transition")
def surface_appeared_transition(ctx: LegContext) -> None:
    _appeared_about(ctx, "surface-appeared")


@implementation("events.surface_appeared_quiet")
def surface_appeared_quiet(ctx: LegContext) -> None:
    app, root = _calculator(ctx.tier)
    ctx.before("atspi.root_children", len(root.children()))
    envelope = ctx.product(["wait", "--event", "surface-appeared", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("atspi.root_children", len(root.children()))
    stop(app)


def _dismiss_about(tier) -> None:
    close = app_root(tier, "gnome-calculator").find(name="Close")
    if close:
        close[0].do_action("click")


@implementation("events.surface_dismissed_transition")
def surface_dismissed_transition(ctx: LegContext) -> None:
    app, root = _calculator(ctx.tier)
    _open_about(ctx.tier)
    ctx.before("atspi.root_children", len(root.children()))
    trigger = delayed(0.6, lambda: _dismiss_about(ctx.tier))
    envelope = _wait(ctx, "surface-dismissed")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "surface-dismissed", envelope.raw
    ctx.after("atspi.root_children", root_windows(ctx.tier, "gnome-calculator"))
    stop(app)


@implementation("events.surface_dismissed_quiet")
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


@implementation("events.focus_changed_transition")
def focus_changed_transition(ctx: LegContext) -> None:
    editor, _root = _editor(ctx.tier)
    calculator, _calc_root = _calculator(ctx.tier)
    target = _window_id(ctx, CALCULATOR)
    before = active_window(ctx.tier)
    ctx.before("x11.active_window", before)
    trigger = delayed(0.6, lambda: ctx.product(["focus-window", "--window-id", target]))
    envelope = _wait(ctx, "focus-changed")
    trigger.join()
    assert envelope.ok, envelope.raw
    after = active_window(ctx.tier)
    ctx.after("x11.active_window", after)
    assert after and after != before
    stop(editor)
    stop(calculator)


@implementation("events.focus_changed_quiet")
def focus_changed_quiet(ctx: LegContext) -> None:
    editor, _root = _editor(ctx.tier)
    before = active_window(ctx.tier)
    ctx.before("x11.active_window", before)
    envelope = ctx.product(["wait", "--event", "focus-changed", "--timeout", "1500"])
    assert timed_out(envelope), envelope.raw
    ctx.after("x11.active_window", active_window(ctx.tier))
    stop(editor)


@implementation("events.scoped_and_unscoped_forms")
def scoped_and_unscoped_forms(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    before = len(root.children())
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
    stop(app)


@implementation("events.open_and_close_inside_one_wait")
def open_and_close_inside_one_wait(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    ctx.before("atspi.root_children", len(root.children()))

    def cycle() -> None:
        other = _new_editor_window(ctx.tier)
        stop(other)

    trigger = delayed(0.6, cycle)
    envelope = _wait(ctx, "window-closed")
    trigger.join()
    assert envelope.ok, envelope.raw
    assert event_kind(envelope.data) == "window-closed", envelope.raw
    ctx.after("atspi.root_children", root_windows(ctx.tier, "gnome-text-editor"))
    stop(app)


@implementation("events.target_killed_mid_wait")
def target_killed_mid_wait(ctx: LegContext) -> None:
    app, root = _editor(ctx.tier)
    ctx.before("proc.text_editor", "running")
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
    ctx.after("proc.text_editor", "gone")
