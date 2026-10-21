#!/usr/bin/env python3
"""Exercise the gate's failure and coverage boundary without a desktop."""

from contextlib import redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location("gate", Path(__file__).with_name("linux-live-gate.py"))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class GateTests(unittest.TestCase):
    def test_partition_keeps_every_case_on_each_applicable_tier(self):
        cases = ["unit::fast", "tree::live::one", "tree::live::two", gate.CURSOR_WAYLAND,
                 *(module + "::case" for module in gate.WAYLAND_MODULES),
                 gate.SELF_MUTTER + "::case"]
        ignored = set(cases[1:])
        for jobs in (1, 2, 3):
            shards = gate.plan(cases, ignored, jobs)
            actual = {tier: [name for tasks in workers for task in tasks for name in task["tests"]]
                      for tier, workers in shards.items()}
            self.assertCountEqual(actual["openbox"], cases[:3])
            self.assertCountEqual(actual["mutter"], [*cases[1:3], cases[-1]])
            self.assertCountEqual(actual["wayland"], cases[3:-1])

    def test_receipts_reject_failure_omission_duplicate_and_missing_shard(self):
        for mode in ("pass", "failure", "omission", "duplicate", "setup_failure"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                directory = Path(temporary)
                binary = directory / "libtest"
                binary.write_text(
                    "#!/usr/bin/env python3\nimport sys\nfrom pathlib import Path\n"
                    "names = sys.argv[sys.argv.index('--exact') + 1:]\n"
                    "log = Path(sys.argv[sys.argv.index('--logfile') + 1:][0])\n"
                    f"mode = {mode!r}\n"
                    "lines = []\n"
                    "for name in names:\n"
                    "    if mode != 'omission':\n"
                    "        lines.append(('failed' if mode == 'failure' else 'ok') + ' ' + name)\n"
                    "    if mode == 'duplicate':\n"
                    "        lines.append('ok ' + name)\n"
                    "log.write_text('\\n'.join(lines) + '\\n')\n"
                    "sys.exit(1 if mode == 'failure' else 0)\n"
                )
                binary.chmod(0o700)
                names = ["checks::one", "checks::two", "checks::three"]
                manifest = directory / "manifest.json"
                gate.save(manifest, dict(binary=str(binary), timeout=10,
                                         shards=gate.plan(names, set(names), 3)))
                bounded = gate.run_bounded

                def launch(command, log, timeout):
                    if command[0] != "bash":
                        return bounded(command, log, timeout)
                    if mode == "setup_failure":
                        return 2, 0.0
                    return gate.worker(manifest, command[-2], int(command[-1])), 0.0

                with patch.object(gate, "run_bounded", side_effect=launch), redirect_stdout(io.StringIO()):
                    passed = gate.run_tier(manifest, "openbox")
                summary = json.loads((directory / "openbox/summary.json").read_text())
                self.assertEqual(passed, mode == "pass")
                self.assertEqual(summary["expected"], 3)
                if mode == "pass":
                    self.assertEqual(summary["counts"], {"ok": 3})
                    self.assertTrue(summary["complete"])


if __name__ == "__main__":
    unittest.main()
