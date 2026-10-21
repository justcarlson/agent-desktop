from __future__ import annotations

import os
import subprocess

from oracle import atspi_raw, proc, x11
from runner import apps
from runner.helper import changed
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import assert_ok, window_of
from scenarios.pointer import delivery_fields, error_delivery, frame_extents


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
    process = _run(tier, ["xprop", "-id", x11.xid(tier, window), "_NET_WM_STATE"])
    return process.stdout


def _alive(tier, pid: int) -> bool:
    return pid in member_pids(proc.members(tier.runtime_dir))


def _window_id(app) -> str:
    return window_of(app)


def _geometry(ctx: LegContext, window: str) -> tuple[int, int, int, int]:
    x, y, width, height = x11.geometry(ctx.tier, window)
    left, right, top, bottom = frame_extents(ctx.tier, window)
    return x + left, y + top, width - left - right, height - top - bottom


def _close_report(ctx: LegContext, app) -> dict:
    envelope = ctx.product(["close-app", app.name])
    data = envelope.data or {}
    return delivery_fields(data) if envelope.ok else error_delivery(envelope)


def _restore_geometry(ctx, window, before):
    x, y, width, height = before
    assert_ok(ctx.product(["resize-window", "--window-id", window,
                           "--width", str(width), "--height", str(height)]))
    assert_ok(ctx.product(["move-window", "--window-id", window,
                           "--x", str(x), "--y", str(y)]))
    ctx.restored("x11.geometry", changed(lambda: _geometry(ctx, window), before, seconds=6))


@implementation("lifecycle.close_app_undelivered_report")
def close_app_undelivered_report(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Calculator")
    pid = app.pid
    os.kill(pid, 19)
    try:
        ctx.before("proc.alive", _alive(ctx.tier, pid))
        report = _close_report(ctx, app)
        ctx.after("proc.alive", _alive(ctx.tier, pid))
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
        native = x11.xid(ctx.tier, target)
        assert_ok(ctx.product(["press", "7", "--window-id", _window_id(editor)]))
        before = x11.active_window(ctx.tier)
        ctx.before("x11.active_window", before)
        assert_ok(ctx.product(["focus-window", "--window-id", target]))
        active = atspi_raw.poll(lambda: x11.active_window(ctx.tier)
                               if x11.active_window(ctx.tier) == native else None, 6.0)
        ctx.after("x11.active_window", active)
        if active != native:
            raise AssertionError("focus-window did not make the target active")
        ctx.restored("x11.active_window", x11.restore_active(ctx.tier, before))
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
        _restore_geometry(ctx, window, before)
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
        _restore_geometry(ctx, window, before)
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
            changed(lambda: is_hidden(_wm_state(ctx.tier, window)), False, seconds=6),
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
            changed(lambda: is_maximised(_wm_state(ctx.tier, window)), False, seconds=6),
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
        ctx.after("x11.geometry", _geometry(ctx, window))
        if moved_report["delivery"] == "not_delivered":
            raise AssertionError("a moved window was reported not delivered")
        missing = ctx.product(["resize-window", "--window-id", "w-missing", "--width", "100", "--height", "100"])
        if missing.ok:
            raise AssertionError("resizing a missing window succeeded")
        if error_delivery(missing)["delivery"] != "not_delivered":
            raise AssertionError("a missing window was not reported as not delivered")
        _restore_geometry(ctx, window, before)
    finally:
        apps.stop(app)


@implementation("lifecycle.openbox_lift_restored_and_active")
def openbox_lift_restored_and_active(ctx: LegContext) -> None:
    calculator = apps.launch(ctx.tier, "Calculator")
    editor = apps.launch(ctx.tier, "Text Editor")
    try:
        target = _window_id(calculator)
        native = x11.xid(ctx.tier, target)
        before = x11.active_window(ctx.tier)
        ctx.before("x11.active_window", before)
        assert_ok(ctx.product(["focus-window", "--window-id", target]))
        active = atspi_raw.poll(
            lambda: x11.active_window(ctx.tier) if x11.active_window(ctx.tier) == native else None, 6.0
        )
        ctx.after("x11.active_window", active)
        if active != native:
            raise AssertionError("the lift did not end with the target active")
        ctx.restored("x11.active_window", x11.restore_active(ctx.tier, before))
    finally:
        apps.stop(editor)
        apps.stop(calculator)
