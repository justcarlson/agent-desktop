from __future__ import annotations

import contextlib
import importlib
import socket
import struct
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope

LEG = "session.hyprland_silent_wayland_refused"
HELD_APP = ROOT / "apps" / "held_wayland_socket.py"
RUNTIME = Path("/run/user/1000")
MAIN = {"signature": "live", "pid": 10, "wayland": "wayland-1"}
NESTED = {"signature": "held", "pid": 20, "wayland": "wayland-2"}
NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}


def module():
    return importlib.import_module("scenarios.hyprland_silent_wayland")


def listed(*names: str) -> Envelope:
    data = [{"id": name, "is_primary": index == 0} for index, name in enumerate(names)]
    return Envelope(ok=True, data=data, error=None, raw="[]", elapsed_ms=5.0, argv=[], exit_code=0)


def refused(code: str, elapsed_ms: float = 5.0, **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details, "disposition": NOT_DELIVERED}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=elapsed_ms, argv=[], exit_code=1)


def silent(socket_path: Path, elapsed_ms: float = 1500.0) -> Envelope:
    return refused("TIMEOUT", elapsed_ms, display_unavailable="silent", retryable=True, detail=str(socket_path))


def mismatch(socket_path: Path) -> Envelope:
    detail = f"Hyprland instance live (pid 10) is bound, but the Wayland socket {socket_path} is served by pid 99"
    return refused("PLATFORM_NOT_SUPPORTED", display_unavailable="hyprland_mismatch", detail=detail)


def held_path(behavior: str) -> Path:
    return RUNTIME / f"wayland-0-ad-e2e-held-{behavior}-1"


def foreign_path(behavior: str) -> Path:
    return RUNTIME / f"wayland-0-ad-e2e-foreign-{behavior}-1"


def tier() -> SimpleNamespace:
    return SimpleNamespace(env={"PATH": "/usr/bin", "HOME": "/home/user", "WAYLAND_DISPLAY": "wayland-1",
                                "HYPRLAND_INSTANCE_SIGNATURE": "live"},
                           state_root=Path("/tmp/state"), runtime_dir=Path("/tmp/work"))


class RegistryTests(unittest.TestCase):
    def test_the_leg_is_a_headless_hyprland_asserted_refusal_with_a_reason(self):
        leg = legs.leg_by_id(LEG)
        self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "asserted-refusal"))
        self.assertTrue(leg.reason)
        self.assertIn("hyprland", leg.oracles)
        self.assertIn(LEG, legs.implementations())


class HeldSocketFixtureTests(unittest.TestCase):
    """The fixture app on its own, with `sleep` in place of a compositor: no display is involved."""

    def start(self, behavior: str) -> tuple[subprocess.Popen, Path]:
        scratch = tempfile.TemporaryDirectory(prefix="held.")
        self.addCleanup(scratch.cleanup)
        directory = Path(scratch.name)
        path = directory / f"wayland-0-{behavior}"
        child = subprocess.Popen([sys.executable, "-I", str(HELD_APP), str(path), behavior, "--", "sleep", "30"])
        self.addCleanup(lambda: (child.kill(), child.wait()))
        deadline = time.monotonic() + 5
        while not path.exists():
            self.assertLess(time.monotonic(), deadline, "the fixture never bound its socket")
            time.sleep(0.02)
        return child, path

    @staticmethod
    def connect(path: Path, timeout: float = 0.3) -> socket.socket:
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(timeout)
        client.connect(str(path))
        return client

    def test_the_peer_is_the_exec_process_and_nothing_ever_replies(self):
        for behavior in ("silent", "mute"):
            with self.subTest(behavior=behavior):
                child, path = self.start(behavior)
                with self.connect(path) as client:
                    peer = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[0]
                    client.sendall(b"\x01\x00\x00\x00\x01\x00\x0c\x00\x02\x00\x00\x00")
                    with self.assertRaises(TimeoutError):
                        client.recv(64)
                self.assertEqual(peer, child.pid)

    def test_silent_never_accepts_and_mute_accepts_every_client(self):
        for behavior, fills in (("silent", True), ("mute", False)):
            with self.subTest(behavior=behavior):
                _, path = self.start(behavior)
                clients, full = [], False
                with contextlib.ExitStack() as stack:
                    for _ in range(100):
                        try:
                            clients.append(stack.enter_context(self.connect(path)))
                        except (TimeoutError, BlockingIOError):
                            full = True
                            break
                self.assertEqual(full, fills)

    def test_the_socket_goes_when_the_exec_process_ends(self):
        child, path = self.start("mute")
        child.kill()
        child.wait()
        deadline = time.monotonic() + 3
        while path.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertFalse(path.exists())

    def test_a_bad_behavior_is_an_argument_error(self):
        result = subprocess.run([sys.executable, "-I", str(HELD_APP), "/nonexistent/x", "loud", "--", "true"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)


class DiscoveryOrderTests(unittest.TestCase):
    def test_the_held_socket_sorts_before_any_numbered_compositor_socket(self):
        first = module().reads_first
        self.assertTrue(first(held_path("mute"), "wayland-1"))
        self.assertTrue(first(held_path("mute"), "wayland-12"))
        self.assertFalse(first(RUNTIME / "wayland-3-late", "wayland-1"))


class LegTests(unittest.TestCase):
    def run_leg(self, answer, launch=None):
        silent_wayland = module()
        self.stops = mock.Mock()
        self.launched = []
        self.foreign = {}

        def held(_tier, behavior):
            self.launched.append(behavior)
            if launch is not None:
                launch(behavior)
            return dict(NESTED), held_path(behavior)

        def listener(_tier, behavior):
            self.foreign[behavior] = mock.Mock()
            return foreign_path(behavior), self.foreign[behavior]

        ctx = SimpleNamespace(tier=tier(), mode="headless", before=mock.Mock(), after=mock.Mock(),
                              restored=mock.Mock(), product=lambda argv, env, headed: answer(env))
        with mock.patch.object(silent_wayland, "shared", return_value=RUNTIME), \
                mock.patch.object(silent_wayland, "main_instance", return_value=MAIN), \
                mock.patch.object(silent_wayland, "compositor_names", return_value=["A-1"]), \
                mock.patch.object(silent_wayland, "desktop", return_value={"focus": "x"}), \
                mock.patch.object(silent_wayland, "held_instance", side_effect=held), \
                mock.patch.object(silent_wayland, "foreign_listener", side_effect=listener), \
                mock.patch.object(silent_wayland, "release", self.stops), \
                mock.patch.object(silent_wayland.second_instance, "stop"):
            silent_wayland.hyprland_silent_wayland_refused(ctx)

    @staticmethod
    def contract(env) -> Envelope:
        signature, display = env.get("HYPRLAND_INSTANCE_SIGNATURE"), env.get("WAYLAND_DISPLAY")
        if signature == "live":
            if display and "foreign" in display:
                return mismatch(RUNTIME / display)
            return listed("A-1")
        return silent(RUNTIME / display) if display else silent(held_path(env["_behavior"]))

    def answer_for(self, override=None):
        def answer(env):
            if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "held" and not env.get("WAYLAND_DISPLAY"):
                env = dict(env, _behavior=self.launched[-1])
            if override is not None:
                result = override(env)
                if result is not None:
                    return result
            return self.contract(env)
        return answer

    def test_the_leg_passes_launches_each_behavior_and_releases_each_instance_and_listener(self):
        self.run_leg(self.answer_for())
        self.assertEqual(self.launched, ["silent", "mute"])
        self.assertEqual(self.stops.call_count, 2)
        for behavior in ("silent", "mute"):
            with self.subTest(behavior=behavior):
                self.foreign[behavior].close.assert_called_once()

    def test_a_named_held_socket_reported_as_a_mismatch_fails(self):
        override = lambda env: (mismatch(RUNTIME / env["WAYLAND_DISPLAY"])
                                if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "held" and env.get("WAYLAND_DISPLAY")
                                else None)
        with self.assertRaises(AssertionError):
            self.run_leg(self.answer_for(override))
        self.assertEqual(self.stops.call_count, 1)

    def test_discovery_that_skips_the_held_socket_and_binds_fails(self):
        override = lambda env: (listed("B-1") if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "held"
                                and not env.get("WAYLAND_DISPLAY") else None)
        with self.assertRaises(AssertionError):
            self.run_leg(self.answer_for(override))

    def test_a_refusal_past_the_budget_fails(self):
        override = lambda env: (silent(RUNTIME / env["WAYLAND_DISPLAY"], elapsed_ms=3100.0)
                                if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "held" and env.get("WAYLAND_DISPLAY")
                                else None)
        with self.assertRaises(AssertionError):
            self.run_leg(self.answer_for(override))

    def test_a_foreign_socket_that_binds_or_hangs_fails(self):
        for name, result in {"binds": listed("A-1"), "hangs": silent(foreign_path("silent"), elapsed_ms=3100.0)}.items():
            with self.subTest(name=name):
                override = lambda env, result=result: (result if "foreign" in (env.get("WAYLAND_DISPLAY") or "")
                                                       else None)
                with self.assertRaises(AssertionError):
                    self.run_leg(self.answer_for(override))

    def test_discovery_by_the_session_signature_must_skip_a_foreign_socket(self):
        override = lambda env: (mismatch(foreign_path("silent")) if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "live"
                                and not env.get("WAYLAND_DISPLAY") else None)
        with self.assertRaises(AssertionError):
            self.run_leg(self.answer_for(override))

    def test_a_failed_launch_releases_nothing_it_did_not_start_and_raises(self):
        def launch(behavior):
            if behavior == "mute":
                raise RuntimeError("no instance")
        with self.assertRaises(RuntimeError):
            self.run_leg(self.answer_for(), launch)
        self.assertEqual(self.stops.call_count, 1)


if __name__ == "__main__":
    unittest.main()
