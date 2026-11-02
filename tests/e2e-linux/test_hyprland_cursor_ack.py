from __future__ import annotations

import os
import sys
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from scenarios import hyprland_cursor_ack as ack

LEG_IDS = ("cursor.arrival_ack_before_dispatch_delayed",
           "cursor.arrival_ack_before_dispatch_withheld")
MS = 1_000_000


def delayed_line(received_ms: int, acked_ms: int, delay_ms: int = 600) -> dict:
    return {"hold": "delay", "delay_ms": delay_ms, "received_ns": received_ms * MS, "acked_ns": acked_ms * MS}


def withheld_line(received_ms: int) -> dict:
    return {"hold": "withhold", "received_ns": received_ms * MS, "acked_ns": None}


def reads(*pairs: tuple[int, str]) -> list[tuple[int, str]]:
    return [(started_ms * MS, value) for started_ms, value in pairs]


class CursorAckLegTests(unittest.TestCase):
    def test_both_legs_are_registered_on_hyprland_headless_with_an_implementation(self):
        for leg_id in LEG_IDS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
            self.assertEqual(leg.modes, legs.HEADLESS)
            self.assertEqual(leg.disposition, "pass")
            self.assertIn("atspi", leg.oracles)
            self.assertIn(leg_id, legs.implementations())

    def test_a_delayed_ack_passes_when_a_read_after_it_still_shows_the_old_display(self):
        timeline = reads((100, "7"), (705, "7"), (710, "73"), (900, "73"))
        self.assertEqual(ack.delayed_problems([delayed_line(100, 700)], timeline, "7"), [])

    def test_a_delayed_ack_fails_when_the_display_changed_before_the_ack(self):
        timeline = reads((100, "7"), (650, "7"), (660, "73"))
        problems = ack.delayed_problems([delayed_line(100, 700)], timeline, "7")
        self.assertTrue(any("may have reached the app before" in problem for problem in problems), problems)

    def test_a_delayed_ack_fails_when_it_came_early_or_past_the_arrival_timeout(self):
        timeline = reads((100, "7"), (1100, "7"), (1110, "73"))
        self.assertTrue(ack.delayed_problems([delayed_line(100, 500)], timeline, "7"))
        self.assertTrue(ack.delayed_problems([delayed_line(100, 1050)], timeline, "7"))

    def test_a_delayed_ack_fails_without_a_trace_or_with_a_withheld_line(self):
        timeline = reads((100, "7"), (800, "7"), (810, "73"))
        self.assertTrue(ack.delayed_problems([], timeline, "7"))
        self.assertTrue(ack.delayed_problems([delayed_line(100, 700), withheld_line(100)], timeline, "7"))

    def test_a_click_that_lands_twice_or_never_fails(self):
        twice = reads((100, "7"), (800, "7"), (810, "73"), (820, "733"))
        never = reads((100, "7"), (800, "7"))
        self.assertTrue(ack.delayed_problems([delayed_line(100, 700)], twice, "7"))
        self.assertTrue(ack.delayed_problems([delayed_line(100, 700)], never, "7"))

    def test_a_withheld_ack_passes_when_the_display_held_past_the_arrival_timeout(self):
        timeline = reads((100, "7"), (960, "7"), (1810, "7"), (1820, "73"))
        lines = [withheld_line(100), withheld_line(1000)]
        self.assertEqual(ack.withheld_problems(lines, timeline, "7"), [])

    def test_a_withheld_ack_fails_when_the_click_landed_inside_the_arrival_timeout(self):
        timeline = reads((100, "7"), (500, "7"), (510, "73"))
        self.assertTrue(ack.withheld_problems([withheld_line(100)], timeline, "7"))

    def test_a_withheld_ack_fails_when_any_ack_was_sent(self):
        timeline = reads((100, "7"), (1810, "7"), (1820, "73"))
        sent = {**withheld_line(100), "acked_ns": 700 * MS}
        self.assertTrue(ack.withheld_problems([sent], timeline, "7"))
        self.assertTrue(ack.withheld_problems([delayed_line(100, 700)], timeline, "7"))

    def test_transitions_list_each_new_value_once(self):
        timeline = reads((1, "7"), (2, "73"), (3, "73"), (4, "731"))
        self.assertEqual(ack.transitions(timeline, "7"), ["73", "731"])
        self.assertEqual(ack.last_unchanged_start(timeline, "7"), 1 * MS)
        self.assertIsNone(ack.last_unchanged_start(reads((1, "73")), "7"))

    def test_the_flag_is_written_owner_only_and_removed_with_its_trace(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = Path(directory)
            (registry / ack.ACK_TRACE).write_text("stale\n")
            with ack.held_ack(registry, "withhold") as trace:
                flag = registry / ack.ACK_FLAG
                self.assertEqual(flag.read_text(), "withhold")
                self.assertEqual(flag.stat().st_mode & 0o777, 0o600)
                self.assertFalse(trace.exists())
                trace.write_text("{}\n")
            self.assertFalse(flag.exists())
            self.assertFalse(trace.exists())

    def test_the_flag_refuses_to_follow_a_planted_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = Path(directory)
            target = registry / "elsewhere"
            os.symlink(target, registry / ack.ACK_FLAG)
            with self.assertRaises(OSError):
                with ack.held_ack(registry, "600"):
                    pass
            self.assertFalse(target.exists())

    def test_the_trace_reader_returns_nothing_for_a_missing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(ack.read_trace(Path(directory) / "absent"), [])

    def test_the_timeline_stops_once_the_display_settles_after_a_change(self):
        values = iter(["7", "7", "73"])
        timeline = ack.DisplayTimeline(lambda: next(values, "73"), "7", quiet_s=0.05)
        started = time.monotonic()
        with timeline:
            pass
        self.assertLess(time.monotonic() - started, 2.0)
        self.assertEqual(ack.transitions(timeline.reads, "7"), ["73"])
        self.assertEqual([value for _, value in timeline.reads[:2]], ["7", "7"])


if __name__ == "__main__":
    unittest.main()
