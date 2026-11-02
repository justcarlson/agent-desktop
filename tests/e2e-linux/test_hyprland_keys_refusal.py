from __future__ import annotations

import ast
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_keys_common as common
from scenarios import hyprland_keys_refusal as refusal

LEG = "input.hyprland_headless_physical_keys_refused"


def envelope(ok: bool, error: dict | None = None, exit_code: int | None = None) -> Envelope:
    code = exit_code if exit_code is not None else (0 if ok else 1)
    return Envelope(ok=ok, data={} if ok else None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=code)


def denied(**changes) -> dict:
    error = {"code": "POLICY_DENIED", "details": {"physical_delivery_started": False},
             "disposition": {"delivery": "not_delivered", "retry": "safe"}}
    error.update(changes)
    return error


class OpaqueFieldFixtureTests(unittest.TestCase):
    def test_the_opaque_mode_shows_an_entry_role_with_no_editable_text(self):
        script = common.APP_SCRIPT.read_text()
        tree = ast.parse(script)
        self.assertIn(f'OPAQUE = "{common.OPAQUE}"', script)
        self.assertIn("Atk.Role.ENTRY", script)
        self.assertIn("Gtk.DrawingArea", script)
        self.assertIn(f'OPAQUE_NAME = "{common.OPAQUE_NAME}"', script)
        self.assertTrue(any(isinstance(node, ast.ImportFrom) and node.module == "gi.repository"
                            and "Atk" in [alias.name for alias in node.names] for node in ast.walk(tree)))

    def test_the_launch_names_the_opaque_mode_only_when_asked(self):
        plain = common.key_log_argv(Path("/py"), Path("/log"), opaque=False)
        opaque = common.key_log_argv(Path("/py"), Path("/log"), opaque=True)
        self.assertEqual(plain, ["/py", str(common.APP_SCRIPT), "/log"])
        self.assertEqual(opaque, [*plain, common.OPAQUE])


class OpaqueTypeRefusalTests(unittest.TestCase):
    def test_a_policy_denied_not_delivered_refusal_passes(self):
        self.assertEqual(refusal.policy_refusal_problems("type", envelope(False, denied())), [])

    def test_a_success_another_code_a_started_delivery_or_another_disposition_fails(self):
        cases = (envelope(True), envelope(False, denied(code="ACTION_FAILED")),
                 envelope(False, denied(details={"physical_delivery_started": True})),
                 envelope(False, denied(disposition={"delivery": "delivery_uncertain", "retry": "unsafe"})),
                 envelope(False, denied(), exit_code=2))
        for case in cases:
            with self.subTest(error=case.error, exit_code=case.exit_code):
                self.assertNotEqual(refusal.policy_refusal_problems("type", case), [])

    def test_the_leg_names_the_physical_type_refusal(self):
        import scenarios  # noqa: F401

        leg = legs.leg_by_id(LEG)
        self.assertIn("POLICY_DENIED", leg.requirement)
        self.assertIn("EditableText", leg.requirement)
        self.assertIn("POLICY_DENIED", leg.reason)


if __name__ == "__main__":
    unittest.main()
