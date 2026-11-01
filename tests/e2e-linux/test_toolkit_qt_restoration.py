import tempfile
import time
import unittest
from contextlib import contextmanager
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from runner import legs, verdict
from runner.verdict import LegContext
from scenarios import toolkit_qt

SESSION = (100, 10, "dbus-daemon")
PREVIEWER = (200, 20, "gjs")
APP = (300, 30, "python3")
LEFTOVER = (400, 40, "python3")


class QtRestorationTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.tier = SimpleNamespace(runtime_dir=Path(directory.name))
        self.present = [SESSION, PREVIEWER]

    def run_leg(self, after_stop, exit_delay_s=0.0):
        ctx = LegContext(self.tier, "headed", legs.leg_by_id("gaps.gesture_qt_first_mouse-click"),
                         member_reader=lambda _: frozenset())
        exits_at = [None]

        @contextmanager
        def managed(ctx, name):
            self.present = self.present + [APP]
            yield SimpleNamespace(pid=APP[0], start_time=APP[1])
            exits_at[0] = time.monotonic() + exit_delay_s

        def members(_):
            if exits_at[0] is not None and time.monotonic() >= exits_at[0]:
                self.present = list(after_stop)
            return sorted(self.present)

        with patch.object(toolkit_qt, "members", side_effect=members), \
            patch.object(toolkit_qt, "managed", managed), \
            patch.object(toolkit_qt, "APP_EXIT_BUDGET_S", 1.0):
            with toolkit_qt.qt_app(ctx):
                ctx.after("applog", toolkit_qt.log_state(ctx))
        return ctx

    def test_an_app_that_exits_shortly_after_stop_passes(self):
        ctx = self.run_leg([SESSION, PREVIEWER], exit_delay_s=0.3)
        reads = ctx._reads["applog"]
        self.assertEqual(reads["restored"], reads["before"])
        self.assertEqual(verdict.evaluate(ctx), [])

    def test_an_app_that_never_exits_fails_with_the_members(self):
        started = time.monotonic()
        with self.assertRaises(AssertionError) as raised:
            self.run_leg([SESSION, PREVIEWER, APP])
        self.assertGreaterEqual(time.monotonic() - started, 1.0)
        self.assertIn(str(APP), str(raised.exception))

    def test_a_session_service_that_exits_during_the_leg_is_not_drift(self):
        ctx = self.run_leg([SESSION])
        reads = ctx._reads["applog"]
        self.assertEqual(reads["restored"], reads["before"])
        self.assertEqual(reads["after"][0]["added"], [APP])
        self.assertEqual(verdict.evaluate(ctx), [])

    def test_a_process_the_leg_leaves_behind_fails(self):
        with self.assertRaises(AssertionError):
            self.run_leg([SESSION, PREVIEWER, LEFTOVER])

    def test_an_app_still_listed_after_teardown_fails(self):
        with self.assertRaises(AssertionError):
            self.run_leg([SESSION, PREVIEWER, APP])


if __name__ == "__main__":
    unittest.main()
