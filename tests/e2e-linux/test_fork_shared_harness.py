"""The shared harness must import and pass its unit tests with no fork-only file present.

The fork-only list is FORK_ONLY_PATHS in fork/check-upstream-ancestry.sh. This test builds a
tree without those files and runs the e2e-linux unit tests in it.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HARNESS = Path(__file__).resolve().parent
REPO = HARNESS.parent.parent
CHECKER = REPO / "fork" / "check-upstream-ancestry.sh"
HARNESS_PREFIX = "tests/e2e-linux/"


def fork_only_paths(checker: Path) -> list[str]:
    text = checker.read_text(encoding="utf-8")
    block = re.search(r"^FORK_ONLY_PATHS=\(\n(.*?)^\)", text, re.M | re.S)
    if block is None:
        raise ValueError(f"no FORK_ONLY_PATHS in {checker}")
    return [line.strip() for line in block.group(1).splitlines()
            if line.strip() and not line.strip().startswith("#")]


def harness_entries(paths: list[str]) -> list[str]:
    return [path[len(HARNESS_PREFIX):] for path in paths if path.startswith(HARNESS_PREFIX)]


def link_all_but(source: Path, target: Path, keep: str) -> None:
    target.mkdir(parents=True)
    for entry in source.iterdir():
        if entry.name != keep:
            (target / entry.name).symlink_to(entry)


def shared_tree(root: Path, paths: list[str]) -> Path:
    link_all_but(REPO, root, "tests")
    (root / "fork").unlink(missing_ok=True)
    link_all_but(REPO / "tests", root / "tests", "e2e-linux")
    harness = root / "tests" / "e2e-linux"
    shutil.copytree(HARNESS, harness, ignore=shutil.ignore_patterns("__pycache__"))
    for entry in harness_entries(paths):
        target = harness / entry
        if entry.endswith("/"):
            shutil.rmtree(target, ignore_errors=True)
        else:
            target.unlink(missing_ok=True)
    return harness


@unittest.skipUnless(CHECKER.is_file(), "fork/ is absent outside the fork")
class SharedHarnessTests(unittest.TestCase):
    def test_the_list_names_the_attached_tier(self):
        entries = harness_entries(fork_only_paths(CHECKER))
        self.assertIn("runner/attached.py", entries)
        self.assertIn("runner/attached_session.py", entries)
        self.assertIn("runner/attached_guard.py", entries)

    def test_unit_tests_pass_without_fork_only_files(self):
        with tempfile.TemporaryDirectory(prefix="shared-harness-") as tmp:
            harness = shared_tree(Path(tmp) / "repo", fork_only_paths(CHECKER))
            for entry in harness_entries(fork_only_paths(CHECKER)):
                self.assertFalse((harness / entry).exists(), entry)
            env = dict(os.environ, AGENT_DESKTOP_E2E_SKIP_LIVE="1", PYTHONDONTWRITEBYTECODE="1")
            run = subprocess.run(
                [sys.executable, "-I", "-m", "unittest", "discover", "-s", str(harness), "-p", "test_*.py"],
                cwd=harness.parent.parent, env=env, capture_output=True, text=True, timeout=900,
            )
            self.assertEqual(run.returncode, 0, run.stderr[-6000:])


if __name__ == "__main__":
    unittest.main()
