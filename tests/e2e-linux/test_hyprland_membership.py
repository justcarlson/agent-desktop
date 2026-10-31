from __future__ import annotations

import json
import signal
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import proc as oracle_proc
from runner import apps, attached, session

SHARED = "/run/user/1000"
SLICE = attached.slice_name(4242, 77)


def make_proc(root: Path, pid: int, start: int, *, cgroup: str, environ: bytes = b"",
              comm: str = "comm") -> None:
    directory = root / str(pid)
    directory.mkdir(parents=True, exist_ok=True)
    fields = " ".join(["S"] + ["0"] * 18 + [str(start)])
    (directory / "stat").write_text(f"{pid} ({comm}) {fields}\n")
    (directory / "environ").write_bytes(environ or f"XDG_RUNTIME_DIR={SHARED}\0".encode())
    (directory / "cgroup").write_text(cgroup + "\n")


def in_slice(name: str) -> str:
    return f"0::/user.slice/user-1000.slice/user@1000.service/{SLICE}/{name}"


SESSION_CGROUP = "0::/user.slice/user-1000.slice/user@1000.service/session.slice/compositor.service"


class AttachedMembershipTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.proc = self.root / "proc"
        self.work = self.root / "rt.owned"
        self.work.mkdir()
        self.suite = self.root / "suite"
        self.suite.mkdir()
        self.record = attached.new_record(
            slice_name=SLICE, shared_runtime=SHARED, signature="main",
            mirror=self.suite / "hyprland.attached.json")
        attached.save(self.work, self.record)

    def test_slice_name_is_one_flat_unit_named_after_the_runner_generation(self):
        self.assertEqual(SLICE, "ad_e2e_4242_77.slice")
        self.assertNotIn("-", SLICE)

    def test_members_are_slice_processes_and_adopted_generations_not_the_shared_session(self):
        make_proc(self.proc, 10, 100, cgroup=in_slice("run-p10-i1.scope"))
        make_proc(self.proc, 11, 110, cgroup=SESSION_CGROUP)
        make_proc(self.proc, 12, 120, cgroup="0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-x.service")
        make_proc(self.proc, 13, 130, cgroup=f"0::/user.slice/user-1000.slice/user@1000.service/{SLICE}x/a.scope")
        attached.adopt(self.work, 12, 120, "text-editor")
        attached.adopt(self.work, 14, 140, "gone")
        self.assertEqual(session._proc_members(self.work, self.proc), [(10, 100), (12, 120)])
        self.assertEqual([member[:2] for member in oracle_proc.members(self.work, self.proc)],
                         [(10, 100), (12, 120)])

    def test_a_stopped_member_waiting_to_be_reaped_is_not_a_member(self):
        make_proc(self.proc, 10, 100, cgroup=in_slice("run-p10-i1.scope"))
        make_proc(self.proc, 12, 120, cgroup=SESSION_CGROUP)
        attached.adopt(self.work, 12, 120, "text-editor")
        for pid in (10, 12):
            stat = self.proc / str(pid) / "stat"
            stat.write_text(stat.read_text().replace(") S ", ") Z ", 1))
        self.assertEqual(session._proc_members(self.work, self.proc), [])

    def test_an_adopted_generation_is_dropped_when_its_pid_is_reused(self):
        make_proc(self.proc, 12, 999, cgroup=SESSION_CGROUP)
        attached.adopt(self.work, 12, 120, "text-editor")
        self.assertEqual(session._proc_members(self.work, self.proc), [])

    def test_the_record_is_mirrored_into_the_suite(self):
        attached.adopt(self.work, 12, 120, "text-editor")
        mirror = json.loads((self.suite / "hyprland.attached.json").read_text())
        self.assertEqual(mirror, attached.load(self.work))
        self.assertEqual(mirror["generations"], [[12, 120, "text-editor"]])

    def test_kill_signals_only_recorded_generations(self):
        make_proc(self.proc, 10, 100, cgroup=in_slice("run-p10-i1.scope"))
        make_proc(self.proc, 11, 110, cgroup=SESSION_CGROUP)
        with mock.patch.object(session.os, "kill") as kill:
            session._kill(11, signal.SIGTERM, 110, self.work, self.proc)
            session._kill(10, signal.SIGTERM, 101, self.work, self.proc)
            kill.assert_not_called()
            session._kill(10, signal.SIGTERM, 100, self.work, self.proc)
            kill.assert_called_once_with(10, signal.SIGTERM)

    def test_attributable_follows_the_slice_for_the_hyprland_tier(self):
        make_proc(self.proc, 10, 100, cgroup=in_slice("run-p10-i1.scope"))
        make_proc(self.proc, 11, 110, cgroup=SESSION_CGROUP)
        tier = SimpleNamespace(name="hyprland", runtime_dir=self.work)
        env = {"XDG_RUNTIME_DIR": SHARED}
        with mock.patch.object(attached, "PROC_ROOT", self.proc):
            self.assertTrue(apps.attributable(tier, 10, 100, env))
            self.assertFalse(apps.attributable(tier, 11, 110, env))

    def test_an_app_root_on_the_hyprland_tier_is_owned_by_a_run_member(self):
        from oracle import atspi_raw as raw
        make_proc(self.proc, 10, 100, cgroup=in_slice("run-p10-i1.scope"))
        make_proc(self.proc, 11, 110, cgroup=SESSION_CGROUP)
        tier = SimpleNamespace(name="hyprland", runtime_dir=self.work, a11y_bus_address="a11y")
        member, outsider = SimpleNamespace(bus=":1.10", name="Calculator"), SimpleNamespace(
            bus=":1.11", name="Calculator")
        real = oracle_proc.facts
        facts = lambda pid, root=self.proc: real(pid, root)
        with mock.patch.object(attached, "PROC_ROOT", self.proc), \
             mock.patch.object(raw, "connect", return_value=object()), \
             mock.patch.object(raw, "registry", return_value=[outsider, member]), \
             mock.patch.object(raw, "owner_pid", side_effect=lambda node: int(node.bus.split(".")[-1])), \
             mock.patch.object(oracle_proc, "facts", side_effect=facts), \
             mock.patch.object(raw, "RawNode", side_effect=lambda connection, bus, path:
                               {":1.10": member, ":1.11": outsider}[bus]):
            self.assertIs(raw.app_root(tier, ":1.10"), member)
            with self.assertRaises(LookupError):
                raw.app_root(tier, ":1.11")
            self.assertIs(raw.app_root(tier, "Calculator"), member)

    def test_a_private_runtime_without_a_record_keeps_environment_membership(self):
        private = self.root / "rt.private"
        private.mkdir()
        make_proc(self.proc, 20, 200, cgroup=SESSION_CGROUP,
                  environ=f"XDG_RUNTIME_DIR={private}\0".encode())
        self.assertEqual(session._proc_members(private, self.proc), [(20, 200)])

    def test_recorded_paths_must_sit_under_the_shared_hypr_directory(self):
        attached.record_fixture(self.work, Path(SHARED) / "hypr" / "ad_e2e_stale_1")
        for bad in (Path(SHARED), Path(SHARED) / "hypr", Path(SHARED) / "bus",
                    Path(SHARED) / "hypr" / "a" / ".." / ".." / "bus", Path("/tmp/x")):
            with self.subTest(path=bad), self.assertRaises(ValueError):
                attached.record_fixture(self.work, bad)
        self.assertEqual(attached.load(self.work)["fixtures"], [f"{SHARED}/hypr/ad_e2e_stale_1"])


if __name__ == "__main__":
    unittest.main()
