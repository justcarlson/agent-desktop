"""Legs whose hyprland scenario asserts a refusal say so in the registry and in the run log."""
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
from runner import legs, main, report, verdict

HYPRLAND_REFUSALS = (
    "pointer.ref_receiver_evidence",
    "pointer.ref_receiver_evidence_under_cursor",
    "pointer.cursor_pass_through",
    "pointer.session_ref_pointer_refusals",
    "ref_actions.double_click",
    "ref_actions.triple_click",
    "ref_actions.scroll",
)
X11_DELIVERIES = {
    "ref_actions.double_click": "Ref double-click selects a document word and restores selection",
    "ref_actions.triple_click": "Ref triple-click selects a document line and restores selection",
    "ref_actions.scroll": "Ref scroll changes visible Settings rows, then restores",
}
DELIVERY_CLAIMS = ("lands", "reach the pad", "delivery returns", "selects", "changes visible", "logs exactly one press")


def rows_for(leg_id: str, tier: str) -> list[legs.Leg]:
    return [leg for leg in legs.REGISTRY if leg.id == leg_id and tier in leg.tiers]


class HyprlandRefusalRegistryTests(unittest.TestCase):
    def test_each_hyprland_refusal_leg_is_an_asserted_refusal_naming_its_decision(self):
        for leg_id in HYPRLAND_REFUSALS:
            with self.subTest(leg=leg_id):
                rows = rows_for(leg_id, "hyprland")
                self.assertEqual(len(rows), 1)
                self.assertEqual(rows[0].tiers, legs.HYPRLAND_ONLY)
                self.assertEqual(rows[0].disposition, "asserted-refusal")
                self.assertIn("D12", rows[0].reason)
                self.assertIn("D11", rows[0].reason)

    def test_each_hyprland_refusal_requirement_names_the_refusal_and_claims_no_delivery(self):
        for leg_id in HYPRLAND_REFUSALS:
            with self.subTest(leg=leg_id):
                requirement = rows_for(leg_id, "hyprland")[0].requirement
                self.assertIn("receiver_unproven", requirement)
                self.assertIn("not_delivered", requirement)
                claimed = [claim for claim in DELIVERY_CLAIMS if claim in requirement]
                self.assertEqual(claimed, [])

    def test_the_x11_rows_of_shared_legs_still_pass_on_delivery(self):
        for leg_id, requirement in X11_DELIVERIES.items():
            for tier in legs.X11:
                with self.subTest(leg=leg_id, tier=tier):
                    rows = rows_for(leg_id, tier)
                    self.assertEqual(len(rows), 1)
                    self.assertNotIn("hyprland", rows[0].tiers)
                    self.assertEqual((rows[0].disposition, rows[0].requirement), ("pass", requirement))

    def test_split_legs_resolve_per_tier_and_refuse_an_unnamed_tier(self):
        self.assertEqual(legs.leg_by_id("ref_actions.scroll", "hyprland").disposition, "asserted-refusal")
        self.assertEqual(legs.leg_by_id("ref_actions.scroll", "x11-openbox").disposition, "pass")
        self.assertEqual({tier for row in legs.leg_rows("ref_actions.scroll") for tier in row.tiers},
                         {"x11-openbox", "x11-mutter", "hyprland"})
        with self.assertRaises(KeyError):
            legs.leg_by_id("ref_actions.scroll")
        with self.assertRaises(KeyError):
            legs.leg_by_id("ref_actions.scroll", "wayland")


class RefusedAsRequiredLogTests(unittest.TestCase):
    def run_leg(self, disposition: str, impl=None):
        leg = legs.Leg("pointer.fake_refusal", "A fake leg", "pad", legs.HYPRLAND_ONLY, legs.HEADED,
                       ("applog",), disposition, "D11 and D12" if disposition != "pass" else "")

        def recorded(ctx):
            ctx.before("applog.pad", 0)
            ctx.after("applog.pad", 0)
            ctx.restored("applog.pad", 0)

        tier = SimpleNamespace(runtime_dir=Path("/unused"))
        with tempfile.TemporaryDirectory() as directory, \
                mock.patch.object(verdict, "_session_members", return_value=frozenset()):
            return main._run_leg(impl or recorded, leg, tier, "hyprland", "headed", Path(directory), verdict, report)

    def test_a_passing_asserted_refusal_leg_is_reported_refused_as_required(self):
        outcome = self.run_leg("asserted-refusal")
        self.assertEqual(outcome.outcome, report.REFUSED)
        text = report.render([outcome])
        self.assertIn("refused as required  pointer.fake_refusal", text)
        self.assertNotIn("pass", text)
        self.assertEqual(report.count_failures([outcome]), 0)

    def test_a_passing_pass_leg_still_says_pass(self):
        outcome = self.run_leg("pass")
        self.assertEqual(outcome.outcome, report.PASS)
        self.assertIn("pass", report.render([outcome]).splitlines()[2].split())

    def test_a_failing_asserted_refusal_leg_still_fails(self):
        def silent(ctx):
            raise AssertionError("the click was delivered")

        outcome = self.run_leg("asserted-refusal", silent)
        self.assertEqual(outcome.outcome, report.FAIL)
        self.assertEqual(report.count_failures([outcome]), 1)

    def test_the_summary_counts_stay_name_then_number(self):
        outcomes = [report.LegOutcome("a", "hyprland", "headed", report.PASS),
                    report.LegOutcome("b", "hyprland", "headed", report.REFUSED),
                    report.LegOutcome("c", "hyprland", "headless", report.REFUSED)]
        summary = report.render(outcomes).splitlines()[-1]
        self.assertTrue(summary.startswith("summary: "))
        counts = dict(part.rsplit(" ", 1) for part in summary.removeprefix("summary: ").split(", "))
        self.assertEqual(counts, {"pass": "1", "refused as required": "2"})


if __name__ == "__main__":
    unittest.main()
