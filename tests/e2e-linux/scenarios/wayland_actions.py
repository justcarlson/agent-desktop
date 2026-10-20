from __future__ import annotations

import subprocess
from pathlib import Path

from runner.apps import launch, stop
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root

CALCULATOR = "Calculator"
REPO_ROOT = Path(__file__).resolve().parents[2]


def walk(node) -> list:
    nodes = [node]
    for child in node.children():
        nodes.extend(walk(child))
    return nodes


def shows(root, needle: str) -> bool:
    for node in walk(root):
        if getattr(node, "name", "") == needle:
            return True
        text = node.text()
        if text is not None and text.strip() == needle:
            return True
    return False


def search(value, needle: str) -> bool:
    target = needle.lower()
    if isinstance(value, str):
        return target in value.lower()
    if isinstance(value, dict):
        return any(search(key, needle) or search(item, needle) for key, item in value.items())
    if isinstance(value, (list, tuple)):
        return any(search(item, needle) for item in value)
    return False


def code_of(envelope) -> str:
    return str((envelope.error or {}).get("code", ""))


def named_ref(tree: dict | None, name: str) -> str | None:
    stack = [tree] if tree else []
    while stack:
        node = stack.pop()
        if node.get("ref") and node.get("name") == name:
            return node["ref"]
        stack.extend(node.get("children") or [])
    return None


def _error_text(envelope) -> dict:
    return {"message": (envelope.error or {}).get("message", ""), "details": (envelope.error or {}).get("details")}


@implementation("wayland_actions.semantic_action_no_input_device")
def semantic_action_no_input_device(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    snapshot = ctx.product(["snapshot", "--app", CALCULATOR, "-i"])
    assert snapshot.ok, snapshot.raw
    ref = named_ref((snapshot.data or {}).get("tree"), "7")
    assert ref, "Calculator has no 7 button ref"
    ctx.before("atspi.display", "")
    envelope = ctx.product(["click", ref])
    assert envelope.ok, envelope.raw
    assert shows(app_root(ctx.tier, "gnome-calculator"), "7"), "the display did not change"
    ctx.after("atspi.display", "7")
    stop(app)


@implementation("wayland_actions.seat_without_keyboard_reported_as_seat")
def seat_without_keyboard_reported_as_seat(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    ctx.before("atspi.display", "")
    envelope = ctx.product(["press", "a"])
    ctx.after("atspi.display", "")
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "ACTION_NOT_SUPPORTED", envelope.raw
    assert search(_error_text(envelope), "seat") or search(_error_text(envelope), "keyboard"), envelope.raw
    stop(app)


@implementation("wayland_actions.hit_test_unknown")
def hit_test_unknown(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    ctx.before("atspi.display", "")
    envelope = ctx.product(["mouse-click", "--x", "20", "--y", "20"], headed=True)
    ctx.after("atspi.display", "")
    assert not envelope.ok, envelope.raw
    assert search(_error_text(envelope), "hit_test") or search(_error_text(envelope), "unknown"), envelope.raw
    stop(app)


@implementation("wayland_actions.focus_window_wayland_session_refusal")
def focus_window_wayland_session_refusal(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    windows = ctx.product(["list-windows", "--app", CALCULATOR])
    assert windows.ok and windows.data, windows.raw
    window_id = windows.data[0]["id"]
    ctx.before("x11.active_window", None)
    envelope = ctx.product(["focus-window", "--window-id", window_id])
    ctx.after("x11.active_window", None)
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "ACTION_NOT_SUPPORTED", envelope.raw
    assert search(_error_text(envelope), "wayland_session"), envelope.raw
    stop(app)


@implementation("wayland_actions.list_windows_no_position")
def list_windows_no_position(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    ctx.before("x11.window_bounds", "absent")
    envelope = ctx.product(["list-windows", "--app", CALCULATOR])
    assert envelope.ok and envelope.data, envelope.raw
    positioned = [item for item in envelope.data if item.get("bounds")]
    ctx.after("x11.window_bounds", "absent")
    assert not positioned, positioned
    stop(app)


@implementation("wayland_actions.unhealthy_shell_named_reason")
def unhealthy_shell_named_reason(ctx: LegContext) -> None:
    script = REPO_ROOT / "scripts" / "private-wayland-shell.sh"
    env = dict(ctx.tier.env)
    env.pop("AGENT_DESKTOP_PRIVATE_SESSION", None)
    ctx.before("session.tier", "wayland")
    completed = subprocess.run(
        ["bash", str(script), "--", "true"],
        env=env,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    ctx.after("session.tier", completed.returncode)
    assert completed.returncode != 0, completed.stdout
    output = (completed.stdout + completed.stderr).lower()
    assert "private-session" in output or "private-wayland-shell" in output, output
