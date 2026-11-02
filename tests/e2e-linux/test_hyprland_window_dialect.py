from __future__ import annotations

import contextlib
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from scenarios import hyprland_window_dialect as dialect, hyprland_window_ops as window_ops


class DialectVerdictTests(unittest.TestCase):
    def test_stable_must_answer_a_conf_file_in_the_legacy_dialect(self):
        self.assertEqual(dialect.dialect_problems("stable", False), [])
        self.assertEqual(len(dialect.dialect_problems("stable", True)), 1)

    def test_edge_reads_a_conf_file_as_lua_and_only_says_so(self):
        self.assertEqual(dialect.dialect_problems("edge", True), [])
        self.assertIn("Lua only", dialect.dialect_note("edge", True))
        self.assertIn("legacy", dialect.dialect_note("edge", False))

    def test_a_run_that_names_no_channel_cannot_judge_the_dialect(self):
        self.assertEqual(len(dialect.dialect_problems("", False)), 1)
        self.assertEqual(len(dialect.dialect_problems(None, True)), 1)

    def test_the_note_names_the_channel_and_the_flag(self):
        note = dialect.dialect_note("stable", False)
        self.assertIn("stable", note)
        self.assertIn("lua=False", note)

    def test_the_channel_comes_from_the_vm(self):
        with mock.patch.dict(dialect.os.environ, {"OMARCHY_VM_CHANNEL": "edge"}):
            self.assertEqual(dialect.channel(), "edge")
        with mock.patch.dict(dialect.os.environ, {}, clear=True):
            self.assertIsNone(dialect.channel())


class WindowLegDialectTests(unittest.TestCase):
    """Each window leg records the legacy instance's dialect and fails when stable answers in Lua."""

    def run_leg(self, channel: str, lua: bool) -> tuple[str | None, list[str]]:
        findings: list[str] = []
        ctx = SimpleNamespace(tier=SimpleNamespace(), before=lambda *a: None, after=lambda *a: None,
                              restored=lambda *a: None, finding=findings.append)

        @contextlib.contextmanager
        def yielding(value):
            yield value

        patches = {
            "main_instance": mock.Mock(return_value={}),
            "desktop": mock.Mock(return_value={}),
            "bystander": mock.Mock(return_value={"address": "0xby"}),
            "cursor": mock.Mock(return_value={"x": 0, "y": 0}),
            "restore_desktop": mock.Mock(),
            "parked": mock.Mock(side_effect=lambda *a, **k: yielding(("w-1", "0xa", "31"))),
            "nested": mock.Mock(return_value=({}, True)),
            "legacy_instance": mock.Mock(side_effect=lambda tier: yielding(({}, lua))),
        }
        with contextlib.ExitStack() as stack:
            for name, value in patches.items():
                stack.enter_context(mock.patch.object(window_ops, name, value))
            stack.enter_context(mock.patch.dict(dialect.os.environ, {"OMARCHY_VM_CHANNEL": channel}))
            try:
                window_ops._leg(ctx, "ad-e2e", False, lambda *a: [], lambda *a: [])
            except AssertionError as error:
                return str(error), findings
        return None, findings

    def test_stable_legacy_passes_and_records_the_flag(self):
        failure, findings = self.run_leg("stable", False)
        self.assertIsNone(failure)
        self.assertEqual(findings, [dialect.dialect_note("stable", False)])

    def test_stable_answering_in_lua_fails_the_leg(self):
        failure, findings = self.run_leg("stable", True)
        self.assertIn("stable", failure or "")
        self.assertEqual(findings, [dialect.dialect_note("stable", True)])

    def test_edge_answering_in_lua_passes_and_is_noted_lua_only(self):
        failure, findings = self.run_leg("edge", True)
        self.assertIsNone(failure)
        self.assertIn("Lua only", findings[0])


if __name__ == "__main__":
    unittest.main()
