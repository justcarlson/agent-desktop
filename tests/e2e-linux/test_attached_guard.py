from __future__ import annotations

import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import attached_guard, attached_session, main, session

BOOT = "5d2f6a1e-0c1b-4d7e-9a43-6f1e2b3c4d5e"
ROOT_DIR = SimpleNamespace(st_mode=stat.S_IFDIR | 0o755, st_uid=0)


def root_file(mode: int = 0o644, uid: int = 0) -> SimpleNamespace:
    return SimpleNamespace(st_mode=stat.S_IFREG | mode, st_uid=uid)


class Host:
    """A guest where every condition holds; each test breaks exactly one."""

    def __init__(self):
        self.env = {attached_guard.OPT_IN: "1", "HOME": "/home/user",
                    "XDG_RUNTIME_DIR": "/run/user/1000"}
        self.files = {attached_guard.MARKER: root_file()}
        for parent in attached_guard.MARKER.parents:
            self.files[parent] = ROOT_DIR
        self.texts = {attached_guard.MARKER: BOOT + "\n", attached_guard.BOOT_ID: BOOT + "\n"}
        self.virt = subprocess.CompletedProcess([], 0, "kvm\n", "")
        self.dmi = {"sys_vendor": "Dell Inc.", "product_name": "XPS 13", "bios_vendor": "Dell Inc."}

    def lstat(self, path):
        try:
            return self.files[Path(path)]
        except KeyError:
            raise FileNotFoundError(path) from None

    def read(self, path):
        try:
            return self.texts[Path(path)]
        except KeyError:
            raise FileNotFoundError(path) from None

    def detect_virt(self):
        if isinstance(self.virt, BaseException):
            raise self.virt
        return self.virt

    def dmi_field(self, name):
        return self.dmi.get(name, "")

    def refusals(self):
        with mock.patch.object(attached_guard, "_lstat", self.lstat), \
             mock.patch.object(attached_guard, "_read", self.read), \
             mock.patch.object(attached_guard, "_detect_virt", self.detect_virt), \
             mock.patch.object(attached_guard, "_dmi_field", self.dmi_field):
            return attached_guard.refusals(self.env)


class GuardConditionTests(unittest.TestCase):
    def setUp(self):
        self.host = Host()

    def only(self, word: str) -> None:
        reasons = self.host.refusals()
        self.assertEqual(len(reasons), 1, reasons)
        self.assertIn(word, reasons[0])

    def test_a_guest_meeting_every_condition_is_allowed(self):
        self.assertEqual(self.host.refusals(), [])

    def test_a_missing_opt_in_alone_refuses(self):
        del self.host.env[attached_guard.OPT_IN]
        self.only(attached_guard.OPT_IN)

    def test_an_opt_in_other_than_one_refuses(self):
        self.host.env[attached_guard.OPT_IN] = "yes"
        self.only(attached_guard.OPT_IN)

    def test_a_missing_marker_alone_refuses(self):
        del self.host.files[attached_guard.MARKER]
        self.only("marker")

    def test_a_marker_the_user_owns_refuses(self):
        self.host.files[attached_guard.MARKER] = root_file(uid=1000)
        self.only("root")

    def test_a_marker_others_can_write_refuses(self):
        self.host.files[attached_guard.MARKER] = root_file(mode=0o666)
        self.only("writable")

    def test_a_symlinked_marker_refuses(self):
        self.host.files[attached_guard.MARKER] = SimpleNamespace(st_mode=stat.S_IFLNK | 0o777, st_uid=0)
        self.only("regular file")

    def test_a_marker_under_a_user_writable_directory_refuses(self):
        self.host.files[attached_guard.MARKER.parent] = SimpleNamespace(
            st_mode=stat.S_IFDIR | 0o755, st_uid=1000)
        self.only("root")

    def test_a_marker_from_another_boot_refuses(self):
        self.host.texts[attached_guard.MARKER] = "00000000-0000-0000-0000-000000000000\n"
        self.only("boot")

    def test_a_marker_inside_home_or_the_runtime_dir_refuses(self):
        for key in ("HOME", "XDG_RUNTIME_DIR"):
            with self.subTest(key=key):
                self.host = Host()
                self.host.env[key] = str(attached_guard.MARKER.parent.parent)
                self.only(key)

    def test_a_bare_metal_host_alone_refuses(self):
        self.host.virt = subprocess.CompletedProcess([], 1, "none\n", "")
        self.only("virtual machine")

    def test_a_host_that_says_none_is_not_rescued_by_dmi(self):
        self.host.virt = subprocess.CompletedProcess([], 1, "none\n", "")
        self.host.dmi["sys_vendor"] = "QEMU"
        self.only("virtual machine")

    def test_without_systemd_detect_virt_dmi_decides(self):
        self.host.virt = FileNotFoundError("systemd-detect-virt")
        self.only("virtual machine")
        self.host.dmi["sys_vendor"] = "QEMU"
        self.assertEqual(self.host.refusals(), [])

    def test_every_missing_condition_is_named(self):
        del self.host.env[attached_guard.OPT_IN]
        del self.host.files[attached_guard.MARKER]
        self.host.virt = subprocess.CompletedProcess([], 1, "none\n", "")
        self.assertEqual(len(self.host.refusals()), 3)

    def test_the_marker_path_cannot_come_from_the_environment(self):
        self.host.env["AGENT_DESKTOP_E2E_ATTACHED_MARKER"] = "/etc/hostname"
        self.assertEqual(attached_guard.MARKER, Path("/var/lib/agent-desktop-e2e/attached-hyprland-vm"))

    def test_the_real_marker_is_outside_home_and_the_runtime_dir(self):
        for key in ("HOME", "XDG_RUNTIME_DIR"):
            value = os.environ.get(key)
            if value:
                self.assertNotIn(Path(value), attached_guard.MARKER.parents)

    def test_require_raises_a_tier_refusal_naming_each_reason(self):
        with mock.patch.object(attached_guard, "refusals", return_value=["a", "b"]):
            with self.assertRaises(session.TierUnavailable) as caught:
                attached_guard.require({})
        self.assertIn("a; b", str(caught.exception))


class GuardOrderTests(unittest.TestCase):
    """The guard runs before the first session, socket, launch or cleanup call."""

    def refuse(self, env):
        raise attached_guard.Refused("hyprland: refused")

    def test_attach_checks_the_guard_before_the_session(self):
        calls = []
        with mock.patch.object(attached_guard, "require", side_effect=lambda env: calls.append("guard")), \
             mock.patch.object(attached_session, "required_env",
                               side_effect=lambda env: calls.append("session") or (_ for _ in ()).throw(
                                   session.TierUnavailable("stop"))):
            with self.assertRaises(session.TierUnavailable):
                with attached_session.start(5.0, {}):
                    pass
        self.assertEqual(calls, ["guard", "session"])

    def test_a_refused_attach_touches_no_session(self):
        spies = {name: mock.MagicMock() for name in ("required_env", "_compositor", "teardown")}
        with mock.patch.object(attached_guard, "require", side_effect=self.refuse), \
             mock.patch.multiple(attached_session, **spies), \
             mock.patch.object(attached_session.session, "session_base") as base:
            with self.assertRaises(attached_guard.Refused):
                with attached_session.start(5.0, {}):
                    pass
        for name, spy in spies.items():
            self.assertFalse(spy.called, name)
        self.assertFalse(base.called)

    def test_a_refused_runner_exits_before_lease_sweep_or_gate(self):
        with mock.patch.object(attached_guard, "refusals", return_value=["no marker"]), \
             mock.patch.object(main, "_acquire_lease") as lease, \
             mock.patch.object(session, "sweep_strays") as sweep, \
             mock.patch.object(session, "start_tier") as start, \
             mock.patch("runner.gate.check_prerequisites") as gate:
            code = main.main(["--tier", "hyprland"])
        self.assertEqual(code, 2)
        for spy in (lease, sweep, start, gate):
            self.assertFalse(spy.called)

    def test_private_tiers_do_not_consult_the_guard(self):
        with mock.patch.object(attached_guard, "refusals") as guard, \
             mock.patch.object(main, "_acquire_lease", return_value="held"):
            code = main.main(["--tier", "x11-openbox"])
        self.assertEqual(code, 2)
        self.assertFalse(guard.called)

    def test_a_stray_attached_run_is_left_alone_when_the_guard_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            work = base / "rt.stray"
            work.mkdir()
            (work / session.OWNER_MARKER).write_text("999999999 1\n")
            (work / "attached.json").write_text("{}")
            with mock.patch.object(attached_guard, "refusals", return_value=["no marker"]), \
                 mock.patch.object(attached_session, "teardown") as teardown:
                swept = session.sweep_strays(base, proc_root=base / "proc")
            self.assertFalse(teardown.called)
            self.assertEqual(swept, [])
            self.assertTrue(work.is_dir())


class GuestStepTests(unittest.TestCase):
    def test_the_vm_tool_writes_the_marker_as_root_and_opts_in(self):
        text = (ROOT.parent.parent / "test" / "omarchy-vm").read_text()
        self.assertIn(str(attached_guard.MARKER), text)
        self.assertIn("-o root", text)
        self.assertIn(f"-E {attached_guard.OPT_IN}=1", text)

    def test_test_all_checks_the_guard_before_it_starts_the_bystander(self):
        text = (ROOT.parent.parent / "test" / "all").read_text()
        guard = text.index("runner/attached_guard.py")
        self.assertLess(guard, text.index("if start_bystander"))


if __name__ == "__main__":
    unittest.main()
