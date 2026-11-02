import json
import unittest

from runner.product import Envelope
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


class EnterLogTests(unittest.TestCase):
    def test_enters_after_the_mark_name_their_widgets(self):
        lines = ['{"ev": "mapped", "pid": 7, "t": 1.0}', "Gtk-WARNING noise",
                 '{"ev": "enter", "widget": "Probe Opener", "t": 2.0}',
                 '{"ev": "enter", "widget": "Probe Popup Item", "t": 3.5}']
        self.assertEqual(hover.entered("\n".join(lines), 3.0), ["Probe Popup Item"])

    def test_a_shown_popup_is_read_from_the_log(self):
        self.assertTrue(hover.popup_shown('{"ev": "popup-shown", "t": 1.0}\n'))
        self.assertFalse(hover.popup_shown('{"ev": "mapped", "pid": 7, "t": 1.0}\n'))


if __name__ == "__main__":
    unittest.main()
