import os
import selectors
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch, Mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
from runner import session


class RunnerRegressionTests(unittest.TestCase):
    def test_hup_overrides_ignore_and_unwinds_private_members(self):
        script = r'''import os, signal, sys
from pathlib import Path
from unittest.mock import patch
sys.path.insert(0, sys.argv[1])
from runner import main, gate, session, stage
root = Path(sys.argv[2])
signal.signal(signal.SIGHUP, signal.SIG_IGN)
def ready(proc, name, env_file, deadline):
    runtime = root / "rt.hup"
    runtime.mkdir()
    (runtime / "state").mkdir()
    (root / "member").write_text(str(proc.pid))
    return {"XDG_RUNTIME_DIR": str(runtime), "AGENT_DESKTOP_PRIVATE_SESSION": str(runtime),
            "AGENT_DESKTOP_HOME": str(runtime / "state"), "DBUS_SESSION_BUS_ADDRESS": "unix:path=/fake/bus"}
def run(*args):
    with session.start_tier("x11-openbox", 10) as tier:
        print("READY", (root / "member").read_text(), flush=True)
        signal.pause()
with (
    patch.object(main, "_acquire_lease", return_value=None),
    patch.object(gate, "check_prerequisites", return_value=[]),
    patch.object(stage, "stage_binary", return_value=root / "binary"),
    patch.object(session, "sweep_strays"),
    patch.object(main, "_run_tier", side_effect=run),
    patch.object(session, "_tier_argv", return_value=[sys.executable, "-c", "import signal; signal.pause()"]),
    patch.object(session, "_wait_for_health", side_effect=ready),
    patch.object(session, "_a11y_address", return_value="unix:path=/fake/a11y"),
    patch.object(session, "_a11y_warm", return_value=True),
    patch.object(session, "_proc_members", return_value=[]),
):
    main.main(["--tier", "x11-openbox", "--leg", "lifecycle.move_window"])
'''
        with tempfile.TemporaryDirectory() as directory:
            env = dict(os.environ, AGENT_DESKTOP_E2E_SUITE=directory)
            child = subprocess.Popen([sys.executable, "-I", "-c", script,
                                      str(Path(__file__).resolve().parent), directory],
                                     env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            member = None
            try:
                with selectors.DefaultSelector() as ready:
                    ready.register(child.stdout, selectors.EVENT_READ)
                    self.assertTrue(ready.select(10), "runner did not enter its tier")
                line = child.stdout.readline().split()
                self.assertTrue(line, child.stderr.read() if child.poll() is not None else "missing readiness")
                self.assertEqual(line[0], "READY")
                member = int(line[1])
                child.send_signal(signal.SIGHUP)
                child.communicate(timeout=10)
                self.assertEqual(child.returncode, 129)
                self.assertFalse((Path(directory) / "rt.hup").exists())
                with self.assertRaises(ProcessLookupError):
                    os.kill(member, 0)
                member = None
            finally:
                if child.poll() is None:
                    child.kill()
                child.communicate(timeout=5)
                if member is not None:
                    try:
                        os.kill(member, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    def test_pre_handshake_timeout_sweeps_only_the_new_owned_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base, suite = root / "base", root / "suite"
            base.mkdir()
            old = base / "rt.existing"
            old.mkdir()
            owner = f"{os.getpid()} {session._own_start_time()}"
            (old / session.OWNER_MARKER).write_text(owner)
            runtime = base / "rt.starting"
            wrapper = Mock(pid=555, stderr=None)
            wrapper.poll.return_value = None
            wrapper.wait.side_effect = [subprocess.TimeoutExpired("wrapper", 20),
                                        subprocess.TimeoutExpired("wrapper", 4), 0]
            members = []
            order = []
            def unhealthy(*args):
                runtime.mkdir()
                (runtime / session.OWNER_MARKER).write_text(owner)
                members.append((777, 123))
                raise session.TierUnavailable("health gate timeout before readiness")
            def sweep(path, proc_root):
                self.assertEqual(path, runtime)
                order.append("sweep")
                members.clear()
                return []
            def send(pid, sig):
                if sig == signal.SIGKILL:
                    self.assertFalse(members, "wrapper was killed before sweeping its members")
                    order.append("kill")
            with patch.dict(os.environ, AGENT_DESKTOP_SESSION_BASE=str(base), AGENT_DESKTOP_E2E_SUITE=str(suite)), \
                 patch.object(session.subprocess, "Popen", return_value=wrapper), \
                 patch.object(session, "_wait_for_health", side_effect=unhealthy), \
                 patch.object(session, "_signal_process", side_effect=send), \
                 patch.object(session, "_sweep_members", side_effect=sweep), \
                 patch.object(session, "_remove_tree", return_value=True) as remove:
                with self.assertRaisesRegex(session.TierUnavailable, "before readiness"):
                    with session.start_tier("x11-openbox", 1):
                        self.fail("unhealthy tier yielded")
            self.assertIn("sweep", order)
            self.assertLess(order.index("sweep"), order.index("kill"))
            remove.assert_called_once_with(runtime)
            self.assertTrue(old.exists())
            self.assertTrue((suite / "x11-openbox.release").exists())
