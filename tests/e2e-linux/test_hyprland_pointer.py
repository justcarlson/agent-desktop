from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_pointer as pointer
from scenarios import hyprland_pointer_pad as pad
from scenarios import hyprland_pointer_refs as refs

NEW_LEGS = (
    "pointer.global_layout_coordinates",
    "pointer.ref_receiver_evidence",
    "pointer.cover_or_missing_evidence_refused",
    "pointer.coordinates_have_no_receiver_proof",
    "pointer.headless_or_cancelled_input",
    "pointer.cancel_or_disconnect_releases_buttons",
)
REUSED_LEGS = (
    "pointer.mouse_click_delivery_report",
    "pointer.mouse_wheel_delivery_report",
    "pointer.drag_delivery_report",
    "ref_actions.scroll",
    "ref_actions.scroll_headless_refusal",
    "ref_actions.double_click",
    "ref_actions.double_click_headless_refusal",
    "ref_actions.triple_click",
    "ref_actions.triple_click_headless_refusal",
)
LOG = [
    {"ev": "motion", "widget": "area", "wx": 5, "wy": 5, "t": 1.0},
    {"ev": "press", "widget": "area", "n": 1, "button": 1, "wx": 10, "wy": 20, "t": 2.0},
    {"ev": "release", "widget": "area", "button": 1, "wx": 30, "wy": 40, "t": 3.0},
    {"ev": "motion", "widget": "area", "wx": 31, "wy": 41, "t": 4.0},
    {"ev": "press", "widget": "button", "n": 1, "button": 1, "wx": 1, "wy": 1, "t": 5.0},
    {"ev": "press", "widget": "button", "n": 2, "button": 1, "wx": 1, "wy": 1, "t": 5.1},
    {"ev": "scroll", "widget": "area", "direction": "smooth", "t": 6.0},
    {"ev": "scroll", "widget": "area", "direction": "down", "t": 6.1},
]


def refused(code: str, delivery: str = "not_delivered", **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details,
             "disposition": {"delivery": delivery, "retry": "safe"}}
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=1.0,
                    argv=["agent-desktop"], exit_code=1)


class PadLogTests(unittest.TestCase):
    def test_presses_keep_gtk_counts_per_widget(self):
        self.assertEqual(pad.presses(LOG, "button"), [1, 2])
        self.assertEqual(pad.presses(LOG, "area"), [1])

    def test_scrolls_leave_out_smooth_events(self):
        self.assertEqual(pad.scrolls(LOG, "area"), ["down"])

    def test_since_drops_the_events_before_the_mark(self):
        self.assertEqual(pad.since(LOG, 6), LOG[6:])

    def test_releases_and_late_motion(self):
        self.assertEqual(len(pad.releases(LOG, "area")), 1)
        self.assertEqual(pad.motions_after(LOG, 3.0), [LOG[3]])

    def test_events_read_the_log_lines(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "pad.events"
            log.write_text("".join(json.dumps(item) + "\n" for item in LOG[:2]) + "\n")
            self.assertEqual(pad.events(log), LOG[:2])


class GeometryTests(unittest.TestCase):
    client = {"at": [-1024, -640], "size": [640, 480]}

    def test_holds_takes_client_and_layer_boxes(self):
        self.assertTrue(pad.holds((-1024, -640), self.client))
        self.assertFalse(pad.holds((-384, -640), self.client))
        self.assertTrue(pad.holds((5, 5), {"x": 0, "y": 0, "w": 10, "h": 10}))
        self.assertFalse(pad.holds((10, 5), {"x": 0, "y": 0, "w": 10, "h": 10}))

    def test_covering_layers_counts_only_top_and_overlay(self):
        layers = {"DP-1": {"levels": {
            "0": [{"x": 0, "y": 0, "w": 100, "h": 100, "namespace": "wallpaper"}],
            "2": [{"x": 0, "y": 0, "w": 100, "h": 30, "namespace": "omarchy-bar"}],
            "3": [{"x": 50, "y": 50, "w": 50, "h": 50, "namespace": "omarchy-notifications"}],
        }}}
        self.assertEqual(pad.covering_layers(layers, (10, 10)), ["omarchy-bar"])
        self.assertEqual(pad.covering_layers(layers, (60, 60)), ["omarchy-notifications"])
        self.assertEqual(pad.covering_layers(layers, (10, 60)), [])

    def test_global_and_window_points_round_trip(self):
        point = pad.global_point(self.client, (0.5, 0.5))
        self.assertEqual(point, (-704, -400))
        self.assertEqual(pad.window_point(point, self.client), (320, 240))
        self.assertEqual(pad.xy((-704.6, -400.2)), "-704,-400")

    def test_lands_at_allows_one_pixel(self):
        self.assertTrue(pad.lands_at({"wx": 11, "wy": 19}, (10, 20)))
        self.assertFalse(pad.lands_at({"wx": 12, "wy": 20}, (10, 20)))


class EnvelopeCheckTests(unittest.TestCase):
    def test_no_receiver_proof_needs_false_and_delivered_events(self):
        self.assertEqual(pointer.no_receiver_proof("c", {"details": {"receiver_proof": False,
                                                                     "delivered_events": 3}}), [])
        self.assertEqual(len(pointer.no_receiver_proof("c", {"details": {"receiver_proof": True,
                                                                         "delivered_events": 3}})), 1)
        self.assertEqual(len(pointer.no_receiver_proof("c", {"receiver_proof": False})), 2)

    def test_refusal_problems_want_policy_denied_before_delivery(self):
        self.assertEqual(pointer.refusal_problems("c", refused("POLICY_DENIED")), [])
        self.assertTrue(pointer.refusal_problems("c", refused("ACTION_FAILED")))
        self.assertTrue(pointer.refusal_problems("c", refused("POLICY_DENIED", physical_delivery_started=True)))
        self.assertTrue(pointer.refusal_problems("c", refused("POLICY_DENIED", "delivery_uncertain")))

    def test_receiver_refusal_wants_the_named_cause(self):
        envelope = refused("ACTION_FAILED", kind="receiver_evidence", cause="layer_cover",
                           physical_delivery_started=False)
        self.assertEqual(refs.receiver_refusal("c", envelope, "layer_cover"), [])
        self.assertTrue(refs.receiver_refusal("c", envelope, "window_cover"))
        self.assertTrue(refs.receiver_refusal("c", refused("ACTION_FAILED", kind="receiver_evidence",
                                                           cause="layer_cover"), "layer_cover"))


class DispatchArgvTests(unittest.TestCase):
    def test_lua_and_legacy_window_dispatchers(self):
        self.assertEqual(refs.window_argv("move", "0x10", True, 4, 5),
                         ["dispatch", 'hl.dsp.window.move({x=4,y=5,window="address:0x10"})'])
        self.assertEqual(refs.window_argv("resize", "0x10", False, 640, 480),
                         ["dispatch", "resizewindowpixel", "exact 640 480,address:0x10"])
        self.assertEqual(refs.window_argv("pin", "0x10", True),
                         ["dispatch", 'hl.dsp.window.pin({action="enable",window="address:0x10"})'])
        self.assertEqual(refs.window_argv("pin", "0x10", False), ["dispatch", "pin", "address:0x10"])


class RegistryTests(unittest.TestCase):
    def test_new_pointer_legs_run_on_hyprland_only(self):
        for leg_id in NEW_LEGS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY, leg_id)
            self.assertIn("applog", leg.oracles, leg_id)
            self.assertIn(leg_id, legs._IMPLEMENTATIONS, leg_id)

    def test_reused_pointer_legs_reach_hyprland_with_the_pad_log(self):
        for leg_id in REUSED_LEGS:
            leg = legs.leg_by_id(leg_id)
            self.assertIn("hyprland", leg.tiers, leg_id)
            self.assertIn("applog", leg.oracles, leg_id)

    def test_ref_drag_stays_off_hyprland(self):
        for leg_id in ("ref_actions.drag", "ref_actions.drag_headless_refusal"):
            self.assertNotIn("hyprland", legs.leg_by_id(leg_id).tiers)


if __name__ == "__main__":
    unittest.main()
