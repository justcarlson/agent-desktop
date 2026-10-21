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


class LegRegistryTests(unittest.TestCase):
    def test_dispositions_carry_an_honest_reason(self):
        for leg in legs.REGISTRY:
            if leg.disposition == "outstanding":
                self.assertIn(leg.id, legs.OUTSTANDING_REASONS)
                self.assertTrue(leg.reason)
            elif leg.disposition != "pass":
                self.assertTrue(leg.reason, leg.id)


if __name__ == "__main__":
    unittest.main()
