from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle.pixels import Image
from runner.product import Envelope
from scenarios import hyprland_capture, hyprland_capture_refusals, hyprland_capture_screen
from scenarios.hyprland_capture_common import dominant, fresh, settled_tick, tick_of, written

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
MS = 1_000_000


def painter():
    spec = importlib.util.spec_from_file_location("terminal_painter", ROOT / "apps" / "terminal_painter.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def solid(width: int, height: int, rgb: tuple[int, int, int]) -> Image:
    return Image(width=width, height=height, rgb=bytes(rgb) * (width * height))


def envelope(ok: bool, *, data=None, error=None, elapsed_ms=5.0, exit_code=None) -> Envelope:
    return Envelope(ok=ok, data=data, error=error, raw="{}", elapsed_ms=elapsed_ms, argv=[],
                    exit_code=exit_code if exit_code is not None else (0 if ok else 1))


def batch(entry: dict, elapsed_ms: float = 40.0) -> Envelope:
    return envelope(True, data={"results": [entry]}, elapsed_ms=elapsed_ms)


class TickTests(unittest.TestCase):
    def test_a_ticker_colour_names_its_tick(self):
        colour_of = painter().tick_colour
        for tick in (1, 255, 256, 777, 65535):
            with self.subTest(tick=tick):
                text = colour_of(tick)
                rgb = (int(text[0:2], 16), int(text[2:4], 16), int(text[4:6], 16))
                self.assertEqual(tick_of(rgb), tick)

    def test_a_colour_no_tick_paints_names_no_tick(self):
        self.assertIsNone(tick_of((0x20, 0x10, 0x41)))

    def test_the_settled_tick_is_the_last_written_a_margin_before_the_mark(self):
        ticks = [(1, 0), (2, 50 * MS), (3, 140 * MS), (4, 190 * MS)]
        self.assertEqual(settled_tick(ticks, 200 * MS), 2)
        self.assertEqual(settled_tick(ticks, 0), 0)

    def test_a_capture_is_fresh_only_from_the_settled_tick_on(self):
        ticks = [(1, 0), (2, 50 * MS), (3, 140 * MS)]
        self.assertTrue(fresh(2, ticks, 200 * MS))
        self.assertTrue(fresh(3, ticks, 200 * MS))
        self.assertFalse(fresh(1, ticks, 200 * MS))
        self.assertFalse(fresh(None, ticks, 200 * MS))

    def test_the_oracle_check_fails_when_it_accepts_the_first_shot_for_the_last_request(self):
        ticks = [(tick, tick * 50 * MS) for tick in range(1, 40)]
        stale = [(5, 6 * 50 * MS), (30, 31 * 50 * MS)]
        self.assertEqual(hyprland_capture._oracle_discriminates(stale, ticks), [])
        blind = [(30, 6 * 50 * MS), (30, 31 * 50 * MS)]
        self.assertTrue(hyprland_capture._oracle_discriminates(blind, ticks))

    def test_a_half_written_log_line_is_skipped(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "painter.log"
            log.write_text("1 100\n2 200\n\x00\x00 \x00\n3")
            self.assertEqual(written(log), [(1, 100), (2, 200)])
            self.assertEqual(written(Path(folder) / "absent.log"), [])


class PaintedTests(unittest.TestCase):
    def test_the_dominant_colour_and_its_share(self):
        image = solid(20, 20, (0xC0, 0x30, 0x20))
        self.assertEqual(dominant(image), ((0xC0, 0x30, 0x20), 1.0))

    def test_a_painted_capture_at_its_scaled_size_passes(self):
        image = solid(150, 75, (0xC0, 0x30, 0x20))
        self.assertEqual(hyprland_capture.painted_problems("w", image, "c03020", (120, 60), 1.25), [])

    def test_another_colour_or_size_fails(self):
        image = solid(150, 75, (0x20, 0xC0, 0x30))
        self.assertTrue(hyprland_capture.painted_problems("w", image, "c03020", (120, 60), 1.25))
        for width, height in ((120, 75), (150, 60)):
            image = solid(width, height, (0xC0, 0x30, 0x20))
            self.assertTrue(hyprland_capture.painted_problems("w", image, "c03020", (120, 60), 1.25))


class Findings:
    def __init__(self):
        self.lines = []

    def finding(self, text: str) -> None:
        self.lines.append(text)


class SettledCaptureTests(unittest.TestCase):
    def run_captures(self, shots):
        answers = iter(shots)
        calls = []

        def take(ctx, window_id, name):
            calls.append(name)
            return next(answers)

        ctx = Findings()
        with mock.patch.object(hyprland_capture, "_capture", take), \
                mock.patch.object(hyprland_capture, "RESIZE_SETTLE_S", 0.5):
            result = hyprland_capture._settled_capture(ctx, "w1", "w", (120, 60), 1.25)
        return result, calls, ctx.lines

    def test_a_capture_mid_animation_is_taken_again_until_it_has_the_window_size(self):
        small = (envelope(True), solid(100, 50, (1, 2, 3)))
        full = (envelope(True), solid(150, 75, (1, 2, 3)))
        result, calls, lines = self.run_captures([small, small, full, small])
        self.assertIs(result, full)
        self.assertEqual(len(calls), 3)
        self.assertEqual(len(set(calls)), 3)
        self.assertEqual(len(lines), 1)

    def test_a_settled_first_capture_is_taken_once_and_records_nothing(self):
        full = (envelope(True), solid(150, 75, (1, 2, 3)))
        result, calls, lines = self.run_captures([full, full])
        self.assertIs(result, full)
        self.assertEqual((len(calls), lines), (1, []))

    def test_a_failed_capture_ends_the_wait_at_once(self):
        failed = (envelope(False, error={"code": "TIMEOUT"}), None)
        result, calls, _ = self.run_captures([failed, failed])
        self.assertIs(result, failed)
        self.assertEqual(len(calls), 1)

    def test_a_size_that_never_settles_returns_the_last_capture_for_the_size_check(self):
        small = (envelope(True), solid(100, 50, (1, 2, 3)))
        result, calls, _ = self.run_captures([small] * 1000)
        self.assertIs(result, small)
        self.assertGreater(len(calls), 1)
        self.assertTrue(hyprland_capture.painted_problems("w", result[1], "010203", (120, 60), 1.25))


class DisplayTests(unittest.TestCase):
    MONITOR = {"name": "AD-E2E-1", "x": -1024, "y": -640, "width": 1280, "height": 800,
               "scale": 1.25, "transform": 0}

    def test_the_expected_display_is_the_logical_size_at_the_origin(self):
        want = hyprland_capture_screen.expected_display(self.MONITOR)
        self.assertEqual(want["bounds"], {"x": -1024.0, "y": -640.0, "width": 1024.0, "height": 640.0})

    def test_a_rotated_output_swaps_its_logical_size(self):
        rotated = {**self.MONITOR, "transform": 1}
        want = hyprland_capture_screen.expected_display(rotated)
        self.assertEqual((want["bounds"]["width"], want["bounds"]["height"]), (640.0, 1024.0))
        self.assertEqual(hyprland_capture_screen.pixel_size(rotated), (800, 1280))

    def test_matching_displays_pass_and_drifted_ones_fail(self):
        right = {"id": "AD-E2E-1", "scale": 1.25,
                 "bounds": {"x": -1024, "y": -640, "width": 1024, "height": 640}}
        self.assertEqual(hyprland_capture_screen.display_problems([right], [self.MONITOR]), [])
        moved = {**right, "bounds": {**right["bounds"], "x": 0}}
        self.assertTrue(hyprland_capture_screen.display_problems([moved], [self.MONITOR]))
        scaled = {**right, "scale": 1.0}
        self.assertTrue(hyprland_capture_screen.display_problems([scaled], [self.MONITOR]))
        self.assertTrue(hyprland_capture_screen.display_problems([], [self.MONITOR]))

    def test_the_output_rule_has_both_config_forms(self):
        lua = hyprland_capture_screen.output_rule(True)
        self.assertEqual(lua[0], "eval")
        self.assertIn('position = "-1024x-640"', lua[1])
        self.assertEqual(hyprland_capture_screen.output_rule(False),
                         ["keyword", "monitor", "AD-E2E-1,1280x800@60,-1024x-640,1.25"])


class RefusalTests(unittest.TestCase):
    DENIED = {"code": "PERM_DENIED", "disposition": NOT_DELIVERED,
              "details": {"option": "ecosystem:enforce_permissions", "enforced": True}}

    def test_a_permission_refusal_passes(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "denied.png"
            found = hyprland_capture_refusals.permission_problems("w", envelope(False, error=self.DENIED), path)
            self.assertEqual(found, [])

    def test_a_slow_refusal_an_image_or_a_missing_option_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "denied.png"
            slow = envelope(False, error=self.DENIED, elapsed_ms=1500.0)
            self.assertTrue(hyprland_capture_refusals.permission_problems("w", slow, path))
            bare = envelope(False, error={**self.DENIED, "details": {}})
            self.assertTrue(hyprland_capture_refusals.permission_problems("w", bare, path))
            other = envelope(False, error={**self.DENIED, "details": {"option": "misc:vfr", "enforced": True}})
            self.assertTrue(hyprland_capture_refusals.permission_problems("w", other, path))
            path.write_bytes(b"png")
            self.assertTrue(hyprland_capture_refusals.permission_problems(
                "w", envelope(False, error=self.DENIED), path))

    def test_the_enforce_control_has_both_config_forms(self):
        self.assertEqual(hyprland_capture_refusals.enforce_argv(True, False),
                         ["keyword", "ecosystem:enforce_permissions", "true"])
        lua = hyprland_capture_refusals.enforce_argv(False, True)
        self.assertEqual(lua, ["eval", "hl.config({ ecosystem = { enforce_permissions = false } })"])

    def test_asker_processes_are_found_by_name(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for pid, name in ((10, "xdg-desktop-portal"), (11, "foot"), (12, "hyprland-dialog")):
                (root / str(pid)).mkdir()
                (root / str(pid) / "comm").write_text(name + "\n")
            self.assertEqual(hyprland_capture_refusals.askers(root),
                             {(10, "xdg-desktop-portal"), (12, "hyprland-dialog")})

    def test_a_batch_budget_captures_or_times_out_not_delivered(self):
        problems = hyprland_capture_refusals.batch_entry_problems
        self.assertEqual(problems(50, batch({"ok": True})), ([], "captured"))
        timed_out = {"ok": False, "error": {"code": "TIMEOUT", "disposition": NOT_DELIVERED,
                                            "details": {"kind": "capture_frame"}}}
        self.assertEqual(problems(50, batch(timed_out)), ([], "capture_frame"))

    def test_another_code_a_late_answer_or_a_broken_batch_fails(self):
        problems = hyprland_capture_refusals.batch_entry_problems
        failed = {"ok": False, "error": {"code": "ACTION_FAILED", "disposition": NOT_DELIVERED}}
        self.assertEqual(problems(50, batch(failed))[1], "broken")
        self.assertTrue(problems(50, batch({"ok": True}, elapsed_ms=2000.0))[0])
        self.assertEqual(problems(50, envelope(False, error={"code": "INVALID_ARGS"}))[1], "broken")


if __name__ == "__main__":
    unittest.main()
