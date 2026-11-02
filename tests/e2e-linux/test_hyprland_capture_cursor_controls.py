from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_capture_controls as capture
from scenarios import hyprland_cursor_controls as controls

NOT_DELIVERED = "not_delivered"
CONTROL_LEGS = {
    "capture.hyprland_withheld_frame_timeout": (("hyprland",), ("headless",), "asserted-refusal"),
    "cursor.hide_ack_delayed_within_deadline": (("hyprland",), ("headed",), "pass"),
    "cursor.hide_hold_across_agents": (("hyprland",), ("headed",), "pass"),
}
TERMINAL_LEGS = ("capture.hyprland_ghostty_toplevel_pixels", "capture.hyprland_alacritty_toplevel_pixels")


def batch(entry: dict, elapsed_ms: float = 1000.0) -> Envelope:
    return Envelope(ok=True, data={"results": [entry]}, error=None, raw="{}", elapsed_ms=elapsed_ms, argv=[],
                    exit_code=0)


def timed_out(kind: str = "capture_frame") -> dict:
    return {"ok": False, "error": {"code": "TIMEOUT", "disposition": {"delivery": NOT_DELIVERED, "retry": "safe"}, "details": {"kind": kind}}}


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

    def test_a_terminal_leg_is_registered_only_with_its_implementation(self):
        import scenarios  # noqa: F401

        registered = {leg.id for leg in legs.REGISTRY}
        for leg_id in TERMINAL_LEGS:
            with self.subTest(leg=leg_id):
                self.assertEqual(leg_id in registered, leg_id in legs.implementations())


class WithheldFrameTests(unittest.TestCase):
    MARK, ANSWER = 10_000_000_000, 11_000_000_000

    def problems(self, envelope, wrote=False, late_at=None):
        ticks = [(1, self.MARK - 4_000_000_000)] + ([(2, late_at)] if late_at else [])
        return capture.withheld_problems(envelope, wrote, ticks, self.MARK, self.ANSWER)

    def test_a_capture_frame_timeout_while_the_draw_runs_passes(self):
        self.assertEqual(self.problems(batch(timed_out()), late_at=self.MARK + 3_000_000_000), [])

    def test_a_captured_frame_fails(self):
        self.assertTrue(self.problems(batch({"ok": True}), late_at=self.MARK + 3_000_000_000))

    def test_another_timeout_kind_fails(self):
        self.assertTrue(self.problems(batch(timed_out("lock_timeout")), late_at=self.MARK + 3_000_000_000))

    def test_a_written_file_fails(self):
        self.assertTrue(self.problems(batch(timed_out()), wrote=True, late_at=self.MARK + 3_000_000_000))

    def test_no_draw_after_the_request_fails(self):
        self.assertTrue(self.problems(batch(timed_out())))

    def test_a_draw_that_ended_before_the_answer_fails(self):
        self.assertTrue(self.problems(batch(timed_out()), late_at=self.MARK + 500_000_000))

    def test_an_answer_past_the_budget_fails(self):
        self.assertTrue(self.problems(batch(timed_out(), elapsed_ms=2500.0), late_at=self.MARK + 3_000_000_000))


class TerminalArgvTests(unittest.TestCase):
    COMMAND = ["python3", "-I", "painter.py"]

    def test_ghostty_runs_the_command_last_with_its_own_class_and_no_user_config(self):
        argv = capture.TERMINALS["ghostty"]("ad.e2e.ghostty-red", self.COMMAND)
        self.assertEqual(argv[0], "ghostty")
        self.assertEqual(argv[-4:], ["-e", *self.COMMAND])
        for flag in ("--class=ad.e2e.ghostty-red", "--title=ad.e2e.ghostty-red", "--config-default-files=false",
                     "--gtk-single-instance=false", "--window-decoration=none"):
            self.assertIn(flag, argv)

    def test_alacritty_runs_the_command_last_with_its_own_class_and_no_user_config(self):
        argv = capture.TERMINALS["alacritty"]("ad.e2e.alacritty-red", self.COMMAND)
        self.assertEqual(argv[0], "alacritty")
        self.assertEqual(argv[-4:], ["-e", *self.COMMAND])
        self.assertEqual(argv[argv.index("--class") + 1], "ad.e2e.alacritty-red")
        self.assertEqual(argv[argv.index("--config-file") + 1], "/dev/null")

    def test_each_terminal_class_is_a_dotted_application_id(self):
        for kind in capture.TERMINALS:
            for name in capture.terminal_paints(kind):
                with self.subTest(name=name):
                    self.assertEqual(name.count(".") >= 2, True)
                    self.assertTrue(all(part and not part[0].isdigit() for part in name.split(".")))


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
