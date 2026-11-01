"""A plain `python3 -I -m unittest discover` starts no desktop session.

The guard runs the whole discovered suite again in a child interpreter, with
neither the opt-in nor the skip variable set, and every tier launcher replaced
by a recorder that refuses. A real launch reaches a recorder, so nothing
starts even when a gate is broken."""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
INNER_ENV = "AGENT_DESKTOP_E2E_PLAIN_DISCOVER_INNER"
GATE_ENVS = ("AGENT_DESKTOP_E2E_LIVE_SESSIONS", "AGENT_DESKTOP_E2E_SKIP_LIVE")

DRIVER = r"""
import json, os, subprocess, sys, unittest
from pathlib import Path

root, repo, record, report = (Path(arg) for arg in sys.argv[1:5])
sys.path.insert(0, str(root))
from runner import attached_session, session

launches = []
scripts = {str(repo / "scripts" / name) for name in (
    "private-session.sh", "private-mutter-shell.sh", "private-wayland-shell.sh",
    "test-private-session.sh")}
scripts |= {str(root / "run.sh"), str(root / "runner" / "prepare.py"), str(root / "runner" / "main.py")}
programs = {"Xvfb", "Xwayland", "Xorg", "openbox", "mutter", "gnome-shell", "Hyprland",
            "hyprctl", "foot"}
real_private_session = session.SCRIPT_PRIVATE_SESSION


class Refused(OSError):
    pass


def refuse(kind, detail):
    launches.append([kind, detail])
    record.write_text(json.dumps(launches))
    raise Refused(f"plain discover reached a live launcher: {kind} {detail}")


real_popen = subprocess.Popen
real_start_tier = session.start_tier
def start_tier(name, *args, **kwargs):
    launches_for_real = session.subprocess.Popen is real_popen
    if launches_for_real and Path(session.SCRIPT_PRIVATE_SESSION) == Path(real_private_session):
        refuse("session.start_tier", name)
    return real_start_tier(name, *args, **kwargs)
session.start_tier = start_tier

real_attached_start = attached_session.start
def attached_start(deadline, source=None):
    if source is None:
        refuse("attached_session.start", "os.environ")
    return real_attached_start(deadline, source)
attached_session.start = attached_start

real_popen_init = subprocess.Popen.__init__
def popen_init(self, args, *rest, **kwargs):
    argv = [args] if isinstance(args, (str, bytes, os.PathLike)) else list(args)
    words = [os.fsdecode(word) for word in argv]
    if kwargs.get("shell"):
        words = " ".join(words).split()
    for word in words:
        if word in scripts or os.path.basename(word) in programs:
            refuse("subprocess", " ".join(words))
    real_popen_init(self, args, *rest, **kwargs)
subprocess.Popen.__init__ = popen_init

record.write_text("[]")
suite = unittest.TestLoader().discover(str(root), pattern="test_*.py")
result = unittest.TestResult()
suite.run(result)
skipped = [[test.id(), reason] for test, reason in result.skipped]
report.write_text(json.dumps({"launches": launches, "skipped": skipped, "ran": result.testsRun}))
"""


class PlainDiscoverTests(unittest.TestCase):
    @unittest.skipIf(os.environ.get(INNER_ENV), "the guard does not run inside itself")
    @unittest.skipUnless(sys.platform.startswith("linux"), "the harness runs on Linux")
    def test_plain_discover_starts_no_tier(self):
        env = {key: value for key, value in os.environ.items() if key not in GATE_ENVS}
        env[INNER_ENV] = "1"
        with tempfile.TemporaryDirectory() as scratch:
            record = Path(scratch) / "launches.json"
            report_path = Path(scratch) / "report.json"
            env["AGENT_DESKTOP_SESSION_BASE"] = str(Path(scratch) / "sessions")
            result = subprocess.run(
                [sys.executable, "-I", "-c", DRIVER, str(ROOT), str(REPO), str(record), str(report_path)],
                env=env, cwd=scratch, capture_output=True, text=True, timeout=300)
            launches = json.loads(record.read_text()) if record.exists() else None
            self.assertEqual(result.returncode, 0, result.stderr[-2000:])
            self.assertEqual(launches, [])
            report = json.loads(report_path.read_text())
        self.assertEqual(report["launches"], [])
        self.assertGreater(report["ran"], 100)
        live = {test: reason for test, reason in report["skipped"] if ".LiveTierTests." in test}
        self.assertEqual(sorted(test.rsplit(".", 1)[1] for test in live), [
            "test_wayland_starts_sweeps_and_leaves_nothing",
            "test_x11_mutter_starts_sweeps_and_leaves_nothing",
            "test_x11_openbox_starts_sweeps_and_leaves_nothing",
        ])
        self.assertTrue(all(GATE_ENVS[0] in reason for reason in live.values()), live)


if __name__ == "__main__":
    unittest.main()
