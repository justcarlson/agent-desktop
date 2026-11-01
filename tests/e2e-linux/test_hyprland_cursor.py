from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_cursor as cursor

LEG_IDS = ("cursor.session_renderer_lifetime", "cursor.no_focus_or_input_interference",
           "cursor.window_capture_excludes_overlay", "cursor.disable_end_crash_and_compositor_loss")


def envelope(ok: bool, data: dict | None) -> Envelope:
    return Envelope(ok=ok, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0 if ok else 1)


class CursorLegTests(unittest.TestCase):
    def test_every_leg_is_registered_on_hyprland_with_an_implementation(self):
        for leg_id in LEG_IDS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
            self.assertIn("hyprland", leg.oracles)
            self.assertIn(leg_id, legs.implementations())

    def test_only_cursor_namespace_layers_are_counted(self):
        answer = {"DP-1": {"levels": {
            "0": [{"namespace": "wallpaper", "pid": 9}],
            "3": [{"namespace": "agent-desktop-cursor", "pid": 41}, {"namespace": "notifications", "pid": 7}],
        }}}
        self.assertEqual(cursor.cursor_layers(answer), [{"monitor": "DP-1", "level": "3", "pid": 41}])
        self.assertEqual(cursor.cursor_layers({}), [])

    def test_records_are_read_per_session(self):
        with tempfile.TemporaryDirectory() as work:
            runtime = Path(work)
            base = runtime / "agent-desktop" / "cursor" / "sig"
            base.mkdir(parents=True)
            (base / "a.json").write_text(json.dumps({"session_id": "one", "pid": 5}))
            (base / "b.json").write_text(json.dumps({"session_id": "two", "pid": 6}))
            (base / "start.lock").write_text("")
            self.assertEqual(cursor.record_of(runtime, "two")["pid"], 6)
            self.assertIsNone(cursor.record_of(runtime, "three"))
            (base / "c.json").write_text(json.dumps({"session_id": "two", "pid": 8}))
            with self.assertRaises(AssertionError):
                cursor.record_of(runtime, "two")

    def test_a_recorded_start_time_must_match_to_count_as_alive(self):
        own = {"pid": os.getpid(), "start_time": cursor.start_time(os.getpid())}
        self.assertTrue(cursor.alive(own))
        self.assertFalse(cursor.alive({**own, "start_time": own["start_time"] + 1}))

    def test_the_outcome_is_read_from_details(self):
        stopped = envelope(True, {"details": {"cursor_renderer": "stopped"}})
        self.assertEqual(cursor.outcome_problems("end", stopped, "stopped"), [])
        self.assertEqual(len(cursor.outcome_problems("end", stopped, "not_running")), 1)
        self.assertEqual(len(cursor.outcome_problems("end", envelope(True, {}), "stopped")), 1)

    def test_the_arrow_box_scales_and_stays_inside_the_image(self):
        self.assertEqual(cursor.arrow_box((0, 0), (100, 50), 2.0, (1000, 1000)), (199, 99, 28, 28))
        self.assertEqual(cursor.arrow_box((0, 0), (995, 0), 1.0, (1000, 1000)), (994, 0, 6, 14))

    def test_the_monitor_holding_a_point_is_found_by_logical_size(self):
        monitors = [{"name": "A", "x": 0, "y": 0, "width": 2560, "height": 1440, "scale": 2.0},
                    {"name": "B", "x": 1280, "y": 0, "width": 1920, "height": 1080, "scale": 1.0}]
        self.assertEqual(cursor.monitor_at(monitors, (1300, 10))["name"], "B")
        with self.assertRaises(AssertionError):
            cursor.monitor_at(monitors, (5000, 10))


if __name__ == "__main__":
    unittest.main()
