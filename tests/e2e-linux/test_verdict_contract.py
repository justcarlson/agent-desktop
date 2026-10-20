from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import verdict
from runner.legs import Leg
from runner.product import Envelope
from runner.session import Tier
from runner.verdict import LegContext


def fake_tier(root: Path) -> Tier:
    runtime = Path(root) / "rt.test"
    runtime.mkdir(exist_ok=True)
    return Tier(
        name="x11-openbox",
        runtime_dir=runtime,
        display=":100",
        wayland_display=None,
        bus_address="unix:path=/nonexistent/bus",
        a11y_bus_address="unix:path=/nonexistent/atspi",
        state_root=runtime,
        keyfile=runtime / "keyfile",
        env={},
    )


def atspi_leg() -> Leg:
    return Leg(
        id="t.leg",
        requirement="r",
        target="t",
        tiers=("x11-openbox",),
        modes=("headless",),
        oracles=("atspi", "proc"),
        disposition="pass",
    )


class VerdictRuleTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.tier = fake_tier(Path(self.root.name))
        self.members = [frozenset({(10, 100), (11, 110)})]

    def ctx(self, leg=None, mode="headless") -> LegContext:
        return LegContext(
            self.tier, mode, leg, member_reader=lambda tier: self.members[0]
        )

    def test_a_leg_with_no_oracle_read_fails(self):
        failures = verdict.evaluate(self.ctx())
        self.assertTrue(any("before" in f for f in failures))
        self.assertTrue(any("after" in f for f in failures))

    def test_before_without_after_fails(self):
        ctx = self.ctx()
        ctx.before("atspi", {"name": "calc"})
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("after" in f for f in failures))

    def test_after_without_before_fails(self):
        ctx = self.ctx()
        ctx.after("atspi", {"name": "calc"})
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("before" in f for f in failures))

    def test_unchanged_reads_pass(self):
        ctx = self.ctx()
        ctx.before("atspi", {"display": "7"})
        ctx.after("atspi", {"display": "7"})
        self.assertEqual(verdict.evaluate(ctx), [])

    def test_change_with_matching_restoration_passes(self):
        ctx = self.ctx()
        ctx.before("atspi", "off")
        ctx.after("atspi", "on")
        ctx.restored("atspi", "off")
        self.assertEqual(verdict.evaluate(ctx), [])

    def test_change_without_restoration_fails(self):
        ctx = self.ctx()
        ctx.before("atspi", "off")
        ctx.after("atspi", "on")
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("no restoration" in f for f in failures))

    def test_change_with_wrong_restoration_fails(self):
        ctx = self.ctx()
        ctx.before("atspi", "off")
        ctx.after("atspi", "on")
        ctx.restored("atspi", "half")
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("not restored" in f for f in failures))

    def test_restoration_disagreeing_without_change_fails(self):
        ctx = self.ctx()
        ctx.before("atspi", "off")
        ctx.after("atspi", "off")
        ctx.restored("atspi", "half")
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("drifted" in f for f in failures))

    def test_restoration_alone_fails(self):
        ctx = self.ctx()
        ctx.restored("atspi", "off")
        failures = verdict.evaluate(ctx)
        self.assertNotEqual(failures, [])

    def test_joined_process_fails_the_leg(self):
        ctx = self.ctx()
        ctx.before("atspi", "a")
        ctx.after("atspi", "a")
        self.members[0] = self.members[0] | {(12, 120)}
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("owned-process set changed" in f for f in failures))
        self.assertTrue(any("joined" in f for f in failures))

    def test_departed_process_fails_the_leg(self):
        ctx = self.ctx()
        ctx.before("atspi", "a")
        ctx.after("atspi", "a")
        self.members[0] = frozenset({(10, 100)})
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("owned-process set changed" in f for f in failures))
        self.assertTrue(any("left" in f for f in failures))

    def test_restarted_process_same_pid_different_start_fails(self):
        ctx = self.ctx()
        ctx.before("atspi", "a")
        ctx.after("atspi", "a")
        self.members[0] = frozenset({(10, 100), (11, 999)})
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("owned-process set changed" in f for f in failures))


class ReadDisciplineTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.tier = fake_tier(Path(self.root.name))

    def ctx(self, leg=None) -> LegContext:
        return LegContext(self.tier, "headless", leg, member_reader=lambda tier: frozenset())

    def test_unknown_channel_is_refused(self):
        ctx = self.ctx()
        with self.assertRaises(ValueError):
            ctx.before("laser", 1)

    def test_undeclared_channel_is_refused(self):
        ctx = self.ctx(leg=atspi_leg())
        with self.assertRaises(ValueError):
            ctx.before("x11", 1)

    def test_recording_the_same_stage_twice_is_refused(self):
        ctx = self.ctx()
        ctx.before("atspi", 1)
        with self.assertRaises(ValueError):
            ctx.before("atspi", 2)

    def test_findings_are_collected(self):
        ctx = self.ctx()
        ctx.finding("the app logged one warning")
        self.assertEqual(ctx.findings, ["the app logged one warning"])


class ProductForwardingTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.tier = fake_tier(Path(self.root.name))

    def test_headed_mode_forwards_headed(self):
        ctx = LegContext(
            self.tier, "headed", member_reader=lambda tier: frozenset()
        )
        captured = {}

        def fake_run_product(tier, argv, **kw):
            captured.update(kw)
            return Envelope(True, {}, None, "{}", 1.0, list(argv))

        with mock.patch.object(verdict, "run_product", fake_run_product):
            ctx.product(["snapshot"])
        self.assertTrue(captured.get("headed"))

    def test_headless_mode_forwards_no_headed(self):
        ctx = LegContext(
            self.tier, "headless", member_reader=lambda tier: frozenset()
        )
        captured = {}

        def fake_run_product(tier, argv, **kw):
            captured.update(kw)
            return Envelope(True, {}, None, "{}", 1.0, list(argv))

        with mock.patch.object(verdict, "run_product", fake_run_product):
            ctx.product(["snapshot"])
        self.assertFalse(captured.get("headed"))

    def test_product_calls_are_recorded(self):
        ctx = LegContext(
            self.tier, "headless", member_reader=lambda tier: frozenset()
        )

        def fake_run_product(tier, argv, **kw):
            return Envelope(True, {}, None, "{}", 1.0, list(argv))

        with mock.patch.object(verdict, "run_product", fake_run_product):
            ctx.product(["snapshot"])
            ctx.product(["click", "@e1"])
        self.assertEqual(len(ctx.products), 2)
        self.assertEqual(ctx.products[1].argv[-1], "@e1")


if __name__ == "__main__":
    unittest.main()
