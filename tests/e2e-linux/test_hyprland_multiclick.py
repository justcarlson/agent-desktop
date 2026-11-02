import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from hyprland_leg_fakes import FakeContext, fake_app, nested_patches
from runner.product import Envelope
from runner.verdict import evaluate
from scenarios import hyprland_multiclick as multiclick

TARGET = "Probe Target"


def clicked(*widgets: str) -> list[dict]:
    return [{"ev": "mapped", "pid": 7, "t": 1.0}] + [{"ev": "clicked", "widget": w, "t": 2.0} for w in widgets]


def report(count: int, receiver_proof=False, delivered_events: int | None = None,
           delivery: str | None = "delivered_unverified") -> Envelope:
    disposition = {} if delivery is None else {"delivery": delivery}
    data = {"x": 300, "y": 200, "count": count, "disposition": disposition,
            "details": {"receiver_proof": receiver_proof,
                        "delivered_events": 2 * count if delivered_events is None else delivered_events}}
    return Envelope(ok=True, data=data, error=None, raw=json.dumps({"data": data}), elapsed_ms=90.0,
                    argv=["--headed", "mouse-click"], exit_code=0)


def refused() -> Envelope:
    error = {"code": "INVALID_ARGS", "details": {"cause": "point_off_output"},
             "disposition": {"delivery": "not_delivered", "retry": "safe"}}
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=10.0,
                    argv=["--headed", "mouse-click"], exit_code=1)


class FixtureClickTests(unittest.TestCase):
    def test_one_click_per_press_on_the_target_passes(self):
        for count in (2, 3):
            with self.subTest(count=count):
                self.assertEqual(multiclick.fixture_click_problems("gtk4", report(count), clicked(*[TARGET] * count),
                                                                   count), [])

    def test_a_wrong_count_fails(self):
        problems = multiclick.fixture_click_problems("gtk4", report(3), clicked(TARGET, TARGET), 3)
        self.assertTrue(any("expected 3" in problem for problem in problems), problems)

    def test_no_click_fails(self):
        problems = multiclick.fixture_click_problems("qt6", report(2), clicked(), 2)
        self.assertTrue(any("expected 2" in problem for problem in problems), problems)

    def test_clicks_on_another_widget_fail(self):
        self.assertTrue(multiclick.fixture_click_problems("gtk3", report(2), clicked("Probe Other", "Probe Other"), 2))

    def test_a_report_that_claims_receiver_proof_fails(self):
        problems = multiclick.fixture_click_problems("gtk4", report(2, receiver_proof=True), clicked(TARGET, TARGET), 2)
        self.assertTrue(any("receiver_proof" in problem for problem in problems), problems)

    def test_a_report_without_delivered_events_fails(self):
        self.assertTrue(multiclick.fixture_click_problems("gtk4", report(2, delivered_events=0),
                                                          clicked(TARGET, TARGET), 2))

    def test_a_report_with_another_delivery_fails(self):
        for delivery in ("delivered_verified", "not_delivered", None):
            with self.subTest(delivery=delivery):
                problems = multiclick.fixture_click_problems("gtk4", report(2, delivery=delivery),
                                                             clicked(TARGET, TARGET), 2)
                self.assertTrue(any("delivered_unverified" in problem for problem in problems), problems)

    def test_a_refused_click_fails(self):
        self.assertTrue(multiclick.fixture_click_problems("gtk4", refused(), clicked(), 2))

    def test_a_report_of_another_count_fails(self):
        self.assertTrue(multiclick.fixture_click_problems("gtk4", report(2), clicked(TARGET, TARGET, TARGET), 3))


class OracleReadTests(unittest.TestCase):
    def test_the_leg_records_one_before_after_and_restored_read_per_oracle(self):
        def click(ctx, env, log, label, centre):
            with log.open("a") as out:
                out.writelines(json.dumps({"ev": "clicked", "widget": TARGET}) + "\n" for _ in range(5))
            return []
        with tempfile.TemporaryDirectory() as directory:
            patches = {**nested_patches(), "fixture_ref": lambda *a: "@s1:e1",
                       "receiver_fixture": fake_app(Path(directory), "fixture", [{"ev": "mapped", "pid": 7}]),
                       "refused_without_input": lambda *a, **kw: [], "target_centre": lambda *a: (300.0, 200.0),
                       "coordinate_clicks": click}
            ctx = FakeContext(lambda argv: None)
            with mock.patch.multiple(multiclick, **patches):
                multiclick.coordinate_multi_click_xwayland(ctx)
        self.assertEqual(evaluate(ctx), [])
        self.assertEqual(set(ctx.reads), {"hyprland.cursor", "applog.fixture"})


if __name__ == "__main__":
    unittest.main()
