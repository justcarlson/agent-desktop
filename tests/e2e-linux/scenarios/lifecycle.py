from __future__ import annotations

import os
import subprocess

from oracle import atspi_raw, gsettings, proc, x11
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import assert_ok, window_of
from scenarios.pointer import delivery_fields, error_delivery

REFUSAL_ACTIVATION_CODES = frozenset(
    {"activation_refused", "activation_not_granted", "overview_open", "frame_not_active"}
)


def _run(tier, argv: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True, env=tier.env, timeout=10, check=False)


def parse_wm_state(text: str) -> set[str]:
    return {token for token in text.replace("=", " ").split() if token.startswith("_NET_WM_STATE_")}


def is_hidden(text: str) -> bool:
    return "_NET_WM_STATE_HIDDEN" in parse_wm_state(text)


def is_maximised(text: str) -> bool:
    states = parse_wm_state(text)
    return "_NET_WM_STATE_MAXIMIZED_VERT" in states or "_NET_WM_STATE_MAXIMIZED_HORZ" in states


def member_pids(members) -> set[int]:
    return {pid for pid, _, _ in members}


def _wm_state(tier, window: str) -> str:
    process = _run(tier, ["xprop", "-id", window, "_NET_WM_STATE"])
    return process.stdout


def _alive(tier, pid: int) -> bool:
    return pid in member_pids(proc.members(tier.runtime_dir))


def _window_id(app) -> str:
    return window_of(app)


def _geometry(ctx: LegContext, window: str) -> tuple[int, int, int, int]:
    return x11.geometry(ctx.tier, window)


def _close_report(ctx: LegContext, app) -> dict:
    envelope = ctx.product(["close-app", app.name])
    data = envelope.data or {}
    return delivery_fields(data) if envelope.ok else error_delivery(envelope)


@implementation("lifecycle.close_app_delivery_report")
def close_app_delivery_report(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    pid = app.pid
    ctx.before("proc.alive", _alive(ctx.tier, pid))
    report = _close_report(ctx, app)
    gone = atspi_raw.poll(lambda: True if not _alive(ctx.tier, pid) else None, 10.0)
    ctx.after("proc.alive", not gone)
    ctx.after("disposition.delivery", report["delivery"])
    if not gone:
        raise AssertionError("close-app reported delivery while the process stayed")
    if report["delivery"] == "not_delivered":
        raise AssertionError("close-app reported not delivered after the process exited")


@implementation("lifecycle.close_app_undelivered_report")
def close_app_undelivered_report(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    pid = app.pid
    os.kill(pid, 19)
    try:
        ctx.before("proc.alive", _alive(ctx.tier, pid))
        report = _close_report(ctx, app)
        ctx.after("proc.alive", _alive(ctx.tier, pid))
        ctx.after("disposition.delivery", report["delivery"])
        if report["delivery"] != "not_delivered":
            raise AssertionError(f"a stopped process was reported {report['delivery']}")
        if not _alive(ctx.tier, pid):
            raise AssertionError("the stopped process exited anyway")
    finally:
        os.kill(pid, 18)
        apps.stop(app)


@implementation("lifecycle.focus_window_active_reads")
def focus_window_active_reads(ctx: LegContext) -> None:
    calculator = apps.launch(ctx.tier, "Calculator")
    editor = apps.launch(ctx.tier, "Text Editor")
    try:
        target = _window_id(calculator)
        assert_ok(ctx.product(["press", "7", "--window-id", _window_id(editor)]))
        ctx.before("x11.active_window", x11.active_window(ctx.tier))
        assert_ok(ctx.product(["focus-window", "--window-id", target]))
        active = atspi_raw.poll(lambda: target if x11.active_window(ctx.tier) == target else None, 6.0)
        ctx.after("x11.active_window", active)
        if active != target:
            raise AssertionError("focus-window did not make the target active")
    finally:
        apps.stop(editor)
        apps.stop(calculator)


@implementation("lifecycle.resize_window")
def resize_window(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = _window_id(app)
        before = _geometry(ctx, window)
        ctx.before("x11.geometry", before)
        assert_ok(ctx.product(["resize-window", "--window-id", window, "--width", str(before[2] + 40), "--height", str(before[3] + 30)]))
        after = atspi_raw.poll(
            lambda: _geometry(ctx, window) if _geometry(ctx, window)[2] > before[2] else None, 6.0
        )
        ctx.after("x11.geometry", after)
        if after is None:
            raise AssertionError("the window did not take the new size")
    finally:
        apps.stop(app)


@implementation("lifecycle.move_window")
def move_window(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = _window_id(app)
        before = _geometry(ctx, window)
        ctx.before("x11.geometry", before)
        assert_ok(ctx.product(["move-window", "--window-id", window, "--x", str(before[0] + 30), "--y", str(before[1] + 30)]))
        after = atspi_raw.poll(
            lambda: _geometry(ctx, window) if _geometry(ctx, window)[0] == before[0] + 30 else None, 6.0
        )
        ctx.after("x11.geometry", after)
        if after is None:
            raise AssertionError("the window did not take the new position")
    finally:
        apps.stop(app)


@implementation("lifecycle.minimize_restore")
def minimize_restore(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = _window_id(app)
        ctx.before("x11.wm_state", is_hidden(_wm_state(ctx.tier, window)))
        assert_ok(ctx.product(["minimize", "--window-id", window]))
        hidden = atspi_raw.poll(
            lambda: True if is_hidden(_wm_state(ctx.tier, window)) else None, 6.0
        )
        ctx.after("x11.wm_state", hidden)
        if not hidden:
            ctx.finding("minimize did not set the hidden state the oracle reads")
            raise AssertionError("the window did not read as hidden")
        assert_ok(ctx.product(["restore", "--window-id", window]))
        ctx.restored(
            "x11.wm_state",
            atspi_raw.poll(lambda: not is_hidden(_wm_state(ctx.tier, window)), 6.0),
        )
    finally:
        apps.stop(app)


@implementation("lifecycle.maximize_restore")
def maximize_restore(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = _window_id(app)
        ctx.before("x11.wm_state", is_maximised(_wm_state(ctx.tier, window)))
        assert_ok(ctx.product(["maximize", "--window-id", window]))
        maximised = atspi_raw.poll(
            lambda: True if is_maximised(_wm_state(ctx.tier, window)) else None, 6.0
        )
        ctx.after("x11.wm_state", maximised)
        if not maximised:
            raise AssertionError("the window did not read as maximised")
        assert_ok(ctx.product(["restore", "--window-id", window]))
        ctx.restored(
            "x11.wm_state",
            atspi_raw.poll(lambda: not is_maximised(_wm_state(ctx.tier, window)), 6.0),
        )
    finally:
        apps.stop(app)


@implementation("lifecycle.window_operation_delivery_reports")
def window_operation_delivery_reports(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = _window_id(app)
        before = _geometry(ctx, window)
        ctx.before("x11.geometry", before)
        moved = assert_ok(ctx.product(["move-window", "--window-id", window, "--x", str(before[0] + 20), "--y", str(before[1] + 20)]))
        moved_report = delivery_fields(moved)
        ctx.after("disposition.delivery", moved_report["delivery"])
        if moved_report["delivery"] == "not_delivered":
            raise AssertionError("a moved window was reported not delivered")
        missing = ctx.product(["resize-window", "--window-id", "w-missing", "--width", "100", "--height", "100"])
        if missing.ok:
            raise AssertionError("resizing a missing window succeeded")
        if error_delivery(missing)["delivery"] != "not_delivered":
            raise AssertionError("a missing window was not reported as not delivered")
    finally:
        apps.stop(app)


@implementation("lifecycle.mutter_lift_outcome_set")
def mutter_lift_outcome_set(ctx: LegContext) -> None:
    calculator = apps.launch(ctx.tier, "Calculator")
    editor = apps.launch(ctx.tier, "Text Editor")
    try:
        target = _window_id(calculator)
        ctx.before("x11.active_window", x11.active_window(ctx.tier))
        assert_ok(ctx.product(["focus-window", "--window-id", target]))
        active = atspi_raw.poll(
            lambda: target if x11.active_window(ctx.tier) == target else None, 2.5
        )
        outcome = active or "target_not_active"
        ctx.after("x11.active_window", outcome)
        if outcome not in (target, "target_not_active"):
            raise AssertionError(f"lift ended in an unmeasured outcome {outcome!r}")
    finally:
        apps.stop(editor)
        apps.stop(calculator)


@implementation("lifecycle.mutter_desktop_switch_read_back")
def mutter_desktop_switch_read_back(ctx: LegContext) -> None:
    gsettings.set(ctx.tier, "org.gnome.mutter", "dynamic-workspaces", "false")
    gsettings.set(ctx.tier, "org.gnome.desktop.wm.preferences", "num-workspaces", "4")
    calculator = apps.launch(ctx.tier, "Calculator")
    try:
        target = _window_id(calculator)
        ctx.before("x11.current_desktop", x11.current_desktop(ctx.tier))
        _run(ctx.tier, ["wmctrl", "-s", "1"])
        switched = atspi_raw.poll(
            lambda: x11.current_desktop(ctx.tier) if x11.current_desktop(ctx.tier) == 1 else None, 6.0
        )
        ctx.after("x11.current_desktop", switched)
        if switched != 1:
            raise AssertionError("the desktop switch was not read back")
        assert_ok(ctx.product(["focus-window", "--window-id", target]))
        ctx.after("x11.active_window", x11.active_window(ctx.tier))
    finally:
        _run(ctx.tier, ["wmctrl", "-s", "0"])
        apps.stop(calculator)


@implementation("lifecycle.mutter_activation_refusal_codes")
def mutter_activation_refusal_codes(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    try:
        window = _window_id(app)
        assert_ok(ctx.product(["press", "super", "--window-id", window]))
        atspi_raw.poll(lambda: True if x11.overview_active(ctx.tier) else None, 6.0)
        ctx.before("x11.overview_active", x11.overview_active(ctx.tier))
        envelope = ctx.product(["focus-window", "--window-id", window])
        if envelope.ok:
            raise AssertionError("focus-window succeeded while the Overview was open")
        details = (envelope.error or {}).get("details") or {}
        cause = details.get("kind") or (envelope.error or {}).get("code")
        ctx.after("activation.cause", cause)
        if cause not in REFUSAL_ACTIVATION_CODES:
            raise AssertionError(f"the activation refusal carried {cause!r}")
    finally:
        assert_ok(ctx.product(["press", "escape"]))
        atspi_raw.poll(lambda: not x11.overview_active(ctx.tier), 6.0)
        apps.stop(app)


@implementation("lifecycle.openbox_lift_restored_and_active")
def openbox_lift_restored_and_active(ctx: LegContext) -> None:
    calculator = apps.launch(ctx.tier, "Calculator")
    editor = apps.launch(ctx.tier, "Text Editor")
    try:
        target = _window_id(calculator)
        ctx.before("x11.active_window", x11.active_window(ctx.tier))
        assert_ok(ctx.product(["focus-window", "--window-id", target]))
        active = atspi_raw.poll(
            lambda: target if x11.active_window(ctx.tier) == target else None, 6.0
        )
        ctx.after("x11.active_window", active)
        if active != target:
            raise AssertionError("the lift did not end with the target active")
    finally:
        apps.stop(editor)
        apps.stop(calculator)
