from __future__ import annotations

import os
import socket
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

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

    def test_the_headless_output_sits_beside_the_first_monitor(self):
        for lua in (True, False):
            with self.subTest(lua=lua):
                command = " ".join(second_instance.output_command(lua))
                self.assertIn(second_instance.OUTPUT, command)
                self.assertIn("1280x0", command)

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
