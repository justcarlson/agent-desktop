import tempfile
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

    def run_leg(self, after_stop):
        ctx = LegContext(self.tier, "headed", legs.leg_by_id("gaps.gesture_qt_first_mouse-click"),
                         member_reader=lambda _: frozenset())

        @contextmanager
        def managed(ctx, name):
            self.present = self.present + [APP]
            yield SimpleNamespace(pid=APP[0], start_time=APP[1])
            self.present = list(after_stop)

        with patch.object(toolkit_qt, "members", side_effect=lambda _: sorted(self.present)), \
            patch.object(toolkit_qt, "managed", managed):
            with toolkit_qt.qt_app(ctx):
                ctx.after("applog", toolkit_qt.log_state(ctx))
        return ctx

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
