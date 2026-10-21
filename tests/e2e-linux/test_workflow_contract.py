import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent


class WorkflowTests(unittest.TestCase):
    def test_live_workflow_fails_closed(self):
        workflow = (REPO / ".github" / "workflows" / "linux-e2e.yml").read_text()
        self.assertIn("workflow_dispatch", workflow)
        self.assertIn("LINUX_E2E_RUNNER", workflow)
        self.assertNotIn("continue-on-error", workflow)
        self.assertIn("tests/e2e-linux/run.sh", workflow)

    def test_hosted_lane_runs_the_self_tests(self):
        ci = (REPO / ".github" / "workflows" / "ci.yml").read_text()
        self.assertIn("discover -s tests/e2e-linux", ci)
        self.assertIn("tests/e2e-linux/run.sh", ci)
        self.assertIn("AGENT_DESKTOP_E2E_SKIP_LIVE", ci)



class SourceRuleTests(unittest.TestCase):
    def test_remote_lock_wait_can_be_bounded_separately_from_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            for name, body in (("ssh", 'printf "%s\\n" "$*"'), ("rsync", "exit 0")):
                path = Path(directory) / name
                path.write_text("#!/bin/sh\n" + body + "\n")
                path.chmod(0o700)
            env = dict(os.environ, PATH=directory + os.pathsep + os.environ["PATH"],
                       AGENT_DESKTOP_REMOTE_TIMEOUT="780",
                       AGENT_DESKTOP_REMOTE_LOCK_TIMEOUT="30")
            result = subprocess.run(["bash", str(REPO / "scripts/remote-linux.sh"),
                                     "-c", "true"], env=env, capture_output=True, text=True,
                                    timeout=10, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("flock -o -w 30", result.stdout)
        self.assertIn("timeout -k 30 780", result.stdout)

    def test_harness_cleanup_does_not_depend_on_a_trash_command(self):
        names = ("private-session.sh", "private-wayland-shell.sh", "remote-linux.sh",
                 "test-private-session.sh", "check-elf-glibc-floor-selftest.sh",
                 "ci-ffi-helper-discovery-smoke.sh")
        trash = re.compile(r"\btrash\b")
        paths = [REPO / "scripts" / name for name in names]
        paths += sorted((ROOT / "runner").glob("*.py")) + sorted((ROOT / "scenarios").glob("*.py"))
        paths += sorted((REPO / "tests" / "linux-dist").glob("run-*.sh"))
        paths = [path for path in paths if path.exists()]
        self.assertEqual([str(path) for path in paths if trash.search(path.read_text())], [])

    def test_rm_scan_covers_e2e_linux(self):
        contract = (REPO / "tests" / "e2e" / "test_harness_contract.py").read_text()
        self.assertIn("e2e-linux", contract)

    def test_runner_files_stay_under_the_line_cap(self):
        over = [
            path.name
            for path in (ROOT / "runner").glob("*.py")
            if len(path.read_text().splitlines()) >= 400
        ]
        self.assertEqual(over, [])
