from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from oracle import notify


class NotificationIdTests(unittest.TestCase):
    def test_last_integer_wins(self) -> None:
        self.assertEqual(notify.parse_notification_id("queued 3\nid 41\n"), 41)

    def test_missing_id_raises(self) -> None:
        with self.assertRaises(ValueError):
            notify.parse_notification_id("no identifiers here")


class NotificationArgvTests(unittest.TestCase):
    def test_body_and_actions_are_passed(self) -> None:
        argv = notify.notification_argv(
            "Harness", "Summary", "Body", (("open", "Open"),), script=Path("/x/oracle.sh")
        )
        self.assertEqual(
            argv,
            [
                "bash",
                "/x/oracle.sh",
                "notify",
                "--app",
                "Harness",
                "--summary",
                "Summary",
                "--body",
                "Body",
                "--action",
                "open=Open",
            ],
        )

    def test_empty_body_and_actions_are_omitted(self) -> None:
        argv = notify.notification_argv("A", "S", script=Path("/oracle.sh"))
        self.assertNotIn("--body", argv)
        self.assertNotIn("--action", argv)

    def test_monitor_filters_the_notification_interface(self) -> None:
        argv = notify.monitor_argv()
        self.assertEqual(argv[0], "dbus-monitor")
        self.assertIn("org.freedesktop.Notifications", argv[-1])


class MonitorDecoderTests(unittest.TestCase):
    SAMPLE = """signal time=1.0 sender=:1.2 -> destination=(null destination) serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=NotificationClosed
   uint32 7
   uint32 2

signal time=2.0 sender=:1.2 -> destination=(null destination) serial=10 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=ActionInvoked
   uint32 7
   string "open"

signal time=3.0 sender=:1.2 -> destination=(null destination) serial=11 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "unrelated"
"""

    def test_closed_and_invoked_are_decoded(self) -> None:
        signals = notify.parse_monitor_text(self.SAMPLE)
        self.assertEqual(
            signals,
            [
                ("NotificationClosed", {"id": 7, "reason": 2}),
                ("ActionInvoked", {"id": 7, "action": "open"}),
            ],
        )

    def test_unrelated_members_are_ignored(self) -> None:
        self.assertEqual(notify.parse_monitor_text(self.SAMPLE)[-1][0], "ActionInvoked")
        self.assertFalse(any(name == "Notify" for name, _ in notify.parse_monitor_text(self.SAMPLE)))

    def test_empty_text_has_no_signals(self) -> None:
        self.assertEqual(notify.parse_monitor_text(""), [])


class RecordTests(unittest.TestCase):
    def test_matching_records_pass(self) -> None:
        expected = [{"app": "A", "summary": "one"}, {"app": "B", "summary": "two"}]
        listed = [
            {"index": 1, "app_name": "A", "title": "one"},
            {"index": 2, "app_name": "B", "title": "two"},
        ]
        self.assertEqual(notify.compare_records(expected, listed), [])

    def test_count_and_identity_mismatches_are_reported(self) -> None:
        problems = notify.compare_records(
            [{"app": "A", "summary": "one"}],
            [{"index": 1, "app_name": "B", "title": "other"}, {"index": 2}],
        )
        self.assertEqual(len(problems), 3)


if __name__ == "__main__":
    unittest.main()
