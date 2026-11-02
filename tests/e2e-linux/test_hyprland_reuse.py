from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs

REUSED_ON_HYPRLAND = (
    "duplicates.two_processes_distinct_window_ids",
    "identity.restart_old_id_window_not_found",
    "identity.restart_new_id_resolves",
    "identity.id_survives_unrelated_launch",
    "ref_contract.zoo_gtk3",
    "ref_contract.zoo_gtk3_windows",
    "ref_contract.zoo_gtk4",
    "ref_contract.zoo_gtk4_windows",
    "ref_contract.zoo_qt",
    "ref_contract.zoo_qt_windows",
)


class HyprlandReuseTests(unittest.TestCase):
    def test_each_reused_leg_is_registered_on_hyprland(self):
        for leg_id in REUSED_ON_HYPRLAND:
            with self.subTest(leg=leg_id):
                self.assertIn("hyprland", legs.leg_by_id(leg_id).tiers)

    def test_each_reused_leg_keeps_its_upstream_tiers(self):
        for leg_id in REUSED_ON_HYPRLAND:
            with self.subTest(leg=leg_id):
                tiers = set(legs.leg_by_id(leg_id).tiers)
                self.assertTrue({"x11-openbox", "x11-mutter"} <= tiers)
                if leg_id.startswith("ref_contract."):
                    self.assertIn("wayland", tiers)

    def test_each_reused_leg_has_an_implementation(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in REUSED_ON_HYPRLAND:
            with self.subTest(leg=leg_id):
                self.assertIn(leg_id, implemented)


if __name__ == "__main__":
    unittest.main()
