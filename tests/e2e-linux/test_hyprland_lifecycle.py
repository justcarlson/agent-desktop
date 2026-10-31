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

from runner import attached, attached_guard, attached_session, cancel, session


def make_proc(root: Path, pid: int, start: int, *, cgroup: str = "0::/session.slice/x.service",
              comm: str = "comm", environ: dict | None = None) -> None:
    directory = root / str(pid)
    directory.mkdir(parents=True, exist_ok=True)
    fields = " ".join(["S"] + ["0"] * 18 + [str(start)])
    (directory / "stat").write_text(f"{pid} ({comm}) {fields}\n")
    (directory / "cgroup").write_text(cgroup + "\n")
    (directory / "environ").write_bytes(b"".join(f"{k}={v}\0".encode() for k, v in (environ or {}).items()))


class AttachedLifecycleTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.proc = self.root / "proc"
        self.proc.mkdir()
        self.shared = self.root / "run"
        (self.shared / "hypr" / "main_sig").mkdir(parents=True)
        (self.shared / "bus").touch()
        self.base = self.shared / "agent-desktop-sessions"
        self.work = self.base / "rt.owned"
        self.work.mkdir(parents=True)
        (self.work / session.OWNER_MARKER).write_text("999999 1\n")
        self.suite = self.root / "suite"
        self.suite.mkdir()
        record = attached.new_record(slice_name="ad_e2e_999999_1.slice", shared_runtime=str(self.shared),
                                     signature="main_sig", mirror=self.suite / "hyprland.attached.json")
        record["compositor"] = [50, 500]
        record["survivors"] = [[50, 500, "Hyprland", "compositor"], [60, 600, "foot", "bystander"]]
        attached.save(self.work, record)
        make_proc(self.proc, 50, 500, comm="Hyprland")
        make_proc(self.proc, 60, 600, comm="foot")

    def teardown(self, **kwargs):
        stopped = []
        attached_session.teardown(self.work, proc_root=self.proc, stop_slice=stopped.append, **kwargs)
        return stopped

    def test_attach_requires_a_live_hyprland_session_environment(self):
        with self.assertRaisesRegex(session.TierUnavailable, "HYPRLAND_INSTANCE_SIGNATURE, WAYLAND_DISPLAY"):
            attached_session.required_env({"XDG_RUNTIME_DIR": "/run/user/1000",
                                           "DBUS_SESSION_BUS_ADDRESS": "unix:path=/run/user/1000/bus"})

    def test_teardown_removes_only_recorded_paths_and_never_the_shared_runtime(self):
        stale = self.shared / "hypr" / "ad_e2e_stale_1"
        stale.mkdir()
        (stale / ".socket.sock").touch()
        foreign = self.shared / "hypr" / "someone_else"
        foreign.mkdir()
        attached.record_fixture(self.work, stale)
        stopped = self.teardown()
        self.assertEqual([record["slice"] for record in stopped], ["ad_e2e_999999_1.slice"])
        self.assertFalse(stale.exists())
        self.assertFalse(self.work.exists())
        for kept in (self.shared, self.shared / "bus", self.shared / "hypr" / "main_sig", foreign, self.base):
            self.assertTrue(kept.exists(), kept)
        mirror = json.loads((self.suite / "hyprland.attached.json").read_text())
        self.assertTrue(mirror["torn_down"])
        self.assertEqual(mirror["teardown_errors"], [])

    def test_a_dead_second_instance_loses_its_directory_and_its_silent_wayland_socket(self):
        (self.shared / "hypr" / "nested_sig").mkdir()
        (self.shared / "wayland-2").touch()
        (self.shared / "wayland-2.lock").touch()
        (self.shared / "wayland-1").touch()
        attached.record_instance(self.work, {"signature": "nested_sig", "pid": 70, "start": 700,
                                             "wayland": "wayland-2"})
        self.teardown()
        self.assertFalse((self.shared / "hypr" / "nested_sig").exists())
        self.assertFalse((self.shared / "wayland-2").exists())
        self.assertFalse((self.shared / "wayland-2.lock").exists())
        self.assertTrue((self.shared / "wayland-1").exists())

    def test_a_listening_wayland_socket_is_never_removed(self):
        (self.shared / "hypr" / "nested_sig").mkdir()
        listener = socket.socket(socket.AF_UNIX)
        self.addCleanup(listener.close)
        listener.bind(str(self.shared / "wayland-2"))
        listener.listen(1)
        attached.record_instance(self.work, {"signature": "nested_sig", "pid": 70, "start": 700,
                                             "wayland": "wayland-2"})
        self.teardown()
        self.assertTrue((self.shared / "wayland-2").exists())

    def test_teardown_reports_a_session_process_that_did_not_survive(self):
        make_proc(self.proc, 60, 601, comm="foot")
        with self.assertRaisesRegex(session.TeardownError, "bystander foot \\(60\\) did not survive"):
            self.teardown()
        self.assertFalse(self.work.exists())

    def test_teardown_reports_a_slice_member_that_outlived_the_stop(self):
        make_proc(self.proc, 80, 800, cgroup="0::/user.slice/ad_e2e_999999_1.slice/run-p80.scope")
        with mock.patch.object(session.os, "kill"), mock.patch.object(session.time, "sleep"), \
                mock.patch.object(session.time, "monotonic", side_effect=[0.0, 10.0]):
            with self.assertRaisesRegex(session.TeardownError, "member survived: \\(80, 800"):
                self.teardown()

    def test_a_stray_attached_run_is_torn_down_without_judging_survivors(self):
        stale = self.shared / "hypr" / "ad_e2e_stale_1"
        stale.mkdir()
        attached.record_fixture(self.work, stale)
        make_proc(self.proc, 60, 601, comm="foot")
        with mock.patch.object(attached_session, "_stop_slice") as stop_slice, \
             mock.patch.object(attached_guard, "refusals", return_value=[]):
            swept = session.sweep_strays(self.base, self.proc)
        self.assertEqual(swept, [self.work])
        stop_slice.assert_called_once()
        self.assertFalse(stale.exists())
        self.assertTrue(self.shared.exists())

    def test_launcher_runs_the_command_in_the_slice_with_only_the_tier_environment(self):
        tier = SimpleNamespace(runtime_dir=self.work)
        script = attached_session.launcher(tier, ["gnome-calculator", "--x=it's"],
                                           {"WAYLAND_DISPLAY": "wayland-1", "A": "b c"}, self.work / "log")
        self.assertEqual(script.read_text(), (
            "#!/bin/sh\nexec systemd-run --user --scope --quiet --collect --slice=ad_e2e_999999_1.slice -- "
            "/usr/bin/env -i 'A=b c' WAYLAND_DISPLAY=wayland-1 gnome-calculator '--x=it'\"'\"'s' "
            f">>{self.work}/log 2>&1\n"))

    def test_only_new_generations_of_the_launched_command_on_this_display_are_adopted(self):
        env = {"XDG_RUNTIME_DIR": str(self.shared), "WAYLAND_DISPLAY": "wayland-1"}
        make_proc(self.proc, 90, 900, comm="gnome-calculato", environ=env)
        make_proc(self.proc, 91, 100, comm="gnome-calculato", environ=env)
        make_proc(self.proc, 92, 900, comm="gnome-calculato", environ=dict(env, WAYLAND_DISPLAY="wayland-2"))
        make_proc(self.proc, 93, 900, comm="xdg-desktop-por", environ=env)
        tier = SimpleNamespace(runtime_dir=self.work, wayland_display="wayland-1")
        adopted = attached_session.adopt_activated(tier, ("gnome-calculator",), 500, self.proc)
        self.assertEqual(adopted, [(90, 900, "gnome-calculato")])
        self.assertEqual(attached.load(self.work)["generations"], [[90, 900, "gnome-calculato"]])

    def test_the_cancel_receipt_names_the_attached_record(self):
        tier = SimpleNamespace(runtime_dir=self.work, name="hyprland")
        (self.work / session.OWNER_MARKER).write_text(f"{os.getpid()} {session._own_start_time()}\n")
        path = cancel.publish(self.suite, tier)
        self.assertEqual(json.loads(path.read_text())["attached"], str(self.suite / "hyprland.attached.json"))

    def test_cleanup_check_needs_a_finished_teardown_and_live_survivors(self):
        mirror = self.suite / "hyprland.attached.json"
        with self.assertRaises(TimeoutError):
            cancel.check_attached_cleanup(self.work, mirror, 0.2, self.proc)
        self.teardown()
        cancel.check_attached_cleanup(self.work, mirror, 0.2, self.proc)
        make_proc(self.proc, 50, 501, comm="Hyprland")
        with self.assertRaisesRegex(RuntimeError, "compositor Hyprland \\(50\\) did not survive"):
            cancel.check_attached_cleanup(self.work, mirror, 0.2, self.proc)


if __name__ == "__main__":
    unittest.main()
