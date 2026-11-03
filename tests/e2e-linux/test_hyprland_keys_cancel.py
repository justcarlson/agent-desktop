from __future__ import annotations

import os
import signal
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from scenarios import hyprland_keys_cancel as cancel

LEG = "input.hyprland_cancel_releases_keys"


def key(event: str, keysym: str, code: int, state: int = 0) -> dict:
    return {"event": event, "keysym": keysym, "code": code, "state": state, "group": 0}


class CancelRegistryTests(unittest.TestCase):
    def test_the_cancel_leg_is_hyprland_only_headed_and_implemented(self):
        import scenarios  # noqa: F401

        leg = legs.leg_by_id(LEG)
        self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headed",), "pass"))
        self.assertTrue({"hyprland", "atspi", "applog"} <= set(leg.oracles))
        self.assertIn(LEG, legs.implementations())

    def test_the_cancel_leg_reads_the_process_it_signals(self):
        self.assertIn("proc", legs.leg_by_id("input.hyprland_cancel_releases_keys").oracles)


class CancelVerdictTests(unittest.TestCase):
    def test_held_keys_names_each_code_pressed_and_not_released(self):
        found = [key("press", "Shift_L", 50), key("press", "B", 56), key("release", "B", 56),
                 {"event": "text", "text": "B"}, key("press", "a", 38)]
        self.assertEqual(cancel.held_keys(found), [50, 38])
        self.assertEqual(cancel.held_keys(found + [key("release", "Shift_L", 50), key("release", "a", 38)]), [])

    def test_a_modifier_is_in_flight_only_when_the_newest_key_event_presses_one(self):
        self.assertTrue(cancel.modifier_in_flight([key("press", "a", 38), key("press", "Shift_L", 50),
                                                   {"event": "text", "text": "a"}]))
        self.assertFalse(cancel.modifier_in_flight([key("press", "Shift_L", 50), key("release", "Shift_L", 50)]))
        self.assertFalse(cancel.modifier_in_flight([key("press", "a", 38)]))
        self.assertFalse(cancel.modifier_in_flight([]))

    def test_the_next_press_passes_with_the_lead_shift_tap_and_one_clean_key(self):
        clean = [key("press", "Shift_L", 50), key("release", "Shift_L", 50, 1), key("press", "x", 61),
                 key("release", "x", 61)]
        self.assertEqual(cancel.next_press_problems(clean, "x"), [])

    def test_the_next_press_fails_with_a_held_modifier_an_extra_key_or_no_key(self):
        stuck = [key("press", "X", 61, 1), key("release", "X", 61, 1)]
        extra = [key("press", "x", 61), key("release", "x", 61), key("press", "B", 56), key("release", "B", 56)]
        twice = [key("press", "x", 61), key("release", "x", 61)] * 2
        held = [key("press", "x", 61)]
        for case in (stuck, extra, twice, held, []):
            with self.subTest(case=case):
                self.assertNotEqual(cancel.next_press_problems(case, "x"), [])

    def test_the_process_must_die_of_the_signal_sent(self):
        self.assertEqual(cancel.exit_problems(signal.SIGKILL, -signal.SIGKILL), [])
        self.assertNotEqual(cancel.exit_problems(signal.SIGTERM, 0), [])
        self.assertNotEqual(cancel.exit_problems(signal.SIGTERM, -signal.SIGKILL), [])
        self.assertNotEqual(cancel.exit_problems(signal.SIGTERM, None), [])

    def test_a_key_left_held_counts_against_sigterm_but_not_sigkill(self):
        self.assertEqual(cancel.held_after_signal_problems(signal.SIGKILL, [50]), [])
        self.assertEqual(cancel.held_after_signal_problems(signal.SIGTERM, []), [])
        self.assertEqual(cancel.held_after_signal_problems(signal.SIGTERM, [50]),
                         ["SIGTERM: the app saw keys [50] pressed and never released"])

    def test_a_key_held_after_the_signal_repeats_characters_until_the_next_delivery_releases_it(self):
        def at(t: float, item: dict) -> dict:
            return dict(item, t=t)

        found = [at(1.0, key("press", "a", 38)), at(1.01, {"event": "text", "text": "a"}),
                 at(2.5, key("release", "a", 38)), at(2.5, key("press", "a", 38)),
                 at(2.51, {"event": "text", "text": "aa"}), at(2.6, key("release", "a", 38)),
                 at(2.6, key("press", "a", 38)), at(2.61, {"event": "text", "text": "aaa"}),
                 at(4.1, key("release", "a", 38)), at(4.2, key("press", "x", 53)),
                 at(4.21, {"event": "text", "text": "aaax"}), at(4.3, key("release", "x", 53))]
        self.assertEqual(cancel.hold_measure(found, 2.0, 4.0, "aBaB"),
                         {"held": [38], "held_at_next": [38], "held_s": 2.1, "window_s": 2.0, "repeat_chars": 2,
                          "focus_left": False})

    def test_keys_in_flight_after_the_signal_count_only_until_the_app_sees_them_released(self):
        def at(t: float, item: dict) -> dict:
            return dict(item, t=t)

        found = [at(0.9, {"event": "text", "text": "a"}), at(1.0, key("press", "Shift_L", 50)),
                 at(2.01, key("press", "B", 56, 1)), at(2.015, {"event": "text", "text": "aB"}),
                 at(2.02, key("release", "B", 56, 1)), at(2.03, key("release", "Shift_L", 50, 1))]
        self.assertEqual(cancel.hold_measure(found, 2.0, 4.0, "aBaB"),
                         {"held": [50], "held_at_next": [], "held_s": 0.03, "window_s": 2.0, "repeat_chars": 0,
                          "focus_left": False})

    def test_a_held_shift_repeats_no_character_and_an_unreleased_key_has_no_held_time(self):
        found = [dict(key("press", "Shift_L", 50), t=1.0), {"event": "focus", "in": False, "t": 3.0}]
        self.assertEqual(cancel.hold_measure(found, 2.0, 4.0, "aBaB"),
                         {"held": [50], "held_at_next": [50], "held_s": None, "window_s": 2.0, "repeat_chars": 0,
                          "focus_left": True})

    def test_after_sigkill_a_repeated_last_character_still_reads_as_a_cut_prefix(self):
        self.assertEqual(cancel.collapse_repeats("aBaaaa"), "aBa")
        self.assertEqual(cancel.collapse_repeats(""), "")
        self.assertEqual(cancel.prefix_problems(cancel.collapse_repeats("aBaaaa"), "aBaB"), [])
        self.assertNotEqual(cancel.prefix_problems("aBaaaa", "aBaB"), [])

    def test_after_sigkill_the_next_press_may_follow_more_repeats_of_the_held_key(self):
        repeats = [key("press", "a", 38), key("release", "a", 38)] * 2 + [key("press", "x", 61),
                                                                          key("release", "x", 61)]
        self.assertEqual(cancel.next_press_problems(repeats, "x", {38}), [])
        self.assertNotEqual(cancel.next_press_problems(repeats, "x"), [])
        self.assertTrue(cancel.next_text_matches("aBaaaaax", "aBaaa", "x", repeated=True))
        self.assertFalse(cancel.next_text_matches("aBaaaaax", "aBaaa", "x", repeated=False))
        self.assertTrue(cancel.next_text_matches("aBax", "aBa", "x", repeated=False))
        self.assertFalse(cancel.next_text_matches("aBaBx", "aBa", "x", repeated=True))
        self.assertFalse(cancel.next_text_matches(None, "aBa", "x", repeated=True))

    def test_a_cut_text_is_a_non_empty_strict_prefix(self):
        self.assertEqual(cancel.prefix_problems("aBa", "aBaB"), [])
        for typed in ("", "aBaB", "ab", None):
            with self.subTest(typed=typed):
                self.assertNotEqual(cancel.prefix_problems(typed, "aBaB"), [])

    def test_app_evidence_names_whether_the_app_lives_its_stderr_and_its_last_records(self):
        with tempfile.TemporaryDirectory() as runtime:
            Path(runtime, "key-log-cancel.log").write_text("Gdk-Message: Error 71 (Protocol error)\n")
            found = cancel.app_evidence(Path(runtime), "cancel", os.getpid(), [{"event": "press", "code": 50}])
        self.assertIn("alive", found)
        self.assertIn("Error 71", found)
        self.assertIn("'code': 50", found)


if __name__ == "__main__":
    unittest.main()


class CancelDebugLogTests(unittest.TestCase):
    def test_each_signal_gets_its_own_product_log_in_the_runtime_dir(self):
        runtime = Path("/run/user/1000/rt")
        self.assertEqual(cancel.product_log(runtime, signal.SIGTERM), runtime / "product-cancel-SIGTERM.log")
        self.assertNotEqual(cancel.product_log(runtime, signal.SIGTERM), cancel.product_log(runtime, signal.SIGKILL))

    def test_a_debug_log_receives_the_products_stderr_under_the_debug_filter(self):
        from types import SimpleNamespace
        from unittest import mock

        from scenarios import hyprland_keys_process as process

        with tempfile.TemporaryDirectory() as scratch:
            log = Path(scratch) / "product.log"
            tier = SimpleNamespace(env={"PATH": os.environ.get("PATH", "")})
            with mock.patch.object(process.product, "_binary_path", return_value="/bin/sh"):
                running = process.running_product(tier, ["-c", 'echo "$RUST_LOG" >&2'], headed=False,
                                                  debug_log=log)
                code, _ = process.finished(running, 10.0)
            self.assertEqual(code, 0)
            self.assertEqual(log.read_text().strip(), process.DEBUG_FILTER)

    def test_without_a_debug_log_the_product_has_no_debug_filter(self):
        from types import SimpleNamespace
        from unittest import mock

        from scenarios import hyprland_keys_process as process

        tier = SimpleNamespace(env={"PATH": os.environ.get("PATH", "")})
        with mock.patch.object(process.product, "_binary_path", return_value="/bin/sh"):
            running = process.running_product(tier, ["-c", 'echo "[$RUST_LOG]"'], headed=False)
            code, out = process.finished(running, 10.0)
        self.assertEqual((code, out.strip()), (0, "[]"))
