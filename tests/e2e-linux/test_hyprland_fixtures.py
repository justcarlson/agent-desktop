from __future__ import annotations

import json
import os
import socket
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import attached, second_instance, socket_fixture
from test_hyprland_lifecycle import make_proc


def exchange(path: Path, timeout: float = 0.4) -> bytes | None:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(timeout)
        client.connect(str(path))
        client.sendall(b"j/version")
        try:
            return client.recv(4096)
        except TimeoutError:
            return None
        except ConnectionResetError:
            return b""


class Workspace(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.shared = self.root / "run"
        (self.shared / "hypr").mkdir(parents=True)
        self.work = self.root / "work"
        self.work.mkdir()
        self.proc = self.root / "proc"
        self.proc.mkdir()
        record = attached.new_record(slice_name="ad_e2e_1_2.slice", shared_runtime=str(self.shared),
                                     signature="main_sig", mirror=self.root / "mirror.json")
        attached.save(self.work, record)

    def listener(self, behavior: str) -> socket_fixture.Listener:
        listener = socket_fixture.Listener(self.root / f"{behavior}.sock", behavior)
        self.addCleanup(listener.close)
        return listener


class ListenerTests(Workspace):
    def test_an_answering_listener_replies_once_and_closes(self):
        self.assertEqual(exchange(self.listener(socket_fixture.ANSWER).path), socket_fixture.REPLY)

    def test_a_hangup_listener_closes_without_a_reply(self):
        self.assertEqual(exchange(self.listener(socket_fixture.HANGUP).path), b"")

    def test_mute_and_silent_listeners_never_reply(self):
        for behavior in (socket_fixture.MUTE, socket_fixture.SILENT):
            with self.subTest(behavior=behavior):
                self.assertIsNone(exchange(self.listener(behavior).path))

    def test_close_removes_the_socket(self):
        listener = socket_fixture.Listener(self.root / "gone.sock", socket_fixture.MUTE)
        listener.close()
        self.assertFalse(listener.path.exists())

    def test_an_unknown_behavior_is_refused(self):
        with self.assertRaises(ValueError):
            socket_fixture.Listener(self.root / "x.sock", "reply-forever")


class FixtureDirectoryTests(Workspace):
    def test_a_fixture_directory_is_recorded_before_it_is_created(self):
        directory = socket_fixture.fixture_dir(self.work, "probe")
        self.assertEqual(directory.parent, self.shared / "hypr")
        self.assertIn(str(directory), attached.load(self.work)["fixtures"])
        self.assertTrue(directory.is_dir())

    def test_a_fake_instance_uses_a_valid_signature_and_answers(self):
        signature, listener = socket_fixture.instance(self.work, socket_fixture.ANSWER)
        self.addCleanup(listener.close)
        from oracle import hyprland
        self.assertIn(signature, hyprland.instances(self.shared))
        self.assertEqual(exchange(listener.path), socket_fixture.REPLY)

    def test_stale_instances_refuse_every_connection(self):
        for name in socket_fixture.stale_instances(self.work):
            with self.subTest(name=name), self.assertRaises(OSError):
                exchange(self.shared / "hypr" / name / ".socket.sock")

    def test_the_private_runtime_holds_only_stale_instances(self):
        runtime = socket_fixture.private_runtime(self.work)
        self.assertEqual(runtime.stat().st_mode & 0o777, 0o700)
        entries = sorted((runtime / "hypr").iterdir())
        self.assertEqual(len(entries), 2)
        for entry in entries:
            with self.subTest(entry=entry.name), self.assertRaises(OSError):
                exchange(entry / ".socket.sock")

    def test_each_mode_gets_its_own_private_runtime(self):
        first, second = (socket_fixture.private_runtime(self.work) for _ in range(2))
        self.assertNotEqual(first, second)
        self.assertEqual(len(list((second / "hypr").iterdir())), 2)

    def test_the_private_runtime_fits_a_socket_under_the_deepest_session_base(self):
        deepest = len("/run/user/4294967294/agent-desktop-sessions/rt.abcdefgh")
        work = self.root / ("w" * (deepest - len(str(self.root)) - 1))
        work.mkdir()
        socket_fixture.private_runtime(work)


class SecondInstanceTests(Workspace):
    def test_both_config_languages_set_a_fixed_mode_and_no_animations(self):
        self.assertIn("hl.monitor", second_instance.config_text(True))
        self.assertIn("monitor = ,", second_instance.config_text(False))
        for lua in (True, False):
            self.assertIn(second_instance.MODE, second_instance.config_text(lua))

    def test_the_second_instance_opens_no_update_or_donation_window(self):
        for lua in (True, False):
            with self.subTest(lua=lua):
                text = second_instance.config_text(lua)
                for key in ("no_update_news", "no_donation_nag"):
                    self.assertRegex(text, rf"{key} = true")

    MONITORS = [{"name": "WAYLAND-1", "width": 0, "height": 0, "availableModes": []},
                {"name": "AGENT-DESKTOP-1", "width": 0, "height": 0, "availableModes": ["1920x1080@60.00Hz"]}]

    def test_the_headless_output_gets_its_advertised_mode_and_the_modeless_output_is_disabled(self):
        self.assertEqual(second_instance.mode_commands(self.MONITORS, False), [
            ["keyword", "monitor", "AGENT-DESKTOP-1,1920x1080@60.00,0x0,1"],
            ["keyword", "monitor", "WAYLAND-1,disable"]])
        self.assertEqual(second_instance.mode_commands(self.MONITORS, True), [
            ["eval", 'hl.monitor({ output = "AGENT-DESKTOP-1", mode = "1920x1080@60.00", position = "0x0", scale = 1 })'],
            ["eval", 'hl.monitor({ output = "WAYLAND-1", disabled = true })']])

    def test_a_headless_output_that_advertises_no_mode_is_a_fixture_error_with_the_dump(self):
        modeless = [dict(monitor, availableModes=[]) for monitor in self.MONITORS]
        with self.assertRaises(second_instance.OutputsNotReady) as raised:
            second_instance.mode_commands(modeless, False)
        self.assertIn('"name": "WAYLAND-1"', str(raised.exception))

    def test_readiness_needs_the_headless_output_alone_with_a_positive_size(self):
        check = second_instance.readiness_problems
        sized = {"name": "AGENT-DESKTOP-1", "width": 1920, "height": 1080}
        self.assertEqual(check([sized, {"name": "WAYLAND-1", "width": 0, "height": 0, "disabled": True}]), [])
        self.assertEqual(check([dict(sized, height=0)]), ["AGENT-DESKTOP-1 is 1920x0"])
        self.assertEqual(len(check([sized, {"name": "WAYLAND-1", "width": 0, "height": 0}])), 2)
        self.assertEqual(len(check([dict(sized, disabled=True)])), 1)

    def test_outputs_that_never_get_a_size_fail_as_a_fixture_error_before_any_product_call(self):
        answers = {("-j", "monitors", "all"): json.dumps(self.MONITORS)}
        control = lambda runtime, signature, argv: answers.get(tuple(argv), "ok")
        with mock.patch.object(second_instance.hyprland, "control", side_effect=control):
            with self.assertRaises(second_instance.OutputsNotReady) as raised:
                second_instance._size_outputs(Path("/run/user/1000"), "nested", False, 0.2)
        self.assertIn("AGENT-DESKTOP-1 is 0x0", str(raised.exception))
        self.assertIn('"availableModes": ["1920x1080@60.00Hz"]', str(raised.exception))

    def test_outputs_that_get_a_size_pass_the_readiness_wait(self):
        sized = [{"name": "WAYLAND-1", "width": 0, "height": 0, "disabled": True, "availableModes": []},
                 {"name": "AGENT-DESKTOP-1", "width": 1920, "height": 1080, "availableModes": ["1920x1080@60.00Hz"]}]
        reads = iter([self.MONITORS, self.MONITORS, sized])
        sent = []

        def control(runtime, signature, argv):
            if argv == ["-j", "monitors", "all"]:
                return json.dumps(next(reads, sized))
            sent.append(argv)
            return "ok"

        with mock.patch.object(second_instance.hyprland, "control", side_effect=control):
            second_instance._size_outputs(Path("/run/user/1000"), "nested", False, 2.0)
        self.assertEqual(len(sent), 2)

    def test_a_refused_mode_rule_is_a_fixture_error(self):
        answers = {("-j", "monitors", "all"): json.dumps(self.MONITORS)}
        control = lambda runtime, signature, argv: answers.get(tuple(argv), "invalid monitor")
        with mock.patch.object(second_instance.hyprland, "control", side_effect=control):
            with self.assertRaises(second_instance.OutputsNotReady):
                second_instance._size_outputs(Path("/run/user/1000"), "nested", False, 0.2)

    def test_the_headless_output_sits_beside_the_first_monitor(self):
        for lua in (True, False):
            with self.subTest(lua=lua):
                command = " ".join(second_instance.output_command(lua))
                self.assertIn(second_instance.OUTPUT, command)
                self.assertIn("1280x0", command)

    def start(self, diagnose: bool) -> dict:
        tier = SimpleNamespace(runtime_dir=self.work, env={"WAYLAND_DISPLAY": "wayland-1", "PATH": "/usr/bin"})
        seen = {"control": [], "spawn": []}
        instance = {"signature": "nested", "pid": 70, "start": 700, "wayland": None}
        patches = [
            mock.patch.object(second_instance.hyprland, "uses_lua", return_value=False),
            mock.patch.object(second_instance.hyprland, "instances", return_value=[]),
            mock.patch.object(second_instance.hyprland, "control",
                              side_effect=lambda runtime, signature, argv: seen["control"].append(argv) or "ok"),
            mock.patch.object(second_instance.attached_session, "spawn",
                              side_effect=lambda tier, argv, env, log, *where: seen["spawn"].append(env)
                              or seen.setdefault("workspace", where)),
            mock.patch.object(second_instance, "find_new", return_value=instance),
            mock.patch.object(second_instance, "wayland_of", return_value="wayland-2"),
            mock.patch.object(second_instance, "_sentinel", return_value=[71, 710]),
            mock.patch.object(second_instance, "_size_outputs"),
        ]
        for patch in patches:
            patch.start()
            self.addCleanup(patch.stop)
        seen["sized"] = second_instance._size_outputs
        seen["instance"] = (second_instance.ensure_sized(tier, 0.2) if diagnose
                            else second_instance.ensure(tier, 0.2))
        seen["config"] = (self.work / "second-instance.conf").read_text()
        return seen

    def test_the_instance_window_gets_an_outer_workspace_no_leg_app_shares(self):
        for diagnose in (False, True):
            self.assertEqual(self.start(diagnose)["workspace"], (second_instance.OUTER_WORKSPACE,))
        self.assertNotEqual(second_instance.OUTER_WORKSPACE, second_instance.attached_session.WORKSPACE)

    def test_a_leg_that_does_not_read_outputs_keeps_the_old_start_path(self):
        seen = self.start(diagnose=False)
        self.assertEqual(seen["control"], [["output", "create", "headless", second_instance.OUTPUT],
                                           second_instance.output_command(False)])
        seen["sized"].assert_not_called()
        self.assertNotIn("WAYLAND_DEBUG", seen["spawn"][0])
        self.assertNotIn("GBM_ALWAYS_SOFTWARE", seen["spawn"][0])
        self.assertNotIn("disable_logs", seen["config"])

    def test_the_output_reading_leg_logs_its_start_and_waits_for_sized_outputs(self):
        seen = self.start(diagnose=True)
        seen["sized"].assert_called_once()
        self.assertNotIn("GBM_ALWAYS_SOFTWARE", seen["spawn"][0])
        self.assertEqual(seen["spawn"][0]["WAYLAND_DEBUG"], "client")
        self.assertIn("disable_logs = false", seen["config"])

    def test_the_output_reading_leg_starts_without_logs_when_logging_is_off(self):
        with mock.patch.object(second_instance, "SIZED_LOGS", False):
            seen = self.start(diagnose=True)
        seen["sized"].assert_called_once()
        self.assertNotIn("WAYLAND_DEBUG", seen["spawn"][0])
        self.assertNotIn("disable_logs", seen["config"])

    def test_outputs_not_ready_carries_the_instance_log_window_and_launch_env(self):
        nested = self.shared / "hypr" / "nested"
        nested.mkdir()
        (nested / "hyprland.log").write_text("Running on WAYLAND_DISPLAY: wayland-1\n")
        (self.work / "second-instance.log").write_text("[wayland-client] xdg_toplevel.configure\n")
        make_proc(self.proc, 70, 700)
        (self.proc / "70" / "environ").write_bytes(b"WAYLAND_DISPLAY=wayland-1\0AQ_TRACE=1\0HOME=/home/user\0")
        tier = SimpleNamespace(runtime_dir=self.work, env={"WAYLAND_DISPLAY": "wayland-1", "HOME": "/home/user"})
        clients = [{"pid": 70, "mapped": True, "size": [1280, 800], "workspace": {"name": "special:ad_e2e"}},
                   {"pid": 12}]
        with mock.patch.object(second_instance.hyprland, "query", return_value=clients):
            found = second_instance.diagnostics(tier, {"signature": "nested", "pid": 70}, True, self.proc)
        self.assertIn("Running on WAYLAND_DISPLAY", found["hyprland_log"]["text"])
        self.assertIn("xdg_toplevel.configure", found["launch_log"]["text"])
        self.assertEqual(found["outer_client"]["size"], [1280, 800])
        self.assertEqual(found["launch_env"]["given"], {"WAYLAND_DEBUG": "client", "WAYLAND_DISPLAY": "wayland-1"})
        self.assertEqual(found["launch_env"]["process"], {"AQ_TRACE": "1", "WAYLAND_DISPLAY": "wayland-1"})
        self.assertEqual(second_instance.outer_client([{"pid": 12}], 70), {"missing": True, "client_pids": [12]})

    def test_the_outer_window_evidence_names_the_nested_window_placement_only(self):
        tier = SimpleNamespace(runtime_dir=self.work)
        clients = [{"pid": 70, "at": [0, 0], "size": [630, 754], "workspace": {"id": 1, "name": "1"},
                    "floating": False, "pinned": False, "fullscreen": 0, "monitor": 0, "title": "x"},
                   {"pid": 12, "at": [640, 0]}]
        with mock.patch.object(second_instance.hyprland, "query", return_value=clients):
            found = second_instance.outer_window(tier, {"pid": 70})
        self.assertEqual(found, {"at": [0, 0], "size": [630, 754], "workspace": {"id": 1, "name": "1"},
                                 "floating": False, "pinned": False, "fullscreen": 0, "monitor": 0})
        with mock.patch.object(second_instance.hyprland, "query", side_effect=RuntimeError("gone")):
            self.assertEqual(second_instance.outer_window(tier, {"pid": 70}), {"error": "gone"})
        with mock.patch.object(second_instance.hyprland, "query", return_value=clients[1:]):
            self.assertEqual(second_instance.outer_window(tier, {"pid": 70}), {"missing": True, "client_pids": [12]})

    def test_ensure_sized_attaches_the_diagnostics_and_reraises(self):
        failure = second_instance.OutputsNotReady("0x0")
        with mock.patch.object(second_instance, "ensure", return_value={"signature": "nested", "pid": 70}), \
                mock.patch.object(second_instance.hyprland, "uses_lua", return_value=False), \
                mock.patch.object(second_instance, "_size_outputs", side_effect=failure), \
                mock.patch.object(second_instance, "diagnostics", return_value={"outer_client": {}}):
            with self.assertRaises(second_instance.OutputsNotReady) as raised:
                second_instance.ensure_sized(SimpleNamespace(runtime_dir=self.work), 0.1)
        self.assertEqual(raised.exception.diagnostics, {"outer_client": {}})

    def test_the_sentinel_is_found_by_the_title_it_started_with(self):
        retitled = {"title": "user@host:~", "initialTitle": second_instance.SENTINEL, "pid": 81}
        other = {"title": second_instance.SENTINEL, "initialTitle": "foot", "pid": 82}
        self.assertEqual(second_instance.sentinel_pid([other, retitled]), 81)
        self.assertIsNone(second_instance.sentinel_pid([other]))
        self.assertIsNone(second_instance.sentinel_pid([dict(retitled, pid=-1)]))

    def test_current_needs_the_same_generation(self):
        record = attached.load(self.work)
        record["instances"] = [{"signature": "nested", "pid": 70, "start": 700, "wayland": "wayland-2"}]
        self.assertIsNone(second_instance.current(record, self.proc))
        make_proc(self.proc, 70, 701)
        self.assertIsNone(second_instance.current(record, self.proc))
        make_proc(self.proc, 70, 700)
        self.assertEqual(second_instance.current(record, self.proc)["signature"], "nested")

    def answering(self, signature: str) -> None:
        (self.shared / "hypr" / signature).mkdir()
        listener = socket_fixture.Listener(self.shared / "hypr" / signature / ".socket.sock",
                                           socket_fixture.ANSWER)
        self.addCleanup(listener.close)

    def test_a_new_instance_counts_only_when_its_peer_is_in_the_run_slice(self):
        self.answering("nested")
        record = attached.load(self.work)
        make_proc(self.proc, os.getpid(), 900, cgroup="0::/user.slice/other.slice/app.scope")
        self.assertIsNone(second_instance.find_new(record, set(), self.proc))
        make_proc(self.proc, os.getpid(), 900, cgroup=f"0::/user.slice/{record['slice']}/run-1.scope")
        self.assertEqual(second_instance.find_new(record, set(), self.proc),
                         {"signature": "nested", "pid": os.getpid(), "start": 900, "wayland": None})
        self.assertIsNone(second_instance.find_new(record, {"nested"}, self.proc))

    def test_the_wayland_socket_is_the_one_the_instance_serves(self):
        listener = socket_fixture.Listener(self.shared / "wayland-7", socket_fixture.MUTE)
        self.addCleanup(listener.close)
        (self.shared / "wayland-3").write_bytes(b"")
        self.assertEqual(second_instance.wayland_of(self.shared, os.getpid()), "wayland-7")
        self.assertIsNone(second_instance.wayland_of(self.shared, os.getpid() + 1))

    def test_stopping_a_dead_instance_removes_its_directory_and_sockets(self):
        record = attached.load(self.work)
        instance = {"signature": "nested", "pid": 70, "start": 700, "wayland": "wayland-9"}
        record["instances"] = [instance]
        attached.save(self.work, record)
        (self.shared / "hypr" / "nested").mkdir()
        (self.shared / "hypr" / "main_sig").mkdir()
        tier = SimpleNamespace(runtime_dir=self.work)
        second_instance.stop(tier, self.proc, seconds=0.1)
        self.assertFalse((self.shared / "hypr" / "nested").exists())
        self.assertTrue((self.shared / "hypr" / "main_sig").exists())
        self.assertTrue(self.shared.is_dir())


if __name__ == "__main__":
    unittest.main()
