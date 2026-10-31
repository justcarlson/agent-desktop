from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from scenarios import hyprland_capture, hyprland_capture_inview
from scenarios.hyprland_capture_common import newest_drawn
from test_hyprland_capture_scenarios import MS, NOT_DELIVERED, batch, envelope, solid


class NewestDrawnTests(unittest.TestCase):
    TICKS = [(1, 0), (2, 1000 * MS), (3, 1210 * MS), (4, 1420 * MS)]

    def test_the_newest_tick_committed_before_the_request_or_a_newer_one_passes(self):
        self.assertTrue(newest_drawn(3, self.TICKS, 1300 * MS))
        self.assertTrue(newest_drawn(4, self.TICKS, 1300 * MS))

    def test_an_older_tick_than_the_newest_committed_fails(self):
        self.assertFalse(newest_drawn(2, self.TICKS, 1300 * MS))

    def test_a_tick_drawn_just_before_the_request_may_still_be_uncommitted(self):
        self.assertTrue(newest_drawn(2, self.TICKS, 1215 * MS))

    def test_the_last_tick_of_a_painter_that_stopped_drawing_fails(self):
        self.assertFalse(newest_drawn(4, self.TICKS, 3000 * MS))

    def test_an_unknown_tick_fails(self):
        self.assertFalse(newest_drawn(None, self.TICKS, 1300 * MS))
        self.assertFalse(newest_drawn(9, self.TICKS, 1300 * MS))

    def test_the_oracle_check_fails_when_it_accepts_the_first_shot_for_the_last_request(self):
        shots = [(3, 1300 * MS), (4, 1500 * MS)]
        self.assertEqual(hyprland_capture._oracle_discriminates(shots, self.TICKS, newest_drawn), [])
        blind = [(4, 1300 * MS), (4, 1500 * MS)]
        self.assertTrue(hyprland_capture._oracle_discriminates(blind, self.TICKS, newest_drawn))


class ViewRoundTests(unittest.TestCase):
    MARK = 1300 * MS

    def round(self, tick: int, ok: bool = True):
        image = solid(8, 8, (tick % 256, tick // 256, 0x40))
        shot = (envelope(True), image) if ok else (envelope(False), None)
        ticks = NewestDrawnTests.TICKS
        with mock.patch.object(hyprland_capture_inview, "written", side_effect=[[], ticks, ticks]), \
                mock.patch.object(hyprland_capture_inview, "monotonic_ns", return_value=self.MARK), \
                mock.patch.object(hyprland_capture_inview, "_capture", return_value=shot):
            return hyprland_capture_inview._view_round(mock.Mock(), "w", Path("log"), "view")

    def test_the_newest_committed_tick_passes(self):
        self.assertEqual(self.round(3), ([], (3, self.MARK)))

    def test_an_older_tick_fails_and_names_its_age(self):
        problems, shot = self.round(2)
        self.assertEqual(shot, (2, self.MARK))
        self.assertIn("first drawn 300 ms before the request; tick 3 was committed", problems[0])

    def test_a_failed_capture_fails_the_round(self):
        problems, shot = self.round(3, ok=False)
        self.assertIn("capture failed", problems[0])
        self.assertEqual(shot, (None, self.MARK))


class StillInViewTests(unittest.TestCase):
    PAINT = (0x50, 0x70, 0xa0)
    FOUND = {"size": [120, 60], "monitor": 0}
    TIMED_OUT = {"ok": False, "error": {"code": "TIMEOUT", "details": {"kind": "capture_frame"},
                                        "disposition": NOT_DELIVERED}}

    def still(self, elapsed_ms: float, entry: dict) -> list[str]:
        image = solid(120, 60, self.PAINT)
        with tempfile.TemporaryDirectory() as folder, \
                mock.patch.object(hyprland_capture_inview, "scale_of", return_value=1.0), \
                mock.patch.object(hyprland_capture_inview, "_settled_capture",
                                  return_value=(envelope(True), image)), \
                mock.patch.object(hyprland_capture_inview, "_capture",
                                  return_value=(envelope(True, elapsed_ms=elapsed_ms), image)), \
                mock.patch.object(hyprland_capture_inview, "saved_image", return_value=image):
            ctx = mock.Mock()
            ctx.tier.runtime_dir = Path(folder)
            ctx.product.side_effect = [batch(entry, 90.0)]
            return hyprland_capture_inview._still_in_view(ctx, "w", self.FOUND)

    def test_a_first_frame_answer_passes(self):
        self.assertEqual(self.still(80.0, {"ok": True}), [])

    def test_a_capture_that_waited_for_a_redraw_fails(self):
        problems = self.still(460.0, {"ok": True})
        self.assertIn("took 460 ms", problems[0])

    def test_a_short_batch_that_times_out_fails(self):
        problems = self.still(80.0, self.TIMED_OUT)
        self.assertIn("answered capture_frame, not a capture", problems[0])


if __name__ == "__main__":
    unittest.main()
