from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import scenarios
from runner.legs import REGISTRY, implementations

LANE_PREFIXES = ("events.", "menus.", "identity.", "surfaces.", "launch.")
LANE_PREFIXES += ("shell_surfaces.", "notifications.", "tray.")
LANE_PREFIXES += ("wayland_actions.", "wayland_input.", "wayland_focus.")


class LaneCoverageTests(unittest.TestCase):
    def test_every_lane_leg_is_implemented(self) -> None:
        registered = {leg.id for leg in REGISTRY if leg.id.startswith(LANE_PREFIXES)}
        implemented = set(implementations())
        self.assertTrue(registered)
        self.assertEqual(registered - implemented, set())

    def test_no_foreign_leg_is_implemented_here(self) -> None:
        registered = {leg.id for leg in REGISTRY if leg.id.startswith(LANE_PREFIXES)}
        lane_sources = {
            "events",
            "menus",
            "identity",
            "surfaces",
            "launch",
            "shell_surfaces",
            "notifications",
            "tray",
            "wayland_actions",
            "wayland_input",
            "wayland_focus",
        }
        implementations_now = implementations()
        for leg_id in sorted(registered):
            module = implementations_now[leg_id].__module__.rsplit(".", 1)[-1]
            self.assertIn(module, lane_sources, leg_id)


if __name__ == "__main__":
    unittest.main()
