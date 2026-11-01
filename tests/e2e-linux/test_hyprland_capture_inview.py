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


def logged(ok: bool, sessions: int, *, data=None) -> object:
    shot = envelope(ok, data=data, elapsed_ms=460.0)
    lines = ["[ 1.000] -> wl_display@1.get_registry(wl_registry@2)",
             "[ 1.001] -> ext_foreign_toplevel_list_v1@7.stop()"]
    lines += [f"[ 1.0{index}2] -> ext_image_copy_capture_manager_v1@9.create_session(new id "
              f"ext_image_copy_capture_session_v1@{20 + index}, ext_image_capture_source_v1@12, 0)"
              for index in range(sessions)]
    lines.append("[ 1.090] ext_image_copy_capture_session_v1@20.done()")
    shot.stderr = "\n".join(lines) + "\n"
    return shot


class CaptureSessionTests(unittest.TestCase):
    def test_each_session_request_counts_once(self):
        for sessions in (0, 1, 11):
            with self.subTest(sessions=sessions):
                self.assertEqual(hyprland_capture_inview.capture_sessions(logged(True, sessions)), sessions)

    def test_events_and_other_requests_do_not_count(self):
        shot = envelope(True)
        shot.stderr = ("[ 1.0] ext_image_copy_capture_manager_v1@9.create_session(...)\n"
                       "[ 1.1] -> ext_image_copy_capture_session_v1@20.create_frame(new id x@21)\n")
        self.assertEqual(hyprland_capture_inview.capture_sessions(shot), 0)


class StillInViewTests(unittest.TestCase):
    PAINT = (0x50, 0x70, 0xa0)
    FOUND = {"size": [120, 60], "monitor": 0}

    def still(self, shot_sessions: int, batch_sessions: int, entry: dict | None = None):
        image = solid(120, 60, self.PAINT)
        with tempfile.TemporaryDirectory() as folder, \
                mock.patch.object(hyprland_capture_inview, "scale_of", return_value=1.0), \
                mock.patch.object(hyprland_capture_inview, "_settled_capture",
                                  return_value=(envelope(True), image)), \
                mock.patch.object(hyprland_capture_inview, "saved_image", return_value=image):
            ctx = mock.Mock()
            ctx.tier.runtime_dir = Path(folder)
            ctx.tier.env = {"WAYLAND_DISPLAY": "wayland-1"}
            ctx.product.side_effect = [logged(True, shot_sessions),
                                       logged(True, batch_sessions, data={"results": [entry or {"ok": True}]})]
            problems = hyprland_capture_inview._still_in_view(ctx, "w", self.FOUND)
            return problems, ctx

    def test_one_session_each_passes_however_long_it_took(self):
        problems, ctx = self.still(1, 1)
        self.assertEqual(problems, [])
        self.assertIn("record only: screenshot 460 ms, batch 460 ms", ctx.finding.call_args.args[0])

    def test_both_runs_ask_for_the_protocol_log(self):
        _, ctx = self.still(1, 1)
        for call in ctx.product.call_args_list:
            self.assertEqual(call.kwargs["env"], {"WAYLAND_DISPLAY": "wayland-1", "WAYLAND_DEBUG": "1"})
        self.assertNotIn("--timeout-ms", ctx.product.call_args_list[1].args[0])

    def test_a_screenshot_that_waited_for_a_redraw_fails(self):
        problems, _ = self.still(11, 1)
        self.assertEqual(len(problems), 1)
        self.assertIn("in view still: an unchanged window in view used 11 capture sessions", problems[0])

    def test_a_batch_that_waited_for_a_redraw_fails(self):
        problems, _ = self.still(1, 2)
        self.assertEqual(len(problems), 1)
        self.assertIn("in view still batch: an unchanged window in view used 2 capture sessions", problems[0])

    def test_a_missing_protocol_log_fails(self):
        problems, _ = self.still(0, 1)
        self.assertIn("used 0 capture sessions, not 1, so it left no protocol log", problems[0])

    def test_a_batch_that_does_not_capture_fails(self):
        timed_out = {"ok": False, "error": {"code": "TIMEOUT", "details": {"kind": "capture_frame"},
                                            "disposition": NOT_DELIVERED}}
        problems, _ = self.still(1, 1, timed_out)
        self.assertIn("the batch did not capture", problems[0])


if __name__ == "__main__":
    unittest.main()
