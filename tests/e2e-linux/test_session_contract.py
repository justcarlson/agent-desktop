from __future__ import annotations

import os
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import session

SKIP_LIVE_ENV = "AGENT_DESKTOP_E2E_SKIP_LIVE"


def live_allowed() -> bool:
    return sys.platform.startswith("linux") and not os.environ.get(SKIP_LIVE_ENV)


def make_proc(root: Path, entries: dict[int, tuple[bytes, int]]) -> Path:
    for pid, (environ, start) in entries.items():
        pid_dir = root / str(pid)
        pid_dir.mkdir(parents=True, exist_ok=True)
        (pid_dir / "environ").write_bytes(environ)
        fields = " ".join(["S"] + ["0"] * 18 + [str(start)])
        (pid_dir / "stat").write_text(f"{pid} (comm) {fields}\n")
    return root


class ProcParsingTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.proc = Path(self.root.name) / "proc"

    def test_members_match_the_runtime_dir(self):
        runtime = Path("/run/user/1000/rt.abc")
        make_proc(
            self.proc,
            {
                10: (f"PATH=/bin\x00XDG_RUNTIME_DIR={runtime}\x00".encode(), 100),
                11: (b"XDG_RUNTIME_DIR=/run/user/1000/other\x00", 110),
                12: (b"HOME=/root\x00", 120),
            },
        )
        members = session._proc_members(runtime, self.proc)
        self.assertEqual(members, [(10, 100)])

    def test_start_time_reads_past_a_parenthesised_comm(self):
        pid_dir = self.proc / "42"
        pid_dir.mkdir(parents=True)
        (pid_dir / "stat").write_text(
            "42 (weird (name) x) S " + " ".join(["0"] * 18 + ["7777"]) + "\n"
        )
        self.assertEqual(session._process_start_time(pid_dir), 7777)

    def test_missing_stat_reads_none(self):
        self.assertIsNone(session._process_start_time(self.proc / "999"))


class SweepTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.proc = Path(self.root.name) / "proc"
        self.runtime = Path(self.root.name) / "rt.sweep"
        self.runtime.mkdir()

    def member_env(self) -> bytes:
        return f"XDG_RUNTIME_DIR={self.runtime}\x00".encode()

    def test_sweep_signals_members_by_membership(self):
        make_proc(self.proc, {50: (self.member_env(), 5)})
        calls = []

        def fake_kill(pid, sig):
            calls.append((pid, sig))
            if sig == signal.SIGKILL:
                (self.proc / str(pid) / "environ").unlink(missing_ok=True)

        with mock.patch.object(session, "_kill", fake_kill):
            remaining = session._sweep_members(self.runtime, self.proc)
        self.assertEqual(remaining, [])
        signals = [sig for _, sig in calls]
        self.assertEqual(signals[0], signal.SIGTERM)
        self.assertEqual(signals[1], signal.SIGKILL)

    def test_survivors_are_reported(self):
        make_proc(self.proc, {51: (self.member_env(), 5)})

        def fake_kill(pid, sig):
            return None

        times = iter([0.0, 1.0, 6.0])
        with mock.patch.object(session, "_kill", fake_kill), mock.patch.object(
            session.time, "sleep", lambda s: None
        ), mock.patch.object(session.time, "monotonic", lambda: next(times)):
            remaining = session._sweep_members(self.runtime, self.proc)
        self.assertEqual(remaining, [(51, 5)])


class StraySweepTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.base = Path(self.root.name) / "ad-wt"
        self.proc = Path(self.root.name) / "proc"
        self.base.mkdir()

    def stray(self, name: str, owner_pid: int, owner_start: int) -> Path:
        runtime = self.base / name
        runtime.mkdir()
        (runtime / session.OWNER_MARKER).write_text(f"{owner_pid} {owner_start}\n")
        return runtime

    def test_dead_runner_runtime_is_swept(self):
        runtime = self.stray("rt.dead", 4242, 777)
        swept = session.sweep_strays(self.base, self.proc)
        self.assertEqual(swept, [runtime])

    def test_live_runner_runtime_is_kept(self):
        make_proc(self.proc, {4242: (b"PATH=/bin\x00", 777)})
        runtime = self.stray("rt.live", 4242, 777)
        swept = session.sweep_strays(self.base, self.proc)
        self.assertEqual(swept, [])
        self.assertTrue(runtime.exists())

    def test_recycled_pid_with_new_start_is_swept(self):
        make_proc(self.proc, {4242: (b"PATH=/bin\x00", 999)})
        runtime = self.stray("rt.recycled", 4242, 777)
        swept = session.sweep_strays(self.base, self.proc)
        self.assertEqual(swept, [runtime])


class EnvFileTests(unittest.TestCase):
    def test_parse_env_file_keeps_only_known_keys(self):
        text = (
            "XDG_RUNTIME_DIR=/tmp/rt.a\n"
            "DISPLAY=:101\n"
            "IGNORED=1\n"
            "DBUS_SESSION_BUS_ADDRESS=unix:path=/tmp/rt.a/bus\n"
            "no separator here\n"
        )
        parsed = session._parse_env_file(text)
        self.assertEqual(parsed["XDG_RUNTIME_DIR"], "/tmp/rt.a")
        self.assertEqual(parsed["DISPLAY"], ":101")
        self.assertNotIn("IGNORED", parsed)
        self.assertEqual(len(parsed), 3)

    def test_tier_argv_maps_each_tier(self):
        openbox = session._tier_argv("x11-openbox")
        self.assertIn("--wm", openbox)
        mutter = session._tier_argv("x11-mutter")
        self.assertIn("private-mutter-shell.sh", " ".join(mutter))
        wayland = session._tier_argv("wayland")
        self.assertIn("private-wayland-shell.sh", " ".join(wayland))
        with self.assertRaises(ValueError):
            session._tier_argv("plan9")

    def test_stderr_tail_reads_what_is_there(self):
        proc = subprocess.Popen(
            ["sh", "-c", "echo the-reason >&2; exit 3"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        proc.wait(timeout=10)
        tail = session._stderr_tail(proc)
        proc.stderr.close()
        self.assertEqual(tail, "the-reason")


class StubWrapperTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.base = Path(self.root.name) / "ad-wt"
        self.suite = Path(self.root.name) / "suite"

    def stub_session(self, body: str) -> Path:
        script = Path(self.root.name) / "private-session.sh"
        script.write_text(f"#!/bin/sh\n{body}\n")
        script.chmod(0o700)
        return script

    def healthy_stub(self) -> Path:
        return self.stub_session(
            'while [ "$1" != "--" ]; do shift; done; shift\n'
            'rt=$(mktemp -d "$AGENT_DESKTOP_SESSION_BASE/rt.XXXXXX")\n'
            'export XDG_RUNTIME_DIR=$rt\n'
            'export DBUS_SESSION_BUS_ADDRESS=unix:path=$rt/bus\n'
            'export AGENT_DESKTOP_PRIVATE_SESSION=$rt\n'
            'mkdir -p "$rt/ad-home"\n'
            "printf 'XDG_RUNTIME_DIR=%s\\nDISPLAY=%s\\nDBUS_SESSION_BUS_ADDRESS=%s\\n"
            "AGENT_DESKTOP_PRIVATE_SESSION=%s\\nAGENT_DESKTOP_HOME=%s\\n' \\\n"
            '  "$rt" ":201" "$DBUS_SESSION_BUS_ADDRESS" "$rt" "$rt/ad-home" > "$AD_TIER_ENV_FILE"\n'
            'printf \'%s %s\\n\' "$AD_RUNNER_PID" "$AD_RUNNER_START" > "$rt/harness-owner"\n'
            'trap \'rm -rf "$rt"\' EXIT\n'
            'while [ ! -f "$AD_TIER_RELEASE_FILE" ]; do sleep 0.1; done\n'
        )

    def env(self):
        return {
            "AGENT_DESKTOP_SESSION_BASE": str(self.base),
            "AGENT_DESKTOP_E2E_SUITE": str(self.suite),
        }

    def test_start_tier_builds_and_tears_down(self):
        wrapper = self.healthy_stub()
        with mock.patch.object(session, "SCRIPT_PRIVATE_SESSION", wrapper), \
            mock.patch.object(session, "_a11y_address", lambda env, dl: "unix:path=/fake/a11y"), \
            mock.patch.object(session, "_a11y_warm", lambda env, dl: True), \
            mock.patch.dict(os.environ, self.env()):
            self.base.mkdir()
            with session.start_tier("x11-openbox", 30.0) as tier:
                self.assertTrue(tier.runtime_dir.is_dir())
                self.assertEqual(tier.display, ":201")
                self.assertEqual(tier.a11y_bus_address, "unix:path=/fake/a11y")
                self.assertEqual(tier.env["AT_SPI_BUS_ADDRESS"], "unix:path=/fake/a11y")
            self.assertFalse(tier.runtime_dir.exists())

    def test_unhealthy_wrapper_fails_with_its_reason(self):
        wrapper = self.stub_session(
            'while [ "$1" != "--" ]; do shift; done; shift\n'
            '"$@"\n'
        )
        shell = Path(self.root.name) / "inner.sh"
        shell.write_text(
            "#!/bin/sh\necho the shell did not become healthy: no seat >&2\nexit 1\n"
        )
        shell.chmod(0o700)
        with mock.patch.object(session, "SCRIPT_PRIVATE_SESSION", wrapper), \
            mock.patch.object(session, "SCRIPT_MUTTER_SHELL", shell), \
            mock.patch.dict(os.environ, self.env()):
            self.base.mkdir()
            with self.assertRaises(session.TierUnavailable) as raised:
                with session.start_tier("x11-mutter", 15.0):
                    pass
            self.assertIn("no seat", str(raised.exception))

    def test_cold_registry_fails_the_tier(self):
        wrapper = self.healthy_stub()
        with mock.patch.object(session, "SCRIPT_PRIVATE_SESSION", wrapper), \
            mock.patch.object(session, "_a11y_address", lambda env, dl: "unix:path=/fake/a11y"), \
            mock.patch.object(session, "_a11y_warm", lambda env, dl: False), \
            mock.patch.dict(os.environ, self.env()):
            self.base.mkdir()
            with self.assertRaises(session.TierUnavailable) as raised:
                with session.start_tier("x11-openbox", 30.0):
                    pass
            self.assertIn("registry", str(raised.exception))

    def test_health_gate_timeout_is_named(self):
        wrapper = self.stub_session(
            'while [ "$1" != "--" ]; do shift; done; shift\n'
            "sleep 60\n"
        )
        with mock.patch.object(session, "SCRIPT_PRIVATE_SESSION", wrapper), \
            mock.patch.dict(os.environ, self.env()):
            self.base.mkdir()
            with self.assertRaises(session.TierUnavailable) as raised:
                with session.start_tier("x11-openbox", 1.5):
                    pass
            self.assertIn("health gate", str(raised.exception))


@unittest.skipUnless(live_allowed(), "live session checks run on the box")
class LiveTierTests(unittest.TestCase):
    def test_every_tier_starts_sweeps_and_leaves_nothing(self):
        suite = Path(tempfile.mkdtemp(prefix="ad-e2e-live-suite-"))
        for name in ("x11-openbox", "x11-mutter", "wayland"):
            with self.subTest(tier=name), \
                mock.patch.dict(os.environ, {"AGENT_DESKTOP_E2E_SUITE": str(suite)}):
                with session.start_tier(name, 120.0) as tier:
                    self.assertTrue(tier.runtime_dir.is_dir())
                    self.assertTrue(tier.display or tier.wayland_display)
                    self.assertTrue(tier.a11y_bus_address)
                    members = session._proc_members(tier.runtime_dir)
                    self.assertNotEqual(members, [])
                self.assertFalse(tier.runtime_dir.exists())
                self.assertEqual(session._proc_members(tier.runtime_dir), [])


if __name__ == "__main__":
    unittest.main()
