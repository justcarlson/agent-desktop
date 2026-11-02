import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from hyprland_leg_fakes import FakeContext, delivered, fake_app, nested_patches
from runner.product import Envelope
from runner.verdict import evaluate
from scenarios import hyprland_hover as hover

REFUSAL = {"code": "ACTION_NOT_SUPPORTED", "message": "Hover by ref has no receiver evidence on Hyprland",
           "details": {"kind": "hyprland_pointer", "cause": "hover_unsupported", "physical_delivery_started": False},
           "disposition": {"delivery": "not_delivered", "retry": "safe"}}
TIMED_OUT = {"code": "TIMEOUT", "message": "Timed out waiting for the element to be actionable",
             "disposition": {"delivery": "not_delivered", "retry": "safe"}}


def refused(error: dict, elapsed_ms: float = 120.0) -> Envelope:
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=elapsed_ms,
                    argv=["--headed", "hover"], exit_code=1)


def hovered(point: dict, delivery: str = "delivered_unverified") -> Envelope:
    data = {**point, "disposition": {"delivery": delivery}}
    return Envelope(ok=True, data=data, error=None, raw=json.dumps({"data": data}), elapsed_ms=40.0,
                    argv=["--headed", "hover"], exit_code=0)


class RefRefusalTests(unittest.TestCase):
    def test_a_fast_hover_unsupported_refusal_passes(self):
        self.assertEqual(hover.ref_refusal_problems("gtk4 popup", refused(REFUSAL)), [])

    def test_a_timeout_is_not_the_refusal(self):
        problems = hover.ref_refusal_problems("gtk4 popup", refused(TIMED_OUT, 5200.0))
        self.assertTrue(any("TIMEOUT" in problem for problem in problems), problems)

    def test_a_refusal_that_spends_the_wait_budget_first_fails(self):
        problems = hover.ref_refusal_problems("gtk4 popup", refused(REFUSAL, hover.REFUSAL_BUDGET_MS + 1))
        self.assertTrue(any("ms" in problem for problem in problems), problems)

    def test_a_renamed_cause_fails(self):
        error = {**REFUSAL, "details": {**REFUSAL["details"], "cause": "receiver_unproven"}}
        problems = hover.ref_refusal_problems("plain", refused(error))
        self.assertTrue(any("hover_unsupported" in problem for problem in problems), problems)

    def test_a_refusal_that_started_delivery_fails(self):
        error = {**REFUSAL, "details": {**REFUSAL["details"], "physical_delivery_started": True}}
        self.assertTrue(hover.ref_refusal_problems("plain", refused(error)))

    def test_a_delivered_hover_by_ref_fails(self):
        self.assertTrue(hover.ref_refusal_problems("plain", hovered({"x": 10, "y": 20})))


class CoordinateHoverTests(unittest.TestCase):
    def test_a_delivered_hover_with_the_cursor_on_the_point_passes(self):
        self.assertEqual(hover.xy_hover_problems(hovered({"x": 300, "y": 240}), (300.0, 240.0), (300, 240)), [])

    def test_a_cursor_left_elsewhere_fails(self):
        problems = hover.xy_hover_problems(hovered({"x": 300, "y": 240}), (12.0, 12.0), (300, 240))
        self.assertTrue(any("cursorpos" in problem for problem in problems), problems)

    def test_a_refused_hover_fails(self):
        self.assertTrue(hover.xy_hover_problems(refused(REFUSAL), (300.0, 240.0), (300, 240)))

    def test_a_hover_that_claims_no_delivery_fails(self):
        self.assertTrue(hover.xy_hover_problems(hovered({"x": 300, "y": 240}, "not_delivered"), (300.0, 240.0),
                                                (300, 240)))

    def test_a_hover_reported_at_another_point_fails(self):
        self.assertTrue(hover.xy_hover_problems(hovered({"x": 1, "y": 2}), (300.0, 240.0), (300, 240)))


class OracleReadTests(unittest.TestCase):
    def run_leg(self, leg, answer=lambda argv: None) -> FakeContext:
        with tempfile.TemporaryDirectory() as directory:
            patches = {**nested_patches(), "pads_left": lambda tier: [], "pad_ref": lambda *a: "@s1:e1",
                       "pointer_pad": fake_app(Path(directory), "pad", []), "_refused_quietly": lambda *a: [],
                       "receiver_fixture": fake_app(Path(directory), "fixture", [{"ev": "mapped", "pid": 7}]),
                       "_popup_target": lambda *a: "@s1:e2",
                       "_cursor_on": lambda tier, instance, aim: (float(aim[0]), float(aim[1]))}
            ctx = FakeContext(answer)
            with mock.patch.multiple(hover, **patches):
                leg(ctx)
        return ctx

    def test_the_ref_refusal_leg_records_one_before_after_and_restored_read_per_oracle(self):
        ctx = self.run_leg(hover.hover_by_ref_refused)
        self.assertEqual(evaluate(ctx), [])
        self.assertEqual(set(ctx.reads), {"hyprland.cursor", "applog.pad", "applog.fixture"})

    def test_the_coordinate_leg_records_one_before_after_and_restored_read_per_oracle(self):
        aim = {"x": 380, "y": 415, "disposition": {"delivery": "delivered_unverified"}}
        ctx = self.run_leg(hover.hover_coordinates, lambda argv: delivered(aim))
        self.assertEqual(evaluate(ctx), [])
        self.assertEqual(set(ctx.reads), {"hyprland.cursor", "applog.pad"})


if __name__ == "__main__":
    unittest.main()
