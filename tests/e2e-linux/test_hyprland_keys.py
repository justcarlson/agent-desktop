from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_keys as keys
from scenarios import hyprland_keys_common as common

KEY_LEGS = ("input.hyprland_type_text_virtual_keyboard", "input.hyprland_press_chord_virtual_keyboard",
            "input.hyprland_type_non_us_layout", "input.hyprland_focus_change_mid_batch_uncertain")


def envelope(ok: bool, error: dict | None = None, data: dict | None = None) -> Envelope:
    return Envelope(ok=ok, data=data, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0 if ok else 1)


def uncertain(delivered, started=True) -> Envelope:
    return envelope(False, {"code": "ACTION_FAILED",
                            "details": {"delivered_events": delivered, "physical_delivery_started": started},
                            "disposition": {"delivery": "delivery_uncertain", "retry": "unsafe"}})


class KeyRegistryTests(unittest.TestCase):
    def test_every_key_leg_is_headed_hyprland_reads_the_app_log_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in KEY_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headed",), "pass"))
                self.assertTrue({"hyprland", "atspi", "applog"} <= set(leg.oracles))
                self.assertIn(leg_id, implemented)

    def test_the_app_takes_the_program_name_the_leg_places_and_finds(self):
        script = common.APP_SCRIPT.read_text()
        self.assertIn(f'KEY_LOG_CLASS = "{common.KEY_LOG_CLASS}"', script)
        self.assertIn("GLib.set_prgname(KEY_LOG_CLASS)", script)
        self.assertIn(common.KEY_LOG_CLASS, common.KEY_LOG_NAMES)


class KeyVerdictTests(unittest.TestCase):
    def test_a_partial_uncertain_stop_passes(self):
        self.assertEqual(keys.uncertain_problems(uncertain(12), 900), [])

    def test_a_success_none_or_all_delivered_or_an_unstarted_stop_fails(self):
        for case in (envelope(True, data={}), uncertain(0), uncertain(900), uncertain("12"),
                     uncertain(12, started=False)):
            with self.subTest(case=case.error):
                self.assertNotEqual(keys.uncertain_problems(case, 900), [])

    def test_a_not_delivered_refusal_is_not_an_uncertain_stop(self):
        refused = envelope(False, {"code": "ACTION_FAILED", "details": {"delivered_events": 3},
                                   "disposition": {"delivery": "not_delivered", "retry": "safe"}})
        self.assertNotEqual(keys.uncertain_problems(refused, 900), [])

    def test_a_step_label_must_name_the_virtual_keyboard(self):
        steps = {"steps": [{"label": "VirtualKeyboard.type_text"}]}
        self.assertEqual(common.labelled("t", envelope(True, data=steps), "VirtualKeyboard.type_text"), [])
        portal = {"steps": [{"label": "RemoteDesktop.type_text"}]}
        self.assertNotEqual(common.labelled("t", envelope(True, data=portal), "VirtualKeyboard.type_text"), [])
        self.assertNotEqual(common.labelled("t", envelope(False, error={}), "VirtualKeyboard.type_text"), [])


if __name__ == "__main__":
    unittest.main()
