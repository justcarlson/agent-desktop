from __future__ import annotations

import contextlib
import os
import shutil
import site
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.apps import OwnedApp
from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import hyprland_close as close

CLOSE_LEGS = ("lifecycle.hyprland_close_app_every_window", "lifecycle.hyprland_close_app_gone_app_not_found")
VERIFIED = {"delivery": "delivered_verified", "retry": "unsafe"}
NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}


def stat(pid: int, state: str, start: int) -> str:
    rest = [state] + ["0"] * 18 + [str(start)] + ["0"] * 10
    return f"{pid} (python3) {' '.join(rest)}\n"


def report(ok: bool = True, delivery: str = "delivered_verified") -> Envelope:
    data = {"disposition": {"delivery": delivery, "retry": "unsafe"}} if ok else None
    error = None if ok else {"code": "TIMEOUT", "disposition": {"delivery": delivery}}
    return Envelope(ok=ok, data=data, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0 if ok else 1)


def listing(*apps: tuple[str, int]) -> Envelope:
    data = {"apps": [{"name": name, "pid": pid} for name, pid in apps]}
    return Envelope(ok=True, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)


def toplevel(address: str, pid: int, workspace: str = "special:ad_e2e", **extra) -> dict:
    return {"address": address, "pid": pid, "workspace": {"name": workspace}, "mapped": True, **extra}


class CloseRegistryTests(unittest.TestCase):
    def test_every_close_leg_is_headless_hyprland_reads_compositor_and_proc_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in CLOSE_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "pass"))
                self.assertTrue({"hyprland", "proc"} <= set(leg.oracles))
                self.assertIn(leg_id, implemented)

    def test_pair_app_exists_and_waits_for_both_windows(self):
        text = close.PAIR_SCRIPT.read_text()
        self.assertIn('GLib.set_prgname("close_pair.py")', text)
        self.assertIn("if not open_windows:", text)


class ExitedTests(unittest.TestCase):
    def check(self, content: str | None, start: int) -> bool:
        with tempfile.TemporaryDirectory() as root:
            if content is not None:
                (Path(root) / "41").mkdir()
                (Path(root) / "41" / "stat").write_text(content)
            return close.exited(41, start, Path(root))

    def test_a_missing_zombie_or_reused_pid_has_exited(self):
        self.assertTrue(self.check(None, 900))
        self.assertTrue(self.check(stat(41, "Z", 900), 900))
        self.assertTrue(self.check(stat(41, "S", 901), 900))

    def test_the_same_running_generation_has_not_exited(self):
        self.assertFalse(self.check(stat(41, "S", 900), 900))
        self.assertFalse(self.check(stat(41, "R", 900), 900))


class ListedNameTests(unittest.TestCase):
    def test_the_name_of_the_pid_is_used_when_it_is_unique(self):
        self.assertEqual(close.listed_name(listing(("python3.14", 41), ("gnome-calculator", 42)), 41), "python3.14")

    def test_the_pair_is_named_apart_from_a_background_python_program(self):
        apps = listing(("python3.14", 720), (close.PAIR_EXECUTABLE, 41))
        self.assertEqual(close.listed_name(apps, 41), close.PAIR_EXECUTABLE)

    def test_a_missing_pid_or_a_shared_name_is_refused(self):
        with self.assertRaises(AssertionError):
            close.listed_name(listing(("gnome-calculator", 42)), 41)
        with self.assertRaises(AssertionError):
            close.listed_name(listing(("python3.14", 41), ("Python3.14", 43)), 41)


class CloseVerdictTests(unittest.TestCase):
    def test_kept_state_ignores_geometry_and_other_processes(self):
        before = [toplevel("0xa", 7, at=[0, 0]), toplevel("0xb", 8)]
        after = [toplevel("0xa", 7, at=[50, 50]), toplevel("0xc", 9)]
        self.assertEqual(close.kept_state(before, {7}), close.kept_state(after, {7}))
        moved = [toplevel("0xa", 7, workspace="3")]
        self.assertNotEqual(close.kept_state(before, {7}), close.kept_state(moved, {7}))

    def test_a_remaining_toplevel_a_live_process_or_a_changed_bystander_fails(self):
        kept = [("0xa", 7, "special:ad_e2e", True)]
        self.assertEqual(close.close_problems("close", [], True, (kept, kept)), [])
        self.assertTrue(close.close_problems("close", [toplevel("0xd", 41)], True, (kept, kept)))
        self.assertTrue(close.close_problems("close", [], False, (kept, kept)))
        self.assertTrue(close.close_problems("close", [], True, (kept, [])))

    def test_the_report_must_claim_a_verified_close(self):
        self.assertEqual(close.report_problems("close", report()), [])
        self.assertTrue(close.report_problems("close", report(delivery="delivered_unverified")))
        self.assertTrue(close.report_problems("close", report(ok=False, delivery="delivered_unverified")))

    def test_an_ok_envelope_does_not_pass_a_close_the_compositor_contradicts(self):
        app = OwnedApp("python3.14", 41, 900, [], [], "")
        left = [toplevel("0xd", 41), toplevel("0xa", 7)]
        ctx = SimpleNamespace(tier=object(), before=mock.Mock(), after=mock.Mock(),
                              product=mock.Mock(return_value=report()))
        with mock.patch.object(close, "clients", return_value=left), \
                mock.patch.object(close, "exited", return_value=False), \
                mock.patch.object(close.raw, "poll", return_value=None):
            problems = close._close(ctx, "close", "python3.14", app, {7})
        ctx.product.assert_called_once_with(["close-app", "python3.14"])
        self.assertTrue(any("still lists" in item for item in problems))
        self.assertTrue(any("still running" in item for item in problems))

    def test_a_close_the_compositor_and_proc_confirm_passes(self):
        app = OwnedApp("python3.14", 41, 900, [], [], "")
        reads = iter([[toplevel("0xd", 41), toplevel("0xa", 7)], [toplevel("0xd", 41)]])
        ctx = SimpleNamespace(tier=object(), before=mock.Mock(), after=mock.Mock(),
                              product=mock.Mock(return_value=report()))
        with mock.patch.object(close, "clients", side_effect=lambda tier: next(reads, [toplevel("0xa", 7)])), \
                mock.patch.object(close, "exited", return_value=True), \
                mock.patch.object(close.raw, "poll", return_value=True):
            self.assertEqual(close._close(ctx, "close", "python3.14", app, {7}), [])


class ClosePairPlacementTests(unittest.TestCase):
    def spawn_pair(self) -> mock.Mock:
        work = Path(self.enterContext(tempfile.TemporaryDirectory()))
        (work / "usr" / "bin").mkdir(parents=True)
        python = work / "usr" / "bin" / "python3"
        python.write_text("interpreter")
        python.chmod(0o755)
        tier = SimpleNamespace(env={"PATH": str(python.parent)}, runtime_dir=work)
        spawn = mock.Mock(side_effect=RuntimeError("stop after spawn"))
        with mock.patch.object(close.apps, "_owners", return_value=[]), \
                mock.patch.object(close.attached_session, "spawn", spawn), \
                self.assertRaises(RuntimeError):
            with close.close_pair(tier):
                pass
        return spawn

    def test_the_pair_runs_under_an_executable_no_other_python_program_shares(self):
        argv = self.spawn_pair().call_args.args[1]
        self.assertEqual(Path(argv[0]).name, close.PAIR_EXECUTABLE)
        self.assertEqual(Path(argv[0]).read_text(), "interpreter")
        self.assertEqual(argv[1:], [str(close.PAIR_SCRIPT)])

    def test_the_pair_is_spawned_with_both_toplevels_placed_by_its_class(self):
        spawn = self.spawn_pair()
        self.assertEqual(spawn.call_args.kwargs.get("toplevels"), ("close_pair.py", 2))
        self.assertEqual(close.PAIR_CLASS, "close_pair.py")
        self.assertIn(f'GLib.set_prgname("{close.PAIR_CLASS}")', close.PAIR_SCRIPT.read_text())


class PairInterpreterTests(unittest.TestCase):
    def test_the_copy_is_named_for_the_pair_and_points_at_the_original_library(self):
        with tempfile.TemporaryDirectory() as root:
            python = Path(root) / "python3.14"
            python.write_text("interpreter")
            target = close.pair_interpreter(Path(root), python)
            self.assertEqual(target.name, close.PAIR_EXECUTABLE)
            self.assertEqual(target.read_text(), "interpreter")
            config = (target.parent.parent / "pyvenv.cfg").read_text().splitlines()
            self.assertIn(f"home = {root}", config)
            self.assertIn("include-system-site-packages = true", config)

    @unittest.skipUnless(Path("/proc/self/exe").exists(), "needs /proc")
    def test_the_copy_runs_as_its_own_process_name_with_the_system_packages(self):
        python = Path(shutil.which("python3") or sys.executable).resolve()
        with tempfile.TemporaryDirectory() as root:
            target = close.pair_interpreter(Path(root), python)
            probe = ("import os, site, sys; print(os.path.basename(os.readlink('/proc/self/exe')));"
                     "print(os.pathsep.join(sys.path))")
            result = subprocess.run([str(target), "-I", "-c", probe], capture_output=True, text=True,
                                    timeout=30, env={"PATH": os.defpath}, check=True)
        name, path = result.stdout.splitlines()
        self.assertEqual(name, close.PAIR_EXECUTABLE)
        self.assertEqual(result.stderr, "")
        system = [item for item in site.getsitepackages() if Path(item).is_dir()]
        self.assertTrue(system and set(system) <= set(path.split(os.pathsep)))


class CloseLegAccountingTests(unittest.TestCase):
    """Each close leg, run against a fake compositor, satisfies the runner's oracle accounting."""

    HOLDER = {"address": "0xby", "pid": 1}
    DESKTOP = {"active_window": HOLDER}

    def run_leg(self, leg_id: str) -> list[str]:
        calculator = OwnedApp("gnome-calculator", 50, 500, [], [], "")
        foot = OwnedApp("ad-e2e-close", 60, 600, [], [], "")
        pair = OwnedApp(close.PAIR_EXECUTABLE, 70, 700, ["w-a", "w-b"], [], "")
        live = {1: [toplevel("0xby", 1)], 50: [toplevel("0xcalc", 50)], 60: [toplevel("0xfoot", 60)],
                70: [toplevel("0xa", 70), toplevel("0xb", 70)], 720: []}
        names = {"python3.14": 720, close.PAIR_EXECUTABLE: 70, "gnome-calculator": 50}

        @contextlib.contextmanager
        def yielding(value):
            yield value

        def product(argv):
            if argv[0] == "list-apps":
                return listing(*((name, pid) for name, pid in names.items() if pid in live))
            pid = names[argv[1]]
            if live.pop(pid, None) is None:
                error = {"code": "APP_NOT_FOUND", "message": "gone", "disposition": NOT_DELIVERED}
                return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=argv, exit_code=1)
            return report()

        ctx = LegContext(SimpleNamespace(), "headless", leg=legs.leg_by_id(leg_id),
                         member_reader=lambda tier: frozenset())
        ctx.product = product
        sampler = SimpleNamespace(changes=[])
        patches = {
            "main_instance": mock.Mock(return_value={}),
            "bystander": mock.Mock(return_value=self.HOLDER),
            "desktop": mock.Mock(side_effect=lambda tier, main: dict(self.DESKTOP)),
            "keeps_focus": mock.Mock(return_value=[]),
            "managed": mock.Mock(side_effect=lambda ctx, name: yielding(calculator)),
            "terminal": mock.Mock(side_effect=lambda tier, app_id: yielding((foot, live[60][0]))),
            "close_pair": mock.Mock(side_effect=lambda tier: yielding((pair, live[70]))),
            "clients": mock.Mock(side_effect=lambda tier: [item for found in live.values() for item in found]),
            "exited": mock.Mock(side_effect=lambda pid, start: pid not in live),
            "Sampler": mock.Mock(return_value=mock.MagicMock(__enter__=mock.Mock(return_value=sampler))),
        }
        with contextlib.ExitStack() as stack:
            for name, value in patches.items():
                stack.enter_context(mock.patch.object(close, name, value))
            stack.enter_context(mock.patch.object(close.raw, "poll", lambda check, timeout: check()))
            legs.implementations()[leg_id](ctx)
        return evaluate(ctx)

    def test_every_recorded_oracle_has_one_before_after_and_restored_read(self):
        import scenarios  # noqa: F401

        for leg_id in CLOSE_LEGS:
            with self.subTest(leg=leg_id):
                self.assertEqual(self.run_leg(leg_id), [])


if __name__ == "__main__":
    unittest.main()
