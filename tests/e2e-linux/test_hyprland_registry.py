from __future__ import annotations

import ast
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import gate, legs, main

NEW_LEGS = {
    "non_interference.headed_changes_only_target": (("hyprland",), ("headed",), "pass"),
    "non_interference.headless_route_preserves_desktop": (("hyprland",), ("headless",), "pass"),
    "session.cancel_cleanup_recorded_generations": (("hyprland",), ("headless", "headed"), "pass"),
    "error_contract.hyprland_delivery_boundaries": (("hyprland",), ("headless", "headed"), "pass"),
    "capture.hyprland_bound_toplevel_pixels": (("hyprland",), ("headless",), "pass"),
    "capture.hyprland_hidden_counter_frame_after_request": (("hyprland",), ("headless",), "pass"),
    "capture.hyprland_static_frame_after_request": (("hyprland",), ("headless",), "pass"),
    "capture.hyprland_in_view_first_frame": (("hyprland",), ("headless",), "pass"),
    "capture.hyprland_screen_exact_output": (("hyprland",), ("headless",), "pass"),
    "capture.hyprland_permission_denied_no_dialog": (("hyprland",), ("headless",), "asserted-refusal"),
    "capture.hyprland_deadline_or_cancel_released": (("hyprland",), ("headless",), "asserted-refusal"),
    "lifecycle.hyprland_window_operation_refusals": (("hyprland",), ("headless", "headed"), "asserted-refusal"),
    "pointer.hover_by_ref_refused": (("hyprland",), ("headed",), "asserted-refusal"),
    "pointer.hover_coordinates": (("hyprland",), ("headed",), "pass"),
    "pointer.coordinate_multi_click_xwayland": (("hyprland",), ("headed",), "pass"),
    "pointer.hover_by_ref_headless_refused": (("hyprland",), ("headless",), "asserted-refusal"),
    "pointer.ref_drag_refused": (("hyprland",), ("headed",), "asserted-refusal"),
    "pointer.ref_drag_headless_refused": (("hyprland",), ("headless",), "asserted-refusal"),
}
REUSED = (
    "non_interference.active_window_unchanged_by_semantic_click",
    "error_contract.recoverable_error_envelopes",
    "observation.stale_ref_after_ui_change",
    "batch_contract.entry_failure_and_stop_on_error",
)


def scenario_leg_ids() -> dict[str, str]:
    """Each leg id a scenario module names, as a literal `implementation(...)` argument or a key of a
    module-level `LEGS` map, with the module that names it."""
    named: dict[str, str] = {}
    for path in sorted((ROOT / "scenarios").glob("*.py")):
        tree = ast.parse(path.read_text(encoding="utf-8"), str(path))
        for node in ast.walk(tree):
            if (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "implementation"
                    and node.args and isinstance(node.args[0], ast.Constant) and isinstance(node.args[0].value, str)):
                named[node.args[0].value] = path.name
        for node in tree.body:
            if (isinstance(node, ast.Assign) and isinstance(node.value, ast.Dict)
                    and any(isinstance(target, ast.Name) and target.id == "LEGS" for target in node.targets)):
                for key in node.value.keys:
                    if isinstance(key, ast.Constant) and isinstance(key.value, str):
                        named[key.value] = path.name
    return named


class HyprlandRegistryTests(unittest.TestCase):
    def test_hyprland_is_a_tier_and_an_oracle(self):
        self.assertIn("hyprland", legs.TIER_NAMES)
        self.assertIn("hyprland", legs.ORACLE_NAMES)

    def test_legs_that_name_every_tier_keep_the_upstream_three(self):
        self.assertEqual(legs.ALL_TIERS, ("x11-openbox", "x11-mutter", "wayland"))
        zoo = legs.leg_by_id("ref_contract.zoo_gtk3")
        self.assertEqual(zoo.tiers, legs.ALL_TIERS + legs.HYPRLAND_ONLY)

    def test_every_new_leg_has_its_tiers_modes_and_disposition(self):
        for leg_id, (tiers, modes, disposition) in NEW_LEGS.items():
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (tiers, modes, disposition))
                if disposition == "asserted-refusal":
                    self.assertTrue(leg.reason)

    def test_every_new_hyprland_leg_reads_the_compositor(self):
        for leg_id, (tiers, _, _) in NEW_LEGS.items():
            if "hyprland" in tiers:
                with self.subTest(leg=leg_id):
                    self.assertIn("hyprland", legs.leg_by_id(leg_id).oracles)

    def test_reused_legs_add_hyprland_and_keep_their_upstream_tiers(self):
        for leg_id in REUSED:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertIn("hyprland", leg.tiers)
                self.assertTrue({"x11-openbox", "x11-mutter"} <= set(leg.tiers))

    def test_the_non_interference_port_reads_the_compositor(self):
        leg = legs.leg_by_id("non_interference.active_window_unchanged_by_semantic_click")
        self.assertIn("hyprland", leg.oracles)

    def test_every_hyprland_leg_has_an_implementation(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        expected = {leg.id for leg in legs.REGISTRY if "hyprland" in leg.tiers} | set(NEW_LEGS)
        for leg_id in sorted(expected):
            with self.subTest(leg=leg_id):
                self.assertIn(leg_id, implemented)

    def test_every_leg_a_scenario_names_is_registered(self):
        named = scenario_leg_ids()
        self.assertIn("pointer.coordinates_have_no_receiver_proof", named)
        unregistered = sorted(f"{module}: {leg_id}" for leg_id, module in named.items()
                              if leg_id not in {leg.id for leg in legs.REGISTRY})
        self.assertEqual(unregistered, [])

    def test_every_lazy_implementation_names_a_scenario_leg(self):
        import importlib

        for leg_id, (module, name) in legs.LAZY_IMPLEMENTATIONS.items():
            with self.subTest(leg=leg_id):
                self.assertIn(leg_id, {leg.id for leg in legs.REGISTRY})
                self.assertTrue(callable(getattr(importlib.import_module(module), name, None)))

    def test_each_tier_keeps_the_run_order_it_had_before_the_hyprland_rows_moved(self):
        def run_order(tier):
            return [leg.id for leg in legs.REGISTRY if tier in leg.tiers]

        hyprland = run_order("hyprland")
        zoo = hyprland.index("ref_contract.zoo_gtk3")
        self.assertLess(hyprland.index("non_interference.headed_changes_only_target"), zoo)
        self.assertLess(hyprland.index("capture.hyprland_deadline_or_cancel_released"), zoo)
        self.assertGreater(hyprland.index("focus.hyprland_other_workspace"), zoo)
        wayland = run_order("wayland")
        self.assertLess(wayland.index("session.other_wayland_route_unchanged"),
                        wayland.index("ref_contract.zoo_gtk3"))

    def test_hyprland_is_selectable_but_not_a_default_tier(self):
        self.assertIn("hyprland", main.TIER_CHOICES)
        self.assertNotIn("hyprland", main.DEFAULT_TIERS)
        self.assertEqual(main.parse_args(["--tier", "hyprland"]).tier, "hyprland")

    def test_the_attached_tier_needs_session_tools_not_a_private_display(self):
        missing = gate.check_prerequisites(("hyprland",), which=lambda tool: None, env={},
                                           gi_probe=lambda: None, platform="linux")
        named = {line.rsplit(": ", 1)[-1] for line in missing}
        for tool in ("Hyprland", "hyprctl", "systemd-run", "systemctl", "foot", "grim", "gdbus"):
            self.assertIn(tool, named)
        for tool in ("Xvfb", "xwd", "xprop"):
            self.assertNotIn(tool, named)

    def test_private_tiers_still_need_the_private_session_tools(self):
        missing = gate.check_prerequisites(("x11-openbox",), which=lambda tool: None, env={},
                                           gi_probe=lambda: None, platform="linux")
        self.assertIn("missing tool: Xvfb", missing)


if __name__ == "__main__":
    unittest.main()
