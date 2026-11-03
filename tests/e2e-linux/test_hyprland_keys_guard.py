from __future__ import annotations

import json
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from scenarios import hyprland_keys_guard as guard

LEG = "input.hyprland_killed_type_guard_releases_keys"


def key(event: str, keysym: str, code: int, t: float) -> dict:
    return {"event": event, "keysym": keysym, "code": code, "state": 0, "group": 0, "t": t}


def measure(**fields) -> dict:
    found = {"held": ["c"], "held_open": [], "repeat_delay_ms": 600, "release_after_press_ms": 20,
             "release_after_kill_ms": 12, "extra_presses": 0, "extra_chars": 0, "bystander_events": 0}
    return dict(found, **fields)


class GuardRegistryTests(unittest.TestCase):
    def test_the_guard_leg_is_hyprland_only_headed_and_implemented(self):
        import scenarios  # noqa: F401

        leg = legs.leg_by_id(LEG)
        self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headed",), "pass"))
        self.assertTrue({"hyprland", "atspi", "applog", "proc"} <= set(leg.oracles))
        self.assertIn(LEG, legs.implementations())

    def test_every_round_kills_by_pid_or_by_group_with_a_letter_or_shift_held(self):
        self.assertEqual(set(guard.ROUNDS), {("pid", "letter"), ("pid", "shift"), ("group", "letter"),
                                             ("group", "shift")})
        self.assertFalse(any(char.isupper() for char in guard.LETTERS))
        self.assertTrue(any(char.isupper() for char in guard.SHIFTED))


class TailTests(unittest.TestCase):
    def test_a_record_still_being_written_is_read_once_it_is_complete(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "key.jsonl"
            first, second = json.dumps({"event": "press"}), json.dumps({"event": "release"})
            log.write_text(first + "\n" + second[:5])
            tail = guard.Tail(log)
            self.assertEqual(tail.records(), [{"event": "press"}])
            log.write_text(first + "\n" + second + "\n")
            self.assertEqual(tail.records(), [{"event": "press"}, {"event": "release"}])
            self.assertEqual(tail.records(), [{"event": "press"}, {"event": "release"}])

    def test_a_tail_at_the_end_reads_only_later_records(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "key.jsonl"
            log.write_text(json.dumps({"event": "press"}) + "\n")
            tail = guard.Tail.at_end(log)
            with log.open("a") as handle:
                handle.write(json.dumps({"event": "text"}) + "\n")
            self.assertEqual(tail.records(), [{"event": "text"}])


class HeldTests(unittest.TestCase):
    def test_held_presses_keep_the_press_of_each_key_not_released(self):
        found = [key("press", "Shift_L", 50, 1.0), key("press", "B", 56, 1.1), key("release", "B", 56, 1.2),
                 {"event": "text", "text": "B"}, key("press", "c", 54, 1.3)]
        self.assertEqual([item["code"] for item in guard.held_presses(found)], [50, 54])

    def test_a_letter_round_wants_a_letter_and_a_shift_round_wants_shift(self):
        shift, letter = key("press", "Shift_L", 50, 1.0), key("press", "c", 54, 1.0)
        self.assertEqual([guard.wanted("letter", shift), guard.wanted("letter", letter)], [False, True])
        self.assertEqual([guard.wanted("shift", shift), guard.wanted("shift", letter)], [True, False])

    def test_a_stopped_process_reads_as_stopped(self):
        self.assertTrue(guard.stopped_state("123 (a (b) c) T 1 2"))
        self.assertTrue(guard.stopped_state("123 (agent-desktop) t 1 2"))
        self.assertFalse(guard.stopped_state("123 (agent-desktop) S 1 2"))


class CatchTests(unittest.TestCase):
    def catch(self, found: list[dict], kind: str):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "key.jsonl"
            log.write_text("".join(json.dumps(item) + "\n" for item in found))
            running = subprocess.Popen(["sleep", "30"])
            try:
                with mock.patch.object(guard, "PROBE_S", 0.2):
                    held, stops = guard.catch_held(running, guard.Tail(log), kind)
                stopped = guard._stopped(running.pid)
            finally:
                running.send_signal(signal.SIGKILL)
                running.wait()
            return held, stops, stopped

    def test_a_held_letter_leaves_the_product_stopped_at_the_first_stop(self):
        held, stops, stopped = self.catch([key("press", "c", 54, 1.0)], "letter")
        self.assertEqual(([item["code"] for item in held], stops, stopped), ([54], 1, True))

    def test_a_released_letter_is_not_caught_and_the_product_runs_on(self):
        held, stops, stopped = self.catch([key("press", "c", 54, 1.0), key("release", "c", 54, 1.01)], "letter")
        self.assertEqual(held, [])
        self.assertGreater(stops, 1)
        self.assertFalse(stopped)

    def test_a_shift_round_does_not_take_a_held_letter(self):
        held, _, _ = self.catch([key("press", "c", 54, 1.0)], "shift")
        self.assertEqual(held, [])


class MeasureTests(unittest.TestCase):
    def test_a_guard_release_is_timed_from_the_press_and_from_the_kill(self):
        held = [key("press", "c", 54, 10.0)]
        found = held + [key("release", "c", 54, 10.25)]
        found = guard.guard_measure(found, 10.2, held, 600)
        self.assertEqual((found["release_after_press_ms"], found["release_after_kill_ms"], found["held_open"],
                          found["extra_presses"]), (250, 50, [], 0))

    def test_a_key_never_released_stays_open(self):
        held = [key("press", "c", 54, 10.0)]
        found = guard.guard_measure(held + [key("press", "c", 54, 10.7)], 10.2, held, 600)
        self.assertEqual((found["held_open"], found["release_after_press_ms"], found["extra_presses"]),
                         ([54], None, 1))

    def test_a_repeat_ends_only_at_the_final_release(self):
        held = [key("press", "c", 54, 10.0)]
        found = held + [key("release", "c", 54, 10.6), key("press", "c", 54, 10.61), key("release", "c", 54, 10.7)]
        found = guard.guard_measure(found, 10.2, held, 600)
        self.assertEqual((found["release_after_press_ms"], found["extra_presses"]), (700, 1))

    def test_extra_chars_count_what_the_entry_gained_beyond_the_typed_text(self):
        self.assertEqual(guard.extra_chars("xabc", "xabcdefg"), 0)
        self.assertEqual(guard.extra_chars("xabccc", "xabcdefg"), 2)

    def test_the_bystander_counts_keys_and_text_but_not_its_focus(self):
        found = [{"event": "focus", "in": True}, key("press", "c", 54, 1.0), {"event": "text", "text": "c"}]
        self.assertEqual(len(guard.bystander_events(found)), 2)


class VerdictTests(unittest.TestCase):
    def test_a_release_before_the_repeat_delay_with_nothing_extra_passes(self):
        self.assertEqual(guard.guard_problems("pid letter", measure(), -signal.SIGKILL), [])

    def test_a_key_left_held_fails(self):
        problems = guard.guard_problems("pid letter", measure(held_open=[54], release_after_press_ms=None),
                                        -signal.SIGKILL)
        self.assertIn("still held", " ".join(problems))

    def test_a_release_at_or_after_the_repeat_delay_fails(self):
        problems = guard.guard_problems("pid letter", measure(release_after_press_ms=600), -signal.SIGKILL)
        self.assertIn("600 ms repeat delay", " ".join(problems))

    def test_extra_characters_presses_or_bystander_events_fail(self):
        for field in ("extra_presses", "extra_chars", "bystander_events"):
            with self.subTest(field):
                problems = guard.guard_problems("group shift", measure(**{field: 1}), -signal.SIGKILL)
                self.assertEqual(problems, [f"group shift: {field} 1, not 0"])

    def test_an_exit_other_than_sigkill_fails(self):
        self.assertIn("not by SIGKILL", " ".join(guard.guard_problems("pid shift", measure(), 0)))


if __name__ == "__main__":
    unittest.main()
