from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope

FOCUS_LEGS = {
    "focus.headless_existing_focus_or_refusal": (("headless",), "pass"),
    "focus.contradictory_or_missing_proof_refused": (("headed",), "asserted-refusal"),
}
REUSED = {
    "ref_actions.focus": ("headed",),
    "ref_actions.focus_headless_refusal": ("headless",),
    "lifecycle.close_app_undelivered_report": ("headless",),
}


def envelope(ok: bool, *, data: dict | None = None, error: dict | None = None, elapsed_ms: float = 40.0,
             exit_code: int | None = None) -> Envelope:
    code = exit_code if exit_code is not None else (0 if ok else 1)
    return Envelope(ok=ok, data=data, error=error, raw="{}", elapsed_ms=elapsed_ms, argv=[], exit_code=code)


SENT = "Hyprland made the window active but the application does not report it active; the activation was sent"


def unverified(cause="frame_not_active", sent=True, delivery="not_delivered", started=False,
               code="ACTION_FAILED", message=SENT, side_effects=None) -> Envelope:
    details = {"cause": cause, "activation_sent": sent,
               "physical_delivery_started": started, "compositor_active": True,
               "toplevel_activated": True, "in_view": True, "frame_active": False}
    if side_effects is not None:
        details["side_effects"] = side_effects
    return envelope(False, error={"code": code, "message": message, "details": details,
                                  "disposition": {"delivery": delivery, "retry": "safe"}})


MOVED = [{"kind": "cursor_moved", "from": {"x": 640.0, "y": 400.0}, "to": {"x": 957.0, "y": 413.0}}]


class FocusProofRegistryTests(unittest.TestCase):
    def test_the_new_focus_legs_are_hyprland_rows_that_read_the_compositor_the_tree_and_the_app(self):
        for leg_id, (modes, disposition) in FOCUS_LEGS.items():
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), modes, disposition))
                self.assertTrue({"hyprland", "atspi", "applog"} <= set(leg.oracles))
                if disposition == "asserted-refusal":
                    self.assertTrue(leg.reason)

    def test_the_reused_rows_add_hyprland_and_keep_their_x11_tiers_and_modes(self):
        for leg_id, modes in REUSED.items():
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertIn("hyprland", leg.tiers)
                self.assertIn("x11-openbox", leg.tiers)
                self.assertEqual(leg.modes, modes)
                self.assertIn("hyprland", leg.oracles)

    def test_every_leg_of_the_slice_has_an_implementation(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in (*FOCUS_LEGS, *REUSED):
            with self.subTest(leg=leg_id):
                self.assertIn(leg_id, implemented)


class ExistingFocusTests(unittest.TestCase):
    def setUp(self):
        from scenarios import hyprland_focus_proof

        self.proof = hyprland_focus_proof

    def test_a_proof_without_activation_passes(self):
        answer = envelope(True, data={"activation": {"activated": False, "proof": "compositor_and_accessibility"}})
        self.assertEqual(self.proof.existing_focus_problems(answer), [])

    def test_an_activation_a_refusal_or_a_missing_report_fails(self):
        for case in (envelope(True, data={"activation": {"activated": True}}), envelope(True, data={}),
                     envelope(False, error={"code": "POLICY_DENIED"})):
            with self.subTest(case=case.data):
                self.assertNotEqual(self.proof.existing_focus_problems(case), [])

    def test_the_focus_witnesses_must_agree_for_the_target(self):
        target = {"address": "0xa", "pid": 7}
        self.assertEqual(self.proof.agreement_problems("t", target, {"address": "0xa", "pid": 7}, True, True), [])
        for active, shown, frame in (({"address": "0xb", "pid": 8}, True, True),
                                     ({"address": "0xa", "pid": 7}, False, True),
                                     ({"address": "0xa", "pid": 7}, True, False)):
            with self.subTest(active=active, shown=shown, frame=frame):
                self.assertNotEqual(self.proof.agreement_problems("t", target, active, shown, frame), [])


class ContradictoryProofTests(unittest.TestCase):
    def setUp(self):
        from scenarios import hyprland_focus_proof

        self.proof = hyprland_focus_proof

    def test_a_reported_activation_that_withheld_input_passes(self):
        self.assertEqual(self.proof.activation_report_problems("p", unverified(), "frame_not_active"), [])

    def test_a_claim_of_no_side_effect_or_a_missing_activation_fails(self):
        for case in (unverified(delivery="delivered_unverified"), unverified(sent=False), unverified(started=True),
                     unverified(message="Hyprland made the window active"),
                     unverified(cause="activation_refused"), unverified(code="STALE_REF"),
                     envelope(True, data={})):
            with self.subTest(case=case.error):
                self.assertNotEqual(self.proof.activation_report_problems("p", case, "frame_not_active"), [])

    def test_a_cursor_the_activation_moved_must_be_reported_as_a_side_effect(self):
        here, there = {"x": 640, "y": 400}, {"x": 957, "y": 413}
        self.assertEqual(self.proof.side_effect_problems("p", unverified(side_effects=MOVED), here, there), [])
        self.assertEqual(self.proof.side_effect_problems("p", unverified(), here, here), [])
        self.assertNotEqual(self.proof.side_effect_problems("p", unverified(), here, there), [])

    def test_a_refusal_must_come_back_within_the_press_deadline_and_a_second(self):
        self.assertEqual(self.proof.DEADLINE_MS, 5000)
        on_time = envelope(False, error=unverified().error, elapsed_ms=4758.0)
        self.assertEqual(self.proof.deadline_problems("p", on_time, self.proof.DEADLINE_MS), [])
        late = envelope(False, error=unverified().error, elapsed_ms=6100.0)
        self.assertNotEqual(self.proof.deadline_problems("p", late, self.proof.DEADLINE_MS), [])

    def test_a_click_refused_before_activation_names_its_cause_and_sends_nothing(self):
        early = envelope(False, error={"code": "ACTION_FAILED",
                                       "details": {"cause": "native_wayland_client", "physical_delivery_started": False},
                                       "disposition": {"delivery": "not_delivered", "retry": "safe"}})
        self.assertEqual(self.proof.unactivated_problems("c", early, "native_wayland_client"), [])
        for case in (unverified(cause="native_wayland_client"), unverified(),
                     envelope(True, data={"disposition": {"delivery": "delivered_unverified"}})):
            with self.subTest(case=case.raw):
                self.assertNotEqual(self.proof.unactivated_problems("c", case, "native_wayland_client"), [])

    def test_the_fixture_withholds_its_active_state_while_the_flag_exists(self):
        script = self.proof.APP_SCRIPT.read_text()
        self.assertIn(f'APP_CLASS = "{self.proof.APP_CLASS}"', script)
        self.assertIn("GLib.set_prgname(APP_CLASS)", script)
        self.assertIn('"focus-in-event"', script)
        self.assertIn("os.path.exists(flag)", script)
        self.assertIn('os.environ["GDK_BACKEND"] = sys.argv[3]', script)
        self.assertIn(self.proof.APP_CLASS, self.proof.APP_NAMES)


class StoppedCloseTests(unittest.TestCase):
    def setUp(self):
        from scenarios import hyprland_close_stopped

        self.close = hyprland_close_stopped

    def test_a_not_delivered_report_on_a_live_listed_process_passes(self):
        report = envelope(False, error={"code": "APP_UNRESPONSIVE",
                                        "disposition": {"delivery": "not_delivered", "retry": "safe"}})
        self.assertEqual(self.close.stopped_close_problems(report, True, (["0x1"], ["0x1"])), [])

    def test_a_delivered_claim_a_kill_or_a_lost_toplevel_fails(self):
        refused = envelope(False, error={"code": "APP_UNRESPONSIVE",
                                         "disposition": {"delivery": "not_delivered", "retry": "safe"}})
        delivered = envelope(True, data={"disposition": {"delivery": "delivered_verified"}})
        for report, alive, toplevels in ((delivered, True, (["0x1"], ["0x1"])), (refused, False, (["0x1"], ["0x1"])),
                                         (refused, True, (["0x1"], []))):
            with self.subTest(report=report.ok, alive=alive, toplevels=toplevels):
                self.assertNotEqual(self.close.stopped_close_problems(report, alive, toplevels), [])


class QtLaunchTests(unittest.TestCase):
    def test_the_hyprland_route_runs_qt_on_wayland_with_accessibility_on(self):
        from scenarios import hyprland_ref_focus

        self.assertEqual(hyprland_ref_focus.QT_ENV["QT_QPA_PLATFORM"], "wayland")
        self.assertEqual(hyprland_ref_focus.QT_ENV["QT_LINUX_ACCESSIBILITY_ALWAYS_ON"], "1")
        self.assertTrue(hyprland_ref_focus.QT_SCRIPT.name == "qt_window.py" and hyprland_ref_focus.QT_SCRIPT.exists())
        self.assertIn("qt_window.py", hyprland_ref_focus.QT_NAMES)


class RoutingTests(unittest.TestCase):
    def test_ref_focus_on_hyprland_runs_the_hyprland_route(self):
        from scenarios import hyprland_ref_focus, ref_focus

        ctx = SimpleNamespace(tier=SimpleNamespace(name="hyprland"), mode="headed")
        with mock.patch.object(hyprland_ref_focus, "focus", return_value="routed") as route:
            ref_focus.focus(ctx)
        route.assert_called_once_with(ctx)

    def test_close_app_undelivered_on_hyprland_runs_the_hyprland_route(self):
        from scenarios import hyprland_close_stopped, lifecycle

        ctx = SimpleNamespace(tier=SimpleNamespace(name="hyprland"), mode="headless")
        with mock.patch.object(hyprland_close_stopped, "close_app_undelivered_report") as route:
            lifecycle.close_app_undelivered_report(ctx)
        route.assert_called_once_with(ctx)


if __name__ == "__main__":
    unittest.main()
