from __future__ import annotations

from contextlib import contextmanager

from runner.helper import delayed
from runner.legs import implementation
from oracle import notify
from oracle.atspi_raw import poll


def _list(ctx, app=None):
    argv = ["list-notifications"]
    if app:
        argv += ["--app", app]
    envelope = ctx.product(argv)
    assert envelope.ok, envelope.raw
    return (envelope.data or {}).get("notifications", [])


def _await(ctx, app, count=1):
    def ready():
        entries = _list(ctx, app)
        return entries if len(entries) == count else None
    result = poll(ready, 8)
    assert result, f"missing {app} notifications"
    return result


def _refusal(envelope, code):
    assert not envelope.ok and (envelope.error or {}).get("code") == code, envelope.raw


def _dismiss(ctx, entry):
    return ctx.product(["dismiss-notification", str(entry["index"]),
                        "--expected-app", entry["app_name"],
                        "--expected-title", entry["title"]])


@contextmanager
def _observing(ctx):
    with notify.observing(ctx) as sender:
        assert not _list(ctx), "private notification list was not empty before seeding"
        yield sender
    assert not _list(ctx), "private notification list was not restored"


@implementation("notifications.list_equals_sender_record")
def list_equals_sender_record(ctx):
    with _observing(ctx) as sender:
        sender.send("HarnessAlpha", "Alpha summary", "first body")
        sender.send("HarnessBeta", "Beta summary", "second body", (("open", "Open"),))
        _await(ctx, "HarnessBeta")
        observed = _list(ctx)
        expected = list(reversed(list(sender.records.values())))
        problems = notify.compare_records(expected, observed)
        assert not problems, "; ".join(problems)
        for want, got in zip(expected, observed):
            assert got.get("body") == want["body"], observed
            assert got.get("actions", []) == want["actions"], observed
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.app_filter_case_insensitive")
def app_filter_case_insensitive(ctx):
    with _observing(ctx) as sender:
        sender.send("HarnessCase", "Case summary")
        sender.send("HarnessOther", "Other summary")
        _await(ctx, "HarnessOther")
        observed = _await(ctx, "NESSCa")
        assert [(entry["app_name"], entry["title"], entry["index"]) for entry in observed] == [
            ("HarnessCase", "Case summary", 2)], observed
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.wait_notification_passes")
def wait_notification_passes(ctx):
    with _observing(ctx) as sender:
        marker = "HarnessWaitMarker"
        trigger = delayed(0.8, lambda: sender.send("HarnessWait", marker))
        try:
            envelope = ctx.product(["wait", "--notification", "--text", marker, "--timeout", "8000"])
        finally:
            trigger.join()
        assert envelope.ok, envelope.raw
        assert (envelope.data or {}).get("matched") is True, envelope.raw
        entry = (envelope.data or {}).get("notification", {})
        assert (entry.get("app_name"), entry.get("title")) == ("HarnessWait", marker), envelope.raw
        assert [(record["app"], record["summary"]) for record in sender.active()] == [
            ("HarnessWait", marker)]
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.wait_notification_times_out")
def wait_notification_times_out(ctx):
    with _observing(ctx) as sender:
        envelope = ctx.product(["wait", "--notification", "--text", "HarnessAbsentMarker", "--timeout", "1500"])
        _refusal(envelope, "TIMEOUT")
        assert not sender.signals(), sender.signals()
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.headless_mutations_policy_denied")
def headless_mutations_policy_denied(ctx):
    with _observing(ctx) as sender:
        sender.send("HarnessMutate", "Mutation summary", actions=(("open", "Open"),))
        before = _await(ctx, "HarnessMutate")
        entry = before[0]
        for argv in (["dismiss-notification", str(entry["index"]), "--expected-app", entry["app_name"]],
                     ["dismiss-all-notifications"],
                     ["notification-action", str(entry["index"]), "Open", "--expected-app", entry["app_name"]]):
            _refusal(ctx.product(argv), "POLICY_DENIED")
        assert not sender.signals(), sender.signals()
        assert _list(ctx, "HarnessMutate") == before
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.wrong_app_not_found")
def wrong_app_not_found(ctx):
    with _observing(ctx) as sender:
        sender.send("HarnessRight", "Right summary")
        before = _await(ctx, "HarnessRight")
        envelope = ctx.product(["dismiss-notification", str(before[0]["index"]),
                                "--expected-app", "HarnessWrong"])
        _refusal(envelope, "NOTIFICATION_NOT_FOUND")
        assert not sender.signals(), sender.signals()
        assert _list(ctx, "HarnessRight") == before
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.empty_list_not_found")
def empty_list_not_found(ctx):
    with _observing(ctx) as sender:
        envelope = ctx.product(["dismiss-notification", "1", "--expected-app", "HarnessAbsent"])
        _refusal(envelope, "NOTIFICATION_NOT_FOUND")
        assert not sender.signals(), sender.signals()
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.dismiss_one_reason_closed")
def dismiss_one_reason_closed(ctx):
    with _observing(ctx) as sender:
        identifier = sender.send("HarnessDismiss", "Dismiss me")
        entry = _await(ctx, "HarnessDismiss")[0]
        envelope = _dismiss(ctx, entry)
        assert envelope.ok, envelope.raw
        assert sender.wait_closed([identifier]) == [{"id": identifier, "reason": 2}]
        ctx.after("dbus.notifications", sender.active())


@implementation("notifications.dismiss_all_closes_every_notification")
def dismiss_all_closes_every_notification(ctx):
    with _observing(ctx) as sender:
        identifiers = {sender.send("HarnessAllA", "All A"), sender.send("HarnessAllB", "All B")}
        _await(ctx, "HarnessAllB")
        envelope = ctx.product(["dismiss-all-notifications"])
        assert envelope.ok, envelope.raw
        closed = sender.wait_closed(identifiers)
        assert sorted((item["id"], item["reason"]) for item in closed) == [
            (identifier, 2) for identifier in sorted(identifiers)], closed
        assert not sender.active(), sender.active()
        ctx.after("dbus.notifications", sender.active())
