from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs

TEST_ALL = ROOT.parents[1] / "test" / "all"

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


def drift_list(name: str) -> set[str]:
    """The leg ids of one `<name>=( ... )` array in test/all."""
    found = re.search(rf"^{name}=\((.*?)^\)", TEST_ALL.read_text(encoding="utf-8"), re.M | re.S)
    if found is None:
        raise AssertionError(f"test/all has no {name} list")
    return {line.strip() for line in found.group(1).splitlines() if line.strip()}


class HyprlandReuseTests(unittest.TestCase):
    def test_each_reused_leg_is_registered_on_hyprland(self):
        """test/all leaves a drift leg out of the run, but only a registered one."""
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

    def test_the_hyprland_drift_list_names_only_hyprland_legs(self):
        for leg_id in sorted(drift_list("hyprland_drift")):
            with self.subTest(leg=leg_id):
                self.assertIn("hyprland", legs.leg_by_id(leg_id).tiers)

    def test_each_reused_leg_has_an_implementation(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in REUSED_ON_HYPRLAND:
            with self.subTest(leg=leg_id):
                self.assertIn(leg_id, implemented)


if __name__ == "__main__":
    unittest.main()
