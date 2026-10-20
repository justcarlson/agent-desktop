from __future__ import annotations

import os
import re
import signal
import subprocess
import sys
from pathlib import Path

from apps.sni_item import read_log
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root, poll

SENDER = Path(__file__).resolve().parents[1] / "apps" / "sni_item.py"
SERVICE = "org.agentdesktop.HarnessTray"
TITLE = "Harness Tray"
ACTIVATE_LABEL = "Activate Harness Item"
DISABLED_LABEL = "Disabled Entry"


def parse_quoted(output: str) -> list[str]:
    return re.findall(r"'([^']*)'|\"([^\"]*)\"", output)


def registered_items(output: str) -> list[str]:
    return [first or second for first, second in parse_quoted(output)]


def calls_named(path: Path, name: str) -> list[dict]:
    return [payload for call, payload in read_log(path) if call == name]


def containing(root, text: str) -> list[str]:
    return [node.name for node in root.find() if text.lower() in node.name.lower()]


def _start(ctx: LegContext, *flags: str):
    log = Path(ctx.tier.runtime_dir) / "tray-calls.log"
    argv = [sys.executable, str(SENDER), "--log", str(log), "--title", TITLE, *flags]
    spawned = subprocess.Popen(
        argv, env=ctx.tier.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    ready = poll(lambda: calls_named(log, "registered") or None, 15.0)
    assert ready, f"the sender never registered (log: {log})"
    return spawned, log


def _stop(spawned) -> None:
    try:
        os.kill(spawned.pid, signal.SIGTERM)
        spawned.wait(timeout=10)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        return


def _watcher_items(ctx: LegContext) -> list[str]:
    completed = subprocess.run(
        [
            "gdbus",
            "call",
            "--session",
            "--dest",
            "org.kde.StatusNotifierWatcher",
            "--object-path",
            "/StatusNotifierWatcher",
            "--method",
            "org.freedesktop.DBus.Properties.Get",
            "org.kde.StatusNotifierWatcher",
            "RegisteredStatusNotifierItems",
        ],
        env=ctx.tier.env,
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )
    return registered_items(completed.stdout)


def _ref(ctx: LegContext, name: str) -> str:
    envelope = ctx.product(["snapshot", "--surface", "system-tray", "-i"])
    assert envelope.ok, envelope.raw
    stack = [(envelope.data or {}).get("tree")]
    while stack:
        node = stack.pop()
        if not node:
            continue
        if node.get("ref") and node.get("name") == name:
            return node["ref"]
        stack.extend(node.get("children") or [])
    raise AssertionError(f"the tray snapshot has no ref named {name!r}: {envelope.raw}")


@implementation("tray.sender_registered_before_first_leg")
def sender_registered_before_first_leg(ctx: LegContext) -> None:
    ctx.before("dbus.tray_items", [])
    spawned, _log = _start(ctx)
    try:
        observed = poll(lambda: _watcher_items(ctx) or None, 15.0)
        ctx.after("dbus.tray_items", observed)
        assert observed, "the watcher lists no item"
        assert any(SERVICE in item for item in observed), observed
    finally:
        _stop(spawned)


@implementation("tray.headless_activate_logged")
def headless_activate_logged(ctx: LegContext) -> None:
    spawned, log = _start(ctx)
    try:
        ref = _ref(ctx, TITLE)
        ctx.before("applog.activations", len(calls_named(log, "Activate")))
        envelope = ctx.product(["click", ref])
        assert envelope.ok, envelope.raw
        activations = poll(lambda: calls_named(log, "Activate") or None, 5.0)
        ctx.after("applog.activations", len(calls_named(log, "Activate")))
        assert activations, "no Activate reached the sender"
        assert len(activations) == 1, activations
    finally:
        _stop(spawned)


@implementation("tray.headless_menu_event_clicked")
def headless_menu_event_clicked(ctx: LegContext) -> None:
    spawned, log = _start(ctx)
    try:
        ref = _ref(ctx, ACTIVATE_LABEL)
        ctx.before("applog.events", [])
        envelope = ctx.product(["click", ref])
        assert envelope.ok, envelope.raw
        events = poll(lambda: calls_named(log, "Event") or None, 5.0)
        ctx.after("applog.events", calls_named(log, "Event"))
        assert events, "no Event reached the sender"
        assert events[0].get("id") == 1 and events[0].get("event") == "clicked", events
    finally:
        _stop(spawned)


@implementation("tray.disabled_entry_refused")
def disabled_entry_refused(ctx: LegContext) -> None:
    spawned, log = _start(ctx, "--disabled-entry")
    try:
        ref = _ref(ctx, DISABLED_LABEL)
        ctx.before("applog.events", [])
        envelope = ctx.product(["click", ref])
        events = calls_named(log, "Event")
        ctx.after("applog.events", events)
        assert not envelope.ok, envelope.raw
        assert (envelope.error or {}).get("code") in ("ACTION_NOT_SUPPORTED", "ACTION_FAILED")
        assert not events, events
    finally:
        _stop(spawned)


@implementation("tray.item_without_activate_refused")
def item_without_activate_refused(ctx: LegContext) -> None:
    spawned, log = _start(ctx, "--no-activate")
    try:
        ref = _ref(ctx, TITLE)
        ctx.before("applog.activations", [])
        envelope = ctx.product(["click", ref])
        activations = calls_named(log, "Activate")
        ctx.after("applog.activations", activations)
        assert not envelope.ok, envelope.raw
        assert (envelope.error or {}).get("code") == "ACTION_NOT_SUPPORTED", envelope.raw
        assert not activations, activations
    finally:
        _stop(spawned)


@implementation("tray.killed_owner_stale_ref")
def killed_owner_stale_ref(ctx: LegContext) -> None:
    spawned, log = _start(ctx)
    ref = _ref(ctx, TITLE)
    ctx.before("applog.owner", spawned.pid)
    _stop(spawned)
    envelope = ctx.product(["click", ref])
    ctx.after("applog.owner", None)
    assert not envelope.ok, envelope.raw
    assert (envelope.error or {}).get("code") == "STALE_REF", envelope.raw
    assert not calls_named(log, "Activate")


@implementation("tray.headed_click_opens_shell_popup")
def headed_click_opens_shell_popup(ctx: LegContext) -> None:
    spawned, log = _start(ctx, "--no-activate")
    try:
        ref = _ref(ctx, TITLE)
        ctx.before("atspi.shell_menu", False)
        envelope = ctx.product(["click", ref], headed=True)
        assert envelope.ok, envelope.raw
        popup = poll(lambda: containing(app_root(ctx.tier, "gnome-shell"), TITLE), 4.0)
        ctx.after("atspi.shell_menu", bool(popup))
        assert popup, "the shell showed no popup named by the item"
        assert not calls_named(log, "Activate")
    finally:
        _stop(spawned)
