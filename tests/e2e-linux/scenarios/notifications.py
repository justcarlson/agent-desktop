from __future__ import annotations

import os
import signal
from pathlib import Path

from runner.helper import delayed
from runner.legs import implementation
from runner.verdict import LegContext
from oracle import notify
from oracle.atspi_raw import poll


def listed(envelope) -> list[dict]:
    return list(((envelope.data or {}).get("notifications")) or [])


def code_of(envelope) -> str:
    return str((envelope.error or {}).get("code", ""))


def closes(text: str) -> list[tuple[str, dict]]:
    return [item for item in notify.parse_monitor_text(text) if item[0] == "NotificationClosed"]


def _send(ctx: LegContext, app: str, summary: str, body: str = "", actions=()) -> int:
    return notify.send(ctx.tier, app, summary, body, actions)


def _list(ctx: LegContext, app: str | None = None) -> list[dict]:
    argv = ["list-notifications"]
    if app:
        argv += ["--app", app]
    envelope = ctx.product(argv)
    assert envelope.ok, envelope.raw
    return listed(envelope)


def _await_count(ctx: LegContext, app: str, count: int) -> list[dict]:
    return poll(lambda: _list(ctx, app) if len(_list(ctx, app)) >= count else None, 8.0)


def _monitor(ctx: LegContext) -> tuple[int, Path]:
    path = Path(ctx.tier.runtime_dir) / "notification-signals.log"
    pid = notify.monitor(ctx.tier, path)
    return pid, path


def _stop_monitor(pid: int) -> None:
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        return


def _monitor_text(path: Path) -> str:
    return path.read_text(encoding="utf-8") if path.exists() else ""


@implementation("notifications.list_equals_sender_record")
def list_equals_sender_record(ctx: LegContext) -> None:
    ctx.before("dbus.notifications", [])
    _send(ctx, "HarnessAlpha", "Alpha summary", "first body")
    _send(ctx, "HarnessBeta", "Beta summary", "second body")
    observed = _await_count(ctx, "HarnessAlpha", 1)
    all_listed = _list(ctx)
    ctx.after("dbus.notifications", len(all_listed))
    problems = notify.compare_records(
        [{"app": "HarnessAlpha", "summary": "Alpha summary"}], observed
    )
    assert not problems, "; ".join(problems)
    assert len(all_listed) >= 2, all_listed


@implementation("notifications.app_filter_case_insensitive")
def app_filter_case_insensitive(ctx: LegContext) -> None:
    ctx.before("dbus.notifications", 0)
    _send(ctx, "HarnessCase", "Case summary")
    observed = _await_count(ctx, "harnesscase", 1)
    ctx.after("dbus.notifications", len(observed))
    assert observed and observed[0]["title"] == "Case summary", observed


@implementation("notifications.wait_notification_passes")
def wait_notification_passes(ctx: LegContext) -> None:
    marker = "HarnessWaitMarker"
    ctx.before("dbus.notifications", 0)
    trigger = delayed(0.8, lambda: _send(ctx, "HarnessWait", marker))
    envelope = ctx.product(["wait", "--notification", "--text", marker, "--timeout", "8000"])
    trigger.join()
    assert envelope.ok, envelope.raw
    data = envelope.data or {}
    assert data.get("matched") is True, envelope.raw
    ctx.after("dbus.notifications", len(_list(ctx)))


@implementation("notifications.wait_notification_times_out")
def wait_notification_times_out(ctx: LegContext) -> None:
    marker = "HarnessAbsentMarker"
    ctx.before("dbus.notifications", 0)
    envelope = ctx.product(["wait", "--notification", "--text", marker, "--timeout", "1500"])
    ctx.after("dbus.notifications", 0)
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "TIMEOUT", envelope.raw


@implementation("notifications.headless_mutations_policy_denied")
def headless_mutations_policy_denied(ctx: LegContext) -> None:
    _send(ctx, "HarnessMutate", "Mutation summary")
    _await_count(ctx, "HarnessMutate", 1)
    before = _list(ctx, "HarnessMutate")
    ctx.before("dbus.notifications", len(before))
    envelope = ctx.product(
        ["dismiss-notification", "1", "--expected-app", "HarnessMutate"]
    )
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "POLICY_DENIED", envelope.raw
    after = _list(ctx, "HarnessMutate")
    ctx.after("dbus.notifications", len(after))
    assert len(after) == len(before)


@implementation("notifications.wrong_app_not_found")
def wrong_app_not_found(ctx: LegContext) -> None:
    _send(ctx, "HarnessRight", "Right summary")
    _await_count(ctx, "HarnessRight", 1)
    pid, path = _monitor(ctx)
    ctx.before("dbus.notifications", len(_list(ctx)))
    envelope = ctx.product(["dismiss-notification", "1", "--expected-app", "HarnessWrong"])
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "NOTIFICATION_NOT_FOUND", envelope.raw
    after = _list(ctx)
    ctx.after("dbus.notifications", len(after))
    _stop_monitor(pid)
    assert not closes(_monitor_text(path)), _monitor_text(path)


@implementation("notifications.empty_list_not_found")
def empty_list_not_found(ctx: LegContext) -> None:
    pid, path = _monitor(ctx)
    ctx.before("dbus.notifications", 0)
    envelope = ctx.product(["dismiss-all-notifications"])
    assert not envelope.ok, envelope.raw
    assert code_of(envelope) == "NOTIFICATION_NOT_FOUND", envelope.raw
    ctx.after("dbus.notifications", 0)
    _stop_monitor(pid)
    assert not closes(_monitor_text(path)), _monitor_text(path)


@implementation("notifications.dismiss_one_reason_closed")
def dismiss_one_reason_closed(ctx: LegContext) -> None:
    _send(ctx, "HarnessDismiss", "Dismiss me")
    _await_count(ctx, "HarnessDismiss", 1)
    pid, path = _monitor(ctx)
    ctx.before("dbus.notifications", 1)
    envelope = ctx.product(
        ["dismiss-notification", "1", "--expected-app", "HarnessDismiss"], headed=True
    )
    assert envelope.ok, envelope.raw
    observed = poll(lambda: closes(_monitor_text(path)), 5.0)
    remaining = _list(ctx, "HarnessDismiss")
    ctx.after("dbus.notifications", len(remaining))
    _stop_monitor(pid)
    assert observed, _monitor_text(path)
    assert observed[0][1]["reason"] == 2, observed
    assert not remaining, remaining


@implementation("notifications.dismiss_keeps_bystander")
def dismiss_keeps_bystander(ctx: LegContext) -> None:
    _send(ctx, "HarnessTarget", "Target summary")
    _send(ctx, "HarnessBystander", "Bystander summary")
    _await_count(ctx, "HarnessBystander", 1)
    ctx.before("dbus.notifications", len(_list(ctx)))
    envelope = ctx.product(
        ["dismiss-notification", "1", "--expected-app", "HarnessTarget"], headed=True
    )
    assert envelope.ok, envelope.raw
    survivors = poll(lambda: _list(ctx, "HarnessBystander") or None, 5.0)
    ctx.after("dbus.notifications", len(_list(ctx)))
    assert survivors, "the bystander notification disappeared"


@implementation("notifications.dismiss_all_closes_every_notification")
def dismiss_all_closes_every_notification(ctx: LegContext) -> None:
    _send(ctx, "HarnessAllA", "All A")
    _send(ctx, "HarnessAllB", "All B")
    _await_count(ctx, "HarnessAllB", 1)
    pid, path = _monitor(ctx)
    ctx.before("dbus.notifications", len(_list(ctx)))
    envelope = ctx.product(["dismiss-all-notifications"], headed=True)
    assert envelope.ok, envelope.raw
    closed = poll(lambda: closes(_monitor_text(path)), 6.0)
    ctx.after("dbus.notifications", len(_list(ctx)))
    _stop_monitor(pid)
    assert len(closed) >= 2, _monitor_text(path)
    assert not _list(ctx)


@implementation("notifications.action_invoked_signal")
def action_invoked_signal(ctx: LegContext) -> None:
    _send(ctx, "HarnessAction", "Action summary", actions=(("open", "Open"),))
    _await_count(ctx, "HarnessAction", 1)
    pid, path = _monitor(ctx)
    ctx.before("dbus.notifications", 1)
    envelope = ctx.product(
        ["notification-action", "1", "open", "--expected-app", "HarnessAction"], headed=True
    )
    assert envelope.ok, envelope.raw
    invoked = poll(
        lambda: [item for item in notify.parse_monitor_text(_monitor_text(path)) if item[0] == "ActionInvoked"],
        5.0,
    )
    ctx.after("dbus.notifications", len(_list(ctx)))
    _stop_monitor(pid)
    assert invoked, _monitor_text(path)
    assert invoked[0][1]["action"] == "open", invoked


@implementation("notifications.delivered_mutation_internal_fails")
def delivered_mutation_internal_fails(ctx: LegContext) -> None:
    _send(ctx, "HarnessInternal", "Internal summary")
    _await_count(ctx, "HarnessInternal", 1)
    pid, path = _monitor(ctx)
    ctx.before("dbus.notifications", 1)
    envelope = ctx.product(
        ["dismiss-notification", "1", "--expected-app", "HarnessInternal"], headed=True
    )
    closed = poll(lambda: closes(_monitor_text(path)), 5.0)
    remaining = _list(ctx, "HarnessInternal")
    ctx.after("dbus.notifications", len(remaining))
    _stop_monitor(pid)
    if not envelope.ok and code_of(envelope) == "INTERNAL":
        assert not closed and remaining, "the mutation was delivered but answered INTERNAL"
