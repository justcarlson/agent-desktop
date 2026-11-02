from __future__ import annotations

import itertools
import os
import signal
import sys
import tempfile
import types
import unittest
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_keys_cancel as cancel
from scenarios import hyprland_keys_controls as controls
from scenarios import hyprland_keys_keymap as keymap
from scenarios import hyprland_keys_refusal as refusal
from scenarios import hyprland_keys_theft as theft

NEW_KEY_LEGS = {
    "input.hyprland_headless_physical_keys_refused": (("headless",), "asserted-refusal"),
    "input.hyprland_cancel_releases_keys": (("headed",), "pass"),
    "input.hyprland_press_return_one_newline": (("headed",), "pass"),
    "input.hyprland_press_select_all_backspace_clears": (("headed",), "pass"),
    "input.hyprland_press_tab_moves_focus": (("headed",), "pass"),
    "input.hyprland_keymap_variant_options_preserved": (("headed",), "pass"),
    "input.hyprland_focus_theft_before_first_key": (("headed",), "asserted-refusal"),
}


def envelope(ok: bool, error: dict | None = None, exit_code: int | None = None) -> Envelope:
    code = exit_code if exit_code is not None else (0 if ok else 1)
    return Envelope(ok=ok, data={} if ok else None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=code)


def refused(code="STALE_REF", check="exact_window_focus", delivery="not_delivered", started=False) -> Envelope:
    details = {"check": check}
    if started is not None:
        details["physical_delivery_started"] = started
    return envelope(False, {"code": code, "details": details, "disposition": {"delivery": delivery}})


def key(event: str, keysym: str, code: int, state: int = 0) -> dict:
    return {"event": event, "keysym": keysym, "code": code, "state": state, "group": 0}


class NewKeyRegistryTests(unittest.TestCase):
    def test_every_new_key_leg_is_hyprland_only_reads_the_compositor_and_the_app_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id, (modes, disposition) in NEW_KEY_LEGS.items():
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), modes, disposition))
                self.assertTrue({"hyprland", "atspi", "applog"} <= set(leg.oracles))
                self.assertIn(leg_id, implemented)
                if disposition == "asserted-refusal":
                    self.assertTrue(leg.reason)

    def test_the_cancel_leg_reads_the_process_it_signals(self):
        self.assertIn("proc", legs.leg_by_id("input.hyprland_cancel_releases_keys").oracles)


class PressRefusalTests(unittest.TestCase):
    def test_a_not_delivered_refusal_with_the_named_check_passes(self):
        self.assertEqual(refusal.press_refusal_problems("p", refused(), "STALE_REF", "exact_window_focus"), [])
        self.assertEqual(refusal.press_refusal_problems("p", refused(started=None), "STALE_REF",
                                                        "exact_window_focus"), [])

    def test_a_delivery_another_code_check_or_disposition_or_a_started_delivery_fails(self):
        for case in (envelope(True), refused(code="ACTION_FAILED"), refused(check="window_active"),
                     refused(delivery="delivery_uncertain"), refused(started=True),
                     envelope(False, {"code": "STALE_REF", "details": {"check": "exact_window_focus"},
                                      "disposition": {"delivery": "not_delivered"}}, exit_code=2)):
            with self.subTest(case=case.error):
                self.assertNotEqual(refusal.press_refusal_problems("p", case, "STALE_REF", "exact_window_focus"), [])


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

    def test_a_cut_text_is_a_non_empty_strict_prefix(self):
        self.assertEqual(cancel.prefix_problems("aBa", "aBaB"), [])
        for typed in ("", "aBaB", "ab", None):
            with self.subTest(typed=typed):
                self.assertNotEqual(cancel.prefix_problems(typed, "aBaB"), [])


class ControlVerdictTests(unittest.TestCase):
    def test_return_gives_exactly_one_newline(self):
        self.assertEqual(controls.newline_problems("ab\nc", "ab\nc"), [])
        for text in ("ab\n\nc", "abc", "ab\r\nc", None):
            with self.subTest(text=text):
                self.assertNotEqual(controls.newline_problems(text, "ab\nc"), [])

    def test_backspace_after_select_all_must_arrive_without_control(self):
        clean = [key("press", "Control_L", 37), key("press", "a", 38, 4), key("release", "a", 38, 4),
                 key("release", "Control_L", 37, 4), key("press", "BackSpace", 22), key("release", "BackSpace", 22)]
        self.assertEqual(controls.backspace_problems(clean), [])
        held = [key("press", "Control_L", 37), key("press", "BackSpace", 22, 4)]
        self.assertNotEqual(controls.backspace_problems(held), [])
        self.assertNotEqual(controls.backspace_problems(clean[:4]), [])

    def test_tab_must_move_focus_to_the_password_and_insert_nothing(self):
        self.assertEqual(controls.tab_problems("xy", "xy", focused_password=True, password_focus_events=1), [])
        for case in (("xy\t", "xy", True, 1), ("xy", "xy", False, 1), ("xy", "xy", True, 0)):
            with self.subTest(case=case):
                self.assertNotEqual(controls.tab_problems(case[0], case[1], focused_password=case[2],
                                                          password_focus_events=case[3]), [])


class KeymapVerdictTests(unittest.TestCase):
    def test_the_same_layout_variant_and_options_pass(self):
        read = {"kb_layout": "us", "kb_variant": "dvorak", "kb_options": "ctrl:nocaps",
                "keyboards": [("kbd", "us", "dvorak", "ctrl:nocaps", "English (Dvorak)")]}
        self.assertEqual(keymap.readback_problems(read, dict(read)), [])

    def test_each_changed_field_is_named(self):
        read = {"kb_layout": "us", "kb_variant": "dvorak", "kb_options": "ctrl:nocaps", "keyboards": []}
        for field, value in (("kb_variant", ""), ("kb_options", "compose:caps"), ("kb_layout", "us,ru"),
                             ("keyboards", [("kbd", "us", "", "", "English (US)")])):
            with self.subTest(field=field):
                problems = keymap.readback_problems(read, {**read, field: value})
                self.assertEqual(len(problems), 1)
                self.assertIn(field, problems[0])

    def test_the_variant_places_each_letter_on_its_own_key(self):
        found = [key("press", "Shift_L", 50), key("press", "H", 44, 1), key("press", "e", 40)]
        self.assertEqual(keymap.placement_problems(found, {"h": 44, "e": 40}), [])
        self.assertNotEqual(keymap.placement_problems(found, {"h": 43, "e": 40}), [])
        self.assertNotEqual(keymap.placement_problems(found, {"h": 44, "e": 40, "d": 41}), [])

    def test_an_input_method_keyboard_still_on_the_old_keymap_reads_as_settled(self):
        us = ("us", "", "compose:caps", "English (US)")
        dvorak = ("us", "dvorak", "ctrl:nocaps", "English (Dvorak)")
        lagging = {"keyboards": [("kbd", *us), ("hl-virtual-keyboard-fcitx5", *us)]}
        synced = {"kb_variant": "dvorak", "keyboards": [("kbd", *dvorak), ("hl-virtual-keyboard-fcitx5", *dvorak)]}
        behind = {"kb_variant": "dvorak", "keyboards": [("kbd", *dvorak), ("hl-virtual-keyboard-fcitx5", *us)]}
        self.assertEqual(keymap.ime_settled(behind, lagging), synced)
        self.assertEqual(keymap.ime_settled(synced, lagging), synced)
        stray = {"keyboards": [("kbd", *dvorak), ("hl-virtual-keyboard-fcitx5", "us", "", "", "English (US)")]}
        self.assertEqual(keymap.ime_settled(stray, lagging), stray)
        other = {"keyboards": [("kbd", *dvorak), ("other-virtual", *us)]}
        self.assertEqual(keymap.ime_settled(other, {"keyboards": [("kbd", *us), ("other-virtual", *us)]}), other)

    def test_the_leg_types_a_key_after_the_restore_so_the_input_method_resyncs(self):
        calls = []
        with mock.patch.object(keymap, "_focused_key_log") as focused, \
                mock.patch.object(keymap, "_typed", side_effect=lambda *args: calls.append(args[3:]) or []):
            focused.return_value.__enter__.return_value = ("app", "log", "@s1:e1")
            keymap.resync_input_method(types.SimpleNamespace(finding=lambda text: None))
        self.assertEqual(calls, [(keymap.SYNC_KEY, keymap.SYNC_KEY)])


class TheftVerdictTests(unittest.TestCase):
    def test_a_not_delivered_focus_refusal_is_a_stop_before_the_first_key(self):
        for case in (refused(code="ACTION_FAILED", check="window_active"),
                     refused(code="ACTION_FAILED", check="target_focus", started=None)):
            with self.subTest(case=case.error):
                self.assertEqual(theft.theft_outcome(case), "refused_before_first_key")

    def test_a_late_steal_a_delivery_or_any_other_answer_is_named(self):
        late = envelope(False, {"code": "ACTION_FAILED", "details": {"physical_delivery_started": True,
                                                                     "delivered_events": 2},
                                "disposition": {"delivery": "delivery_uncertain"}})
        self.assertEqual(theft.theft_outcome(late), "stopped_after_keys")
        self.assertEqual(theft.theft_outcome(envelope(True)), "delivered")
        self.assertEqual(theft.theft_outcome(refused(code="INTERNAL", check="window_active")), "other")
        self.assertEqual(theft.theft_outcome(refused(code="ACTION_FAILED", check="window_active", started=True)),
                         "other")
        self.assertEqual(theft.theft_outcome(None), "other")


class LiveHelperTests(unittest.TestCase):
    def test_tab_reads_the_password_state_as_a_property(self):
        node = types.SimpleNamespace(states=frozenset({controls.FOCUSED}))
        zoo = mock.Mock()
        zoo.events.side_effect = itertools.chain([[]], itertools.repeat([{"event": "focus"}]))
        with mock.patch.object(controls, "zoo_ref", return_value="@s:e1"), \
                mock.patch.object(controls, "zoo_typed", return_value=[]), \
                mock.patch.object(controls, "zoo_pressed", return_value=[]), \
                mock.patch.object(controls, "zoo_node", return_value=node), \
                mock.patch.object(controls, "zoo_text", side_effect=["", "xy"]):
            self.assertEqual(controls.tab_moves_focus(types.SimpleNamespace(tier=None), zoo), [])

    def test_the_steal_waits_until_the_product_has_focused_the_target(self):
        reads = iter(["thief", "thief", "target"])
        steal = mock.Mock()
        self.assertTrue(theft.steal_once_active(lambda: next(reads), "target", steal, 2.0))
        steal.assert_called_once_with()

    def test_no_steal_when_the_target_never_takes_focus(self):
        steal = mock.Mock()
        self.assertFalse(theft.steal_once_active(lambda: "thief", "target", steal, 0.05))
        steal.assert_not_called()

    def test_keys_sent_before_the_steal_make_a_late_steal(self):
        self.assertTrue(theft.late_steal("stopped_after_keys"))
        self.assertTrue(theft.late_steal("delivered"))
        self.assertFalse(theft.late_steal("refused_before_first_key"))
        self.assertFalse(theft.late_steal("other"))

    def test_app_evidence_names_whether_the_app_lives_its_stderr_and_its_last_records(self):
        with tempfile.TemporaryDirectory() as runtime:
            Path(runtime, "key-log-cancel.log").write_text("Gdk-Message: Error 71 (Protocol error)\n")
            found = cancel.app_evidence(Path(runtime), "cancel", os.getpid(), [{"event": "press", "code": 50}])
        self.assertIn("alive", found)
        self.assertIn("Error 71", found)
        self.assertIn("'code': 50", found)


if __name__ == "__main__":
    unittest.main()
