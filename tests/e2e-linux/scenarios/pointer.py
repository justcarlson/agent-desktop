from __future__ import annotations

import re
import subprocess

from oracle import atspi_raw, x11
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import assert_ok, live_root, raw_names, ref_for, window_of

POINTER_TOLERANCE = 2
DISPLAY_RE = re.compile(r"^\d+$")
FRAME_EXTENTS_RE = re.compile(r"-?\d+")


def assert_error(envelope, codes) -> dict:
    if envelope.ok:
        raise AssertionError(f"{envelope.argv} unexpectedly succeeded")
    error = envelope.error or {}
    if error.get("code") not in codes:
        raise AssertionError(f"{envelope.argv} answered {error.get('code')!r}")
    return error


def parse_frame_extents(text: str) -> tuple[int, int, int, int]:
    values = [int(value) for value in FRAME_EXTENTS_RE.findall(text)]
    if len(values) < 4:
        return (0, 0, 0, 0)
    return (values[0], values[1], values[2], values[3])


def expected_point(
    origin: tuple[int, int],
    frame: tuple[int, int, int, int],
    extents: tuple[int, int, int, int],
) -> tuple[int, int]:
    x, y, width, height = extents
    return (origin[0] + frame[0] + x + width // 2, origin[1] + frame[2] + y + height // 2)


def activation_count(text: str) -> int:
    return sum(1 for line in text.splitlines() if line.strip())


def within(actual: tuple[int, int], expected: tuple[int, int], tolerance: int = POINTER_TOLERANCE) -> bool:
    return abs(actual[0] - expected[0]) <= tolerance and abs(actual[1] - expected[1]) <= tolerance


def delivery_fields(data: dict) -> dict:
    disposition = data.get("disposition") or {}
    delivery = disposition.get("delivery")
    if delivery not in ("delivered_verified", "delivered_unverified", "not_delivered"):
        raise AssertionError(f"delivery report carries {delivery!r}")
    details = data.get("details") or {}
    return {
        "delivery": delivery,
        "retry": disposition.get("retry"),
        "events": details.get("delivered_events"),
        "steps": data.get("steps") or [],
    }


def error_delivery(envelope) -> dict:
    error = envelope.error or {}
    disposition = error.get("disposition") or {}
    delivery = disposition.get("delivery")
    if delivery not in ("not_delivered", "delivered_unverified", "delivered_verified"):
        raise AssertionError(f"error report carries {delivery!r}")
    return {"delivery": delivery, "retry": disposition.get("retry")}


def _xprop(tier, window: str, name: str) -> str:
    process = subprocess.run(
        ["xprop", "-id", window, name],
        capture_output=True,
        text=True,
        env=tier.env,
        timeout=5,
        check=False,
    )
    return process.stdout.strip()


def frame_extents(tier, window: str) -> tuple[int, int, int, int]:
    return parse_frame_extents(_xprop(tier, window, "_GTK_FRAME_EXTENTS"))


def _window_id(app) -> str:
    return window_of(app)


def _display_weight(root) -> int:
    return sum(len(name) for name in raw_names(root) if DISPLAY_RE.match(name or ""))


def _terminal_tabs(root) -> int:
    return sum(1 for name in raw_names(root) if name == "Terminal")


def _log_count(tier) -> int:
    path = tier.runtime_dir / "qt-window.log"
    if not path.exists():
        return 0
    return activation_count(path.read_text(encoding="utf-8", errors="replace"))


def _targets(ctx: LegContext) -> list[dict]:
    calculator = apps.launch(ctx.tier, "Calculator")
    terminal = apps.launch(ctx.tier, "Terminal")
    assert_ok(ctx.product(["press", "ctrl+shift+t", "--window-id", window_of(terminal)]))
    atspi_raw.poll(lambda: _terminal_tabs(live_root(ctx.tier, terminal)) >= 2, 6.0)
    qt = apps.launch_qt(ctx.tier)
    return [
        {
            "name": "gtk4",
            "app": calculator,
            "button": "7",
            "window": window_of(calculator),
            "effect": lambda: _display_weight(live_root(ctx.tier, calculator)),
        },
        {
            "name": "gtk3",
            "app": terminal,
            "button": "New Tab",
            "window": window_of(terminal),
            "effect": lambda: _terminal_tabs(live_root(ctx.tier, terminal)),
        },
        {
            "name": "qt",
            "app": qt,
            "button": "Press me",
            "window": window_of(qt),
            "effect": lambda: _log_count(ctx.tier),
        },
    ]


def _stop_all(targets) -> None:
    for target in targets:
        apps.stop(target["app"])


def _button_point(ctx: LegContext, target: dict) -> tuple[int, int]:
    root = live_root(ctx.tier, target["app"])
    nodes = [node for node in _flat(root) if node.name == target["button"]]
    if not nodes:
        raise AssertionError(f"{target['name']}: button {target['button']!r} is absent")
    extents = nodes[0].extents(window_coords=True)
    origin = x11.client_origin(ctx.tier, target["window"])
    return expected_point(origin, frame_extents(ctx.tier, target["window"]), extents)


def _scroll_ref(ctx: LegContext, window: str) -> str:
    for role in ("listbox", "scrollarea", "terminal", "text"):
        try:
            return ref_for(ctx, window, role=role, deadline_s=2.0)
        except AssertionError:
            continue
    return ref_for(ctx, window, role="group")


def _flat(node) -> list:
    collected = [node]
    for child in node.children():
        collected.extend(_flat(child))
    return collected


def _product_for_verb(ctx: LegContext, target: dict, verb: str, point: tuple[int, int]) -> object:
    window = target["window"]
    ref = ref_for(ctx, window, target["button"])
    coordinates = f"{point[0]},{point[1]}"
    if verb in ("click", "double-click", "triple-click", "right-click"):
        return ctx.product([verb, ref])
    if verb == "hover":
        return ctx.product(["hover", "--xy", coordinates])
    if verb == "mouse-move":
        return ctx.product(["mouse-move", "--xy", coordinates])
    if verb == "mouse-click":
        return ctx.product(["mouse-click", "--xy", coordinates])
    if verb == "mouse-wheel":
        return ctx.product(["mouse-wheel", "--x", str(point[0]), "--y", str(point[1]), "--dy", "-3"])
    if verb == "drag":
        return ctx.product(["drag", "--from-xy", coordinates, "--to-xy", f"{point[0] + 20},{point[1]}"])
    if verb == "scroll":
        return ctx.product(["scroll", _scroll_ref(ctx, window), "--direction", "down", "--amount", "3"])
    raise AssertionError(f"unknown verb {verb}")


def _gesture(ctx: LegContext, verb: str, clicks: int = 1, expect_effect: bool = True) -> None:
    targets = _targets(ctx)
    try:
        for target in targets:
            point = _button_point(ctx, target)
            before_effect = target["effect"]()
            before_pointer = x11.pointer(ctx.tier)
            ctx.before(f"x11.pointer.{target['name']}", before_pointer)
            ctx.before(f"{target['name']}.effect", before_effect)
            envelope = _product_for_verb(ctx, target, verb, point)
            if not envelope.ok and verb == "scroll" and target["name"] == "qt":
                assert_error(envelope, {"ACTION_NOT_SUPPORTED"})
                ctx.after(f"{target['name']}.effect", target["effect"]())
                continue
            assert_ok(envelope)
            pointer = ctx.after(f"x11.pointer.{target['name']}", x11.pointer(ctx.tier))
            destination = (point[0] + 20, point[1]) if verb == "drag" else point
            if verb not in ("mouse-wheel", "scroll") and not within(pointer, destination):
                raise AssertionError(f"{target['name']}: pointer {pointer} missed {destination}")
            if expect_effect:
                expected = before_effect + clicks
                after = atspi_raw.poll(
                    lambda: target["effect"]() if target["effect"]() >= expected else None, 6.0
                )
                ctx.after(f"{target['name']}.effect", after)
                if after is None:
                    raise AssertionError(f"{target['name']}: {verb} left no application effect")
    finally:
        _stop_all(targets)


@implementation("pointer.click")
def click(ctx: LegContext) -> None:
    _gesture(ctx, "click")


@implementation("pointer.double_click")
def double_click(ctx: LegContext) -> None:
    _gesture(ctx, "double-click", clicks=2)


@implementation("pointer.triple_click")
def triple_click(ctx: LegContext) -> None:
    _gesture(ctx, "triple-click", clicks=3)


@implementation("pointer.right_click")
def right_click(ctx: LegContext) -> None:
    _gesture(ctx, "right-click", expect_effect=False)


@implementation("pointer.hover")
def hover(ctx: LegContext) -> None:
    _gesture(ctx, "hover", expect_effect=False)


@implementation("pointer.drag")
def drag(ctx: LegContext) -> None:
    _gesture(ctx, "drag", expect_effect=False)


@implementation("pointer.mouse_move")
def mouse_move(ctx: LegContext) -> None:
    _gesture(ctx, "mouse-move", expect_effect=False)


@implementation("pointer.mouse_click")
def mouse_click(ctx: LegContext) -> None:
    _gesture(ctx, "mouse-click")


@implementation("pointer.mouse_wheel")
def mouse_wheel(ctx: LegContext) -> None:
    _gesture(ctx, "mouse-wheel", expect_effect=False)


@implementation("pointer.scroll")
def scroll(ctx: LegContext) -> None:
    _gesture(ctx, "scroll", expect_effect=False)


def _extents_at(ctx: LegContext, target: dict, offset: int) -> None:
    window = target["window"]
    geometry = x11.geometry(ctx.tier, window)
    assert_ok(ctx.product(["move-window", "--window-id", window, "--x", str(geometry[0] + offset), "--y", str(geometry[1] + offset)]))
    atspi_raw.poll(lambda: x11.geometry(ctx.tier, window)[0] == geometry[0] + offset, 6.0)
    point = _button_point(ctx, target)
    ctx.before(f"x11.pointer.{target['name']}", x11.pointer(ctx.tier))
    assert_ok(ctx.product(["mouse-move", "--xy", f"{point[0]},{point[1]}"]))
    pointer = ctx.after(f"x11.pointer.{target['name']}", x11.pointer(ctx.tier))
    if not within(pointer, point):
        raise AssertionError(f"{target['name']}: expected point {point}, pointer read {pointer}")


def _extents_leg(ctx: LegContext, wanted: str) -> None:
    targets = _targets(ctx)
    try:
        for target in targets:
            if target["name"] != wanted:
                continue
            _extents_at(ctx, target, 0)
            _extents_at(ctx, target, 60)
    finally:
        _stop_all(targets)


@implementation("pointer.gtk3_extents_origin")
def gtk3_extents_origin(ctx: LegContext) -> None:
    _extents_leg(ctx, "gtk3")


@implementation("pointer.gtk4_two_positions")
def gtk4_two_positions(ctx: LegContext) -> None:
    _extents_leg(ctx, "gtk4")


@implementation("pointer.qt_extents_origin")
def qt_extents_origin(ctx: LegContext) -> None:
    _extents_leg(ctx, "qt")


def _delivered_argv(verb: str, point: tuple[int, int]) -> list[str]:
    x, y = point
    if verb == "drag":
        return ["drag", "--from-xy", f"{x},{y}", "--to-xy", f"{x + 40},{y}"]
    if verb == "mouse-wheel":
        return ["mouse-wheel", "--x", str(x), "--y", str(y), "--dy", "-3"]
    return ["mouse-click", "--xy", f"{x},{y}"]


def _refused_argv(verb: str) -> list[str]:
    if verb == "drag":
        return ["drag", "--from-xy", "-40,-40", "--to-xy", "-30,-30"]
    if verb == "mouse-wheel":
        return ["mouse-wheel", "--x", "-40", "--y", "-40"]
    return ["mouse-click", "--xy", "-40,-40"]


def _delivery_leg(ctx: LegContext, verb: str) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        target = {"name": "gtk4", "app": app, "button": "7", "window": window_of(app), "effect": lambda: 0}
        point = _button_point(ctx, target)
        expected = (point[0] + 40, point[1]) if verb == "drag" else point
        ctx.before("x11.pointer", x11.pointer(ctx.tier))
        data = assert_ok(ctx.product(_delivered_argv(verb, point)))
        report = delivery_fields(data)
        ctx.after("disposition.delivery", report["delivery"])
        if report["delivery"] != "delivered_unverified":
            raise AssertionError(f"delivered command reported {report['delivery']}")
        if not report["events"] or not report["steps"]:
            raise AssertionError("delivered command reported no events or no steps")
        pointer = ctx.after("x11.pointer", x11.pointer(ctx.tier))
        if verb != "mouse-wheel" and not within(pointer, expected):
            raise AssertionError(f"pointer {pointer} did not reach {expected}")
        refused = ctx.product(_refused_argv(verb))
        if refused.ok:
            raise AssertionError("a point outside the screen was accepted")
        assert_error(refused, {"INVALID_ARGS"})
        if error_delivery(refused)["delivery"] != "not_delivered":
            raise AssertionError("an out-of-screen point was not reported as not delivered")
    finally:
        apps.stop(app)


@implementation("pointer.drag_delivery_report")
def drag_delivery_report(ctx: LegContext) -> None:
    _delivery_leg(ctx, "drag")


@implementation("pointer.mouse_wheel_delivery_report")
def mouse_wheel_delivery_report(ctx: LegContext) -> None:
    _delivery_leg(ctx, "mouse-wheel")


@implementation("pointer.mouse_click_delivery_report")
def mouse_click_delivery_report(ctx: LegContext) -> None:
    _delivery_leg(ctx, "mouse-click")
