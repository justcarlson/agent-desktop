import os
import sys
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

from oracle import atspi_raw as raw, gsettings, proc
from runner import apps
from runner.helper import delayed


class ProcessOracleTests(unittest.TestCase):
    def test_proc_stat_handles_spaces_and_closing_parentheses(self):
        fields = ["S", "2", "17"] + ["0"] * 16 + ["987654", "0"]
        self.assertEqual(proc.stat_fields("123 (an app ) name) " + " ".join(fields)),
                         (987654, 17, "an app ) name"))
        with self.assertRaises((IndexError, ValueError)):
            proc.stat_fields("not a stat record")

    def test_process_facts_refuse_a_reused_pid(self):
        fields = ["S", "2", "17"] + ["0"] * 16 + ["987654", "0"]
        first = "123 (calculator) " + " ".join(fields)
        second = first.replace("987654", "987655")
        with patch.object(Path, "read_text", side_effect=[first, first]), patch.object(
                Path, "read_bytes", return_value=b"XDG_RUNTIME_DIR=/private/a\0"):
            self.assertEqual(proc.facts(123), (987654, 17, "calculator", {"XDG_RUNTIME_DIR": "/private/a"}))
        with patch.object(Path, "read_text", side_effect=[first, second]), patch.object(
                Path, "read_bytes", return_value=b"XDG_RUNTIME_DIR=/private/a\0"):
            with self.assertRaises(ProcessLookupError):
                proc.facts(123)

    def test_environment_preserves_equals_and_exact_runtime(self):
        self.assertEqual(proc.environment(b"XDG_RUNTIME_DIR=/private/a\0TOKEN=a=b\0ignored\0"),
                         {"XDG_RUNTIME_DIR": "/private/a", "TOKEN": "a=b"})
        tier = SimpleNamespace(runtime_dir=Path("/private/a"))
        self.assertTrue(apps.attributable(tier, 123, 9, {"XDG_RUNTIME_DIR": "/private/a"}))
        self.assertFalse(apps.attributable(tier, 123, 9, {"XDG_RUNTIME_DIR": "/private/ab"}))
        self.assertFalse(apps.attributable(tier, 123, 0, {"XDG_RUNTIME_DIR": "/private/a"}))

    def test_registry_attribution_rejects_foreign_first_owner_and_counts_two(self):
        tier = SimpleNamespace(runtime_dir=Path("/private/a"))

        def owner(pid):
            frame = SimpleNamespace(bus=f":1.{pid}", path="/frame", role=23)
            return SimpleNamespace(bus=frame.bus, name="gnome-calculator",
                                   children=lambda: [frame])

        owners = [owner(pid) for pid in (11, 12, 13)]

        def facts(pid):
            return (100 + pid, pid, "calculator", {"XDG_RUNTIME_DIR":
                    "/foreign" if pid == 11 else "/private/a", "DISPLAY": ":151"})

        with patch.object(raw, "registry", return_value=owners), patch.object(
                raw, "owner_pid", side_effect=lambda node: int(node.bus.split(".")[-1])), patch.object(
                proc, "facts", side_effect=facts):
            found = apps.owners_named(tier, "Calculator")
        self.assertEqual([app.pid for app in found], [12, 13])
        self.assertEqual([app.start_time for app in found], [112, 113])
        self.assertEqual([app.windows for app in found], [["w-1.12-frame"], ["w-1.13-frame"]])

    def test_window_identity_changes_with_the_bus_owner(self):
        first = SimpleNamespace(bus=":1.17", path="/org/a11y/atspi/accessible/5")
        second = SimpleNamespace(bus=":1.18", path=first.path)
        self.assertEqual(apps.window_id(first), "w-1.17-org-a11y-atspi-accessible-5")
        self.assertNotEqual(apps.window_id(first), apps.window_id(second))
        with self.assertRaises(ValueError):
            apps.window_id(SimpleNamespace(bus="org.app", path="/frame"))

    def test_wrong_display_is_not_accepted(self):
        tier = SimpleNamespace(display=":151")
        app = SimpleNamespace(display=":100", name="Terminal", windows=[], pid=123)
        with patch.object(apps.x11, "stacking_xids", return_value=[]), self.assertRaises(AssertionError):
            apps._display_verified(tier, app)

    def test_unowned_stop_is_refused_before_signalling(self):
        app = apps.OwnedApp("foreign", 2147483000, 999, [], [], ":0")
        with self.assertRaises(ValueError):
            apps.stop(app)

    def test_stop_never_signals_a_reused_process(self):
        app = apps.OwnedApp("calculator", 123, 40, [":1.7"], ["w-1.7-frame"], ":1")
        tier = SimpleNamespace(runtime_dir=Path("/private/a"))
        with patch.dict(apps._TIERS, {(123, 40): tier}), patch.object(Path, "read_text", return_value="123 (cal) S"), patch.object(
                proc, "facts", return_value=(41, 123, "cal", {"XDG_RUNTIME_DIR": "/private/a"})), patch.object(
                raw, "poll", return_value=True), patch.object(apps.os, "kill") as kill, patch.object(apps.os, "killpg") as killpg:
            apps.stop(app)
        kill.assert_not_called()
        killpg.assert_not_called()

    def test_oracle_tool_uses_the_private_environment_and_checks_exit(self):
        tier = SimpleNamespace(env=dict(os.environ, ORACLE_TEST_VALUE="private-value"))
        value = gsettings.tool(tier, [sys.executable, "-c", "import os; print(os.environ['ORACLE_TEST_VALUE'])"])
        self.assertEqual(value, "private-value")
        with self.assertRaises(RuntimeError):
            gsettings.tool(tier, [sys.executable, "-c", "raise SystemExit(7)"])


class TriggerTests(unittest.TestCase):
    def test_helper_fires_after_delay_and_records_time(self):
        observed = []
        started = time.monotonic()
        trigger = delayed(0.025, lambda: observed.append(time.monotonic()))
        trigger.join()
        self.assertEqual(len(observed), 1)
        self.assertGreaterEqual(trigger.fired_at - started, 0.025)
        self.assertLessEqual(trigger.fired_at, observed[0])

    def test_helper_propagates_failure(self):
        def fail():
            raise LookupError("oracle failed")

        trigger = delayed(0, fail)
        with self.assertRaisesRegex(LookupError, "oracle failed"):
            trigger.join()

    def test_cancelled_helper_does_not_mutate(self):
        observed = []
        trigger = delayed(2, lambda: observed.append("late"))
        trigger.cancel()
        self.assertEqual(observed, [])
        self.assertIsNone(trigger.fired_at)
