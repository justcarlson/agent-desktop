#!/usr/bin/env python3
"""Exercise smoke staging against a package containing installed executables."""
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


class SmokeStagingTests(unittest.TestCase):
    def test_staging_preserves_installed_binaries_on_failure_success_and_overlap(self):
        system = platform.system()
        arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x64"}[platform.machine()]
        name = f"agent-desktop-{'darwin' if system == 'Darwin' else 'linux'}-{arch}"
        helper = "agent-desktop-macos-helper" if system == "Darwin" else "agent-desktop-linux-clipboard-owner"
        for scenario in ("missing-source", "missing-helper", "success", "overlap"):
            with self.subTest(scenario=scenario), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                (root / "scripts").mkdir()
                shutil.copy(ROOT / "scripts/ci-npm-wrapper-smoke.sh", root / "scripts/")
                shutil.copytree(ROOT / "npm/lib", root / "npm/lib")
                (root / "npm/bin").mkdir()
                shutil.copy(ROOT / "npm/bin/agent-desktop.js", root / "npm/bin/")
                installed = [root / "npm/bin" / name, root / "npm/bin" / helper]
                for binary in installed:
                    binary.write_bytes(b"installed executable sentinel")
                    binary.chmod(0o751)
                (root / "target/release").mkdir(parents=True)
                output = {"ok": True, "data": {"version": "0.0.0", "os": "macos" if system == "Darwin" else "linux"}}
                if scenario != "missing-source":
                    binary = root / "target/release/agent-desktop"
                    binary.write_text("#!/bin/sh\nprintf '%s\\n' '" + json.dumps(output) + "'\n")
                    binary.chmod(0o755)
                if scenario != "missing-helper":
                    (root / "target/release" / helper).write_text("helper")
                tools = root / "tools"
                tools.mkdir()
                trash = tools / "trash"
                trash.write_text('#!/bin/sh\nmv "$1" "$1.trashed"\n')
                trash.chmod(0o755)
                env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"], TMPDIR=str(root))
                runs = [subprocess.Popen(["bash", "scripts/ci-npm-wrapper-smoke.sh"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2 if scenario == "overlap" else 1)]
                results = [(run.communicate(timeout=20), run) for run in runs]
                for binary in installed:
                    self.assertEqual(binary.read_bytes(), b"installed executable sentinel")
                    self.assertEqual(binary.stat().st_mode & 0o777, 0o751)
                for (stdout, stderr), run in results:
                    self.assertEqual(run.returncode == 0, scenario in ("success", "overlap"), (stdout, stderr))
                self.assertFalse([path for path in root.glob("agent-desktop-npm.*") if not path.name.endswith(".trashed")])
                self.assertEqual(len(list(root.glob("agent-desktop-npm.*.trashed"))), len(runs))


if __name__ == "__main__":
    unittest.main()
