from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_cursor_controls as controls

CONTROL_LEGS = {
    "cursor.hide_ack_delayed_within_deadline": (("hyprland",), ("headed",), "pass"),
    "cursor.hide_hold_across_agents": (("hyprland",), ("headed",), "pass"),
}


def screen(ok: bool = True, elapsed_ms: float = 260.0) -> Envelope:
    return Envelope(ok=ok, data={} if ok else None, error=None if ok else {"code": "TIMEOUT"}, raw="{}",
                    elapsed_ms=elapsed_ms, argv=[], exit_code=0 if ok else 1)


class RegistryTests(unittest.TestCase):
    def test_each_control_leg_has_its_tier_mode_and_disposition(self):
        for leg_id, (tiers, modes, disposition) in CONTROL_LEGS.items():
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (tiers, modes, disposition))
                self.assertIn("hyprland", leg.oracles)

    def test_each_control_leg_has_an_implementation(self):
        import scenarios  # noqa: F401

        for leg_id in CONTROL_LEGS:
            with self.subTest(leg=leg_id):
                self.assertIn(leg_id, legs.implementations())


class DelayedAckTests(unittest.TestCase):
    GOOD = {"stopped": True, "pending": True, "running_at_resume": True, "file_at_resume": False,
            "written_after": True, "shows": False}

    def test_a_capture_that_waits_for_the_late_ack_passes(self):
        self.assertEqual(controls.delayed_ack_problems(screen(), dict(self.GOOD)), [])

    def test_each_broken_observation_fails(self):
        for key, value in (("stopped", False), ("pending", False), ("running_at_resume", False),
                           ("file_at_resume", True), ("written_after", False), ("shows", True), ("shows", None)):
            with self.subTest(key=key, value=value):
                self.assertTrue(controls.delayed_ack_problems(screen(), {**self.GOOD, key: value}))

    def test_a_refused_capture_fails(self):
        self.assertTrue(controls.delayed_ack_problems(screen(ok=False), dict(self.GOOD)))

    def test_a_capture_faster_than_the_delay_fails(self):
        self.assertTrue(controls.delayed_ack_problems(screen(elapsed_ms=150.0), dict(self.GOOD)))


class HeldAcrossAgentsTests(unittest.TestCase):
    def test_no_cursor_in_any_grab_under_the_hold_passes(self):
        self.assertEqual(controls.held_problems([("after click", False, False), ("after capture", False, False)]),
                         [])

    def test_either_cursor_in_a_grab_under_the_hold_fails(self):
        self.assertTrue(controls.held_problems([("after click", False, True)]))
        self.assertTrue(controls.held_problems([("after capture", True, False)]))

    def test_no_grab_fails(self):
        self.assertTrue(controls.held_problems([]))

    def test_the_hold_must_outlast_the_last_grab(self):
        self.assertTrue(controls.hold_outlasts(10_000, 7_000))
        self.assertFalse(controls.hold_outlasts(10_000, 9_500))


if __name__ == "__main__":
    unittest.main()
