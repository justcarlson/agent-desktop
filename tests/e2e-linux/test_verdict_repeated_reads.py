from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import atspi_raw as raw, x11
from runner import legs, verdict
from runner.legs import Leg
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


class RepeatedReadTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.tier = fake_tier(Path(self.root.name))

    def ctx(self, leg=None) -> LegContext:
        return LegContext(self.tier, "headless", leg, member_reader=lambda tier: frozenset())

    def test_repeated_reads_accept_none_tuples_and_dicts(self):
        ctx = self.ctx()
        ctx.before("atspi", None)
        ctx.before("atspi", ("a", (1, 2)))
        ctx.before("atspi", {"window": (3, 4)})
        ctx.after("atspi", None)
        ctx.restored("atspi", None)
        self.assertEqual(verdict.evaluate(ctx), [])

    def test_a_mutation_still_needs_a_restoration_on_the_channel(self):
        ctx = self.ctx()
        ctx.before("atspi", None)
        ctx.before("atspi", {"window": (3, 4)})
        ctx.after("atspi", {"window": (5, 6)})
        failures = verdict.evaluate(ctx)
        self.assertTrue(any("no restoration read" in failure for failure in failures))

    def test_repeated_unchanged_reads_pass_without_a_restoration(self):
        ctx = self.ctx()
        ctx.before("atspi", (1, 2))
        ctx.before("atspi", {"a": None})
        ctx.after("atspi", (1, 2))
        ctx.after("atspi", {"a": None})
        self.assertEqual(verdict.evaluate(ctx), [])




if __name__ == "__main__":
    unittest.main()
