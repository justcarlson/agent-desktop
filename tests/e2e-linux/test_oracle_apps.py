import os
import sys
import tempfile
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
    def test_owner_scan_ignores_retired_bus_names_but_preserves_other_errors(self):
        tier = SimpleNamespace(runtime_dir=Path("/private/a"))
        frame = SimpleNamespace(role=23, bus=":1.2", path="/frame")
        node = SimpleNamespace(name="Calculator", bus=":1.2", children=lambda: [frame])
        retired = RuntimeError("GDBus.Error:org.freedesktop.DBus.Error.NameHasNoOwner: gone")
        with patch.object(apps.raw, "registry", return_value=[node, node]), \
            patch.object(apps.raw, "owner_pid", side_effect=[retired, 124]), \
            patch.object(apps.proc, "facts", return_value=(99, 124, "calculator",
                {"XDG_RUNTIME_DIR": "/private/a", "DISPLAY": ":150"})):
            self.assertEqual([app.pid for app in apps._owners(tier)], [124])
        for code in ("NoReply", "AccessDenied"):
            with self.subTest(error=code), \
                patch.object(apps.raw, "registry", return_value=[node]), \
                patch.object(apps.raw, "owner_pid", side_effect=RuntimeError(code)):
                with self.assertRaisesRegex(RuntimeError, code):
                    apps._owners(tier)

    def test_owner_scan_skips_a_process_during_unreadable_exec(self):
        tier = SimpleNamespace(runtime_dir=Path("/private/a"))
        frame = SimpleNamespace(role=23, bus=":1.2", path="/frame")
        node = SimpleNamespace(name="Calculator", bus=":1.2", children=lambda: [frame])
        with patch.object(apps.raw, "registry", return_value=[node, node]), \
            patch.object(apps.raw, "owner_pid", side_effect=[123, 124]), \
            patch.object(apps.proc, "facts", side_effect=[PermissionError(),
                (99, 124, "calculator", {"XDG_RUNTIME_DIR": "/private/a", "DISPLAY": ":150"})]):
            self.assertEqual([app.pid for app in apps._owners(tier)], [124])

    def test_members_skip_processes_that_cannot_be_attributed(self):
        opaque = SimpleNamespace(name="1331", stat=lambda: SimpleNamespace(st_uid=os.geteuid()))
        owned = SimpleNamespace(name="124")
        facts = (99, 124, "owned", {"XDG_RUNTIME_DIR": "/private/a"})
        with patch.object(Path, "iterdir", return_value=[opaque, owned]), \
            patch.object(proc, "facts", side_effect=[PermissionError("opaque process"), facts]):
            self.assertEqual(proc.members(Path("/private/a")), [(124, 99, "owned")])

    def test_private_mutter_launch_closes_overview_before_returning(self):
        with tempfile.TemporaryDirectory() as root:
            tier = SimpleNamespace(name="x11-mutter", runtime_dir=Path(root), env={})
            app = apps.OwnedApp("calculator", 123, 40, [":1.7"], ["w-1.7-frame"], ":150")
            child = SimpleNamespace(pid=123, poll=lambda: None)
            with patch.object(apps, "_owners", side_effect=[[], [app]]), \
                patch.object(apps, "_display_verified", return_value=True), \
                patch.object(apps.subprocess, "Popen", return_value=child), \
                patch.object(apps.x11, "overview_active", side_effect=[True, True, False]) as active, \
                patch.object(apps, "tool") as tool, \
                patch.dict(apps._SPAWNED, clear=True), patch.dict(apps._TIERS, clear=True):
                self.assertIs(apps.launch(tier, "Calculator"), app)
                tool.assert_called_once_with(tier, ["xdotool", "key", "Escape"])
                self.assertEqual(active.call_count, 3)

    def test_warmup_activates_private_app_services_before_the_leg_baseline(self):
        for display, names in ((":150", ["Calculator", "Files", "Terminal", "Text Editor", "Settings"]),
                               (None, ["Calculator", "Text Editor"])):
            tier = SimpleNamespace(display=display, runtime_dir=Path("/private/a"))
            with patch.object(apps, "launch", side_effect=lambda tier, name: name) as launch, \
                patch.object(apps, "stop") as stop, \
                patch.object(apps.proc, "members", return_value=[]), \
                patch.object(apps.time, "sleep"), \
                patch.object(apps, "tool") as tool, \
                patch.object(apps.time, "monotonic", side_effect=[0, 0, 1]):
                apps.prime_desktop(tier)
            self.assertEqual([call.args[1] for call in launch.call_args_list], names)
            self.assertEqual([call.args[0] for call in stop.call_args_list], names)
            self.assertEqual(tool.call_count, 5 if display else 0)
            if display:
                self.assertIn(["gsettings", "set", "org.freedesktop.Tracker3.Miner.Files",
                               "index-recursive-directories", "[]"], [call.args[1] for call in tool.call_args_list])

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

    def test_stop_handles_unreadable_environ_without_signalling_a_reused_pid(self):
        app = apps.OwnedApp("calculator", 123, 40, [":1.7"], ["w-1.7-frame"], ":150")
        tier = SimpleNamespace(runtime_dir=Path("/private/a"))
        for start in (40, 41):
            state = ["S"]
            def stat():
                return "123 (cal) " + " ".join([state[0], "1", "123"] + ["0"] * 16 + [str(start)])
            with patch.dict(apps._TIERS, {(123, 40): tier}), \
                patch.object(Path, "read_text", side_effect=stat), \
                patch.object(proc, "facts", side_effect=PermissionError()), \
                patch.object(apps.os, "kill", side_effect=lambda *args: state.__setitem__(0, "Z")) as send, \
                patch.object(apps.os, "killpg") as group:
                apps.stop(app)
            self.assertEqual(send.call_count, int(start == 40))
            group.assert_not_called()

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
