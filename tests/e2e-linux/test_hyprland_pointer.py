from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_pointer as pointer
from scenarios import hyprland_pointer_pad as pad
from scenarios import hyprland_pointer_refs as refs
from scenarios import hyprland_receiver_fixture as fixture

NEW_LEGS = (
    "pointer.global_layout_coordinates",
    "pointer.ref_receiver_evidence",
    "pointer.cover_or_missing_evidence_refused",
    "pointer.coordinates_have_no_receiver_proof",
    "pointer.headless_or_cancelled_input",
    "pointer.cancel_or_disconnect_releases_buttons",
    "pointer.native_wayland_ref_refused",
)
REUSED_LEGS = (
    "pointer.mouse_click_delivery_report",
    "pointer.mouse_wheel_delivery_report",
    "pointer.drag_delivery_report",
    "ref_actions.scroll",
    "ref_actions.scroll_headless_refusal",
    "ref_actions.double_click",
    "ref_actions.double_click_headless_refusal",
    "ref_actions.triple_click",
    "ref_actions.triple_click_headless_refusal",
)
LOG = [
    {"ev": "motion", "widget": "area", "wx": 5, "wy": 5, "t": 1.0},
    {"ev": "press", "widget": "area", "n": 1, "button": 1, "wx": 10, "wy": 20, "t": 2.0},
    {"ev": "release", "widget": "area", "button": 1, "wx": 30, "wy": 40, "t": 3.0},
    {"ev": "motion", "widget": "area", "wx": 31, "wy": 41, "t": 4.0},
    {"ev": "press", "widget": "button", "n": 1, "button": 1, "wx": 1, "wy": 1, "t": 5.0},
    {"ev": "press", "widget": "button", "n": 2, "button": 1, "wx": 1, "wy": 1, "t": 5.1},
    {"ev": "scroll", "widget": "area", "direction": "smooth", "t": 6.0},
    {"ev": "scroll", "widget": "area", "direction": "down", "t": 6.1},
]


def refused(code: str, delivery: str = "not_delivered", **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details,
             "disposition": {"delivery": delivery, "retry": "safe"}}
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=1.0,
                    argv=["agent-desktop"], exit_code=1)


class PadLogTests(unittest.TestCase):
    def test_presses_keep_gtk_counts_per_widget(self):
        self.assertEqual(pad.presses(LOG, "button"), [1, 2])
        self.assertEqual(pad.presses(LOG, "area"), [1])

    def test_scrolls_keep_discrete_events_over_smooth_ones(self):
        self.assertEqual(pad.scrolls(LOG, "area"), ["down"])

    def test_scrolls_read_a_smooth_wheel_from_the_sign_of_its_delta(self):
        smooth = [{"ev": "scroll", "widget": "area", "direction": "smooth", "dx": dx, "dy": dy, "t": 1.0}
                  for dx, dy in ((0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 0.0), (None, None))]
        self.assertEqual(pad.scrolls(smooth, "area"), ["down", "up", "right", "left"])

    def test_gdk3_presses_add_the_synthetic_double_and_triple_press(self):
        self.assertEqual(pad.gdk3_presses(1), [1])
        self.assertEqual(pad.gdk3_presses(2), [1, 1, 2])
        self.assertEqual(pad.gdk3_presses(3), [1, 1, 2, 1, 3])

    def test_pad_saw_counts_each_event_kind_per_widget(self):
        self.assertEqual(pad.pad_saw(LOG), " (pad saw: motion/area 2, press/area 1, press/button 2, "
                                           "release/area 1, scroll/area 2)")
        self.assertEqual(pad.pad_saw([]), " (pad saw: nothing)")

    def test_since_drops_the_events_before_the_mark(self):
        self.assertEqual(pad.since(LOG, 6), LOG[6:])

    def test_releases_and_late_motion(self):
        self.assertEqual(len(pad.releases(LOG, "area")), 1)
        self.assertEqual(pad.motions_after(LOG, 3.0), [LOG[3]])

    def test_events_read_the_log_lines(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "pad.events"
            log.write_text("".join(json.dumps(item) + "\n" for item in LOG[:2]) + "\n")
            self.assertEqual(pad.events(log), LOG[:2])


class GeometryTests(unittest.TestCase):
    client = {"at": [-1024, -640], "size": [640, 480]}

    def test_holds_takes_client_and_layer_boxes(self):
        self.assertTrue(pad.holds((-1024, -640), self.client))
        self.assertFalse(pad.holds((-384, -640), self.client))
        self.assertTrue(pad.holds((5, 5), {"x": 0, "y": 0, "w": 10, "h": 10}))
        self.assertFalse(pad.holds((10, 5), {"x": 0, "y": 0, "w": 10, "h": 10}))

    def test_covering_layers_counts_only_top_and_overlay(self):
        layers = {"DP-1": {"levels": {
            "0": [{"x": 0, "y": 0, "w": 100, "h": 100, "namespace": "wallpaper"}],
            "2": [{"x": 0, "y": 0, "w": 100, "h": 30, "namespace": "omarchy-bar"}],
            "3": [{"x": 50, "y": 50, "w": 50, "h": 50, "namespace": "omarchy-notifications"}],
        }}}
        self.assertEqual(pad.covering_layers(layers, (10, 10)), ["omarchy-bar"])
        self.assertEqual(pad.covering_layers(layers, (60, 60)), ["omarchy-notifications"])
        self.assertEqual(pad.covering_layers(layers, (10, 60)), [])

    def test_global_and_window_points_round_trip(self):
        point = pad.global_point(self.client, (0.5, 0.5))
        self.assertEqual(point, (-704, -400))
        self.assertEqual(pad.window_point(point, self.client), (320, 240))
        self.assertEqual(pad.xy((-704.6, -400.2)), "-704,-400")

    def test_lands_at_allows_one_pixel(self):
        self.assertTrue(pad.lands_at({"wx": 11, "wy": 19}, (10, 20)))
        self.assertFalse(pad.lands_at({"wx": 12, "wy": 20}, (10, 20)))


class EnvelopeCheckTests(unittest.TestCase):
    def test_no_receiver_proof_needs_false_and_delivered_events(self):
        self.assertEqual(pointer.no_receiver_proof("c", {"details": {"receiver_proof": False,
                                                                     "delivered_events": 3}}), [])
        self.assertEqual(len(pointer.no_receiver_proof("c", {"details": {"receiver_proof": True,
                                                                         "delivered_events": 3}})), 1)
        self.assertEqual(len(pointer.no_receiver_proof("c", {"receiver_proof": False})), 2)

    def test_refusal_problems_want_policy_denied_before_delivery(self):
        self.assertEqual(pointer.refusal_problems("c", refused("POLICY_DENIED")), [])
        self.assertTrue(pointer.refusal_problems("c", refused("ACTION_FAILED")))
        self.assertTrue(pointer.refusal_problems("c", refused("POLICY_DENIED", physical_delivery_started=True)))
        self.assertTrue(pointer.refusal_problems("c", refused("POLICY_DENIED", "delivery_uncertain")))


def occluded(role: str, name: str | None) -> Envelope:
    occluder = {"role": role, "bounds": {"x": 0.0, "y": 0.0, "width": 1280.0, "height": 800.0}}
    if name:
        occluder["name"] = name
    checks = [{"check": "visible", "status": "pass"},
              {"check": "receives_events", "status": "fail", "reason": f"occluded by {role}", "occluder": occluder}]
    error = {"code": "ACTION_FAILED", "message": f"Target is not actionable: receives_events (occluded by {role})",
             "details": {"actionable": False, "checks": checks},
             "disposition": {"delivery": "not_delivered", "retry": "safe"}}
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=1.0,
                    argv=["agent-desktop"], exit_code=1)


class CoverStateTests(unittest.TestCase):
    """A headed ref click activates the pad, so the cover's focus history moves; its box must not."""

    def test_focus_history_is_not_a_cover_change(self):
        cover = {"address": "0x1", "at": [646, 21], "size": [613, 758], "floating": True, "pinned": False,
                 "mapped": True, "hidden": False, "workspace": {"id": 1, "name": "1"}, "focusHistoryID": 0}
        self.assertFalse(refs.cover_changed(cover, dict(cover, focusHistoryID=1)))
        self.assertTrue(refs.cover_changed(cover, dict(cover, at=[0, 0])))
        self.assertTrue(refs.cover_changed(cover, dict(cover, pinned=True)))
        self.assertTrue(refs.cover_changed(cover, None))


class SessionRefusalTests(unittest.TestCase):
    """A headed ref click on the pad on a hidden special workspace refuses before any activation,
    so the special workspace stays hidden."""

    def run_refusals(self, revealed: str):
        clicks, controls = [], []
        client = {"at": [100, 50], "size": [400, 300], "workspace": {"name": "special:ad"}}
        ctx = mock.Mock(tier=mock.Mock())

        def clicked(_ctx, _env, _log, _button, cause, label):
            clicks.append(cause)
            return []

        with mock.patch.multiple(
                refs, main_instance=mock.Mock(return_value={"signature": "s"}),
                instance_env=mock.Mock(return_value={}), pad_ref=mock.Mock(return_value="@s:button"),
                covered_click=clicked, instance_query=mock.Mock(return_value={}),
                covering_layers=mock.Mock(return_value=["omarchy-bar"]),
                pad_client=mock.Mock(return_value=client),
                instance_control=mock.Mock(side_effect=lambda *args: controls.append(args)),
                shown=mock.Mock(return_value={"Virtual-1": ("1", revealed)}),
                raw=mock.Mock(poll=lambda check, _timeout: check())):
            problems = refs._session_refusals(ctx, mock.Mock(pid=1), client, Path("/dev/null"))
        return clicks, controls, problems

    def test_the_refused_click_leaves_the_special_workspace_hidden(self):
        clicks, controls, problems = self.run_refusals("")
        self.assertEqual(clicks, ["layer_cover"])
        self.assertEqual(controls, [])
        self.assertEqual(problems, [])

    def test_a_click_that_reveals_the_special_workspace_is_a_problem(self):
        _, _, problems = self.run_refusals("special:ad")
        self.assertEqual(len(problems), 1)


class DispatchArgvTests(unittest.TestCase):
    def test_lua_and_legacy_window_dispatchers(self):
        self.assertEqual(refs.window_argv("move", "0x10", True, 4, 5),
                         ["dispatch", 'hl.dsp.window.move({x=4,y=5,window="address:0x10"})'])
        self.assertEqual(refs.window_argv("resize", "0x10", False, 640, 480),
                         ["dispatch", "resizewindowpixel", "exact 640 480,address:0x10"])
        self.assertEqual(refs.window_argv("pin", "0x10", True),
                         ["dispatch", 'hl.dsp.window.pin({action="enable",window="address:0x10"})'])
        self.assertEqual(refs.window_argv("pin", "0x10", False), ["dispatch", "pin", "address:0x10"])


class RegistryTests(unittest.TestCase):
    def test_new_pointer_legs_run_on_hyprland_only(self):
        for leg_id in NEW_LEGS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY, leg_id)
            self.assertIn("applog", leg.oracles, leg_id)
            self.assertIn(leg_id, legs._IMPLEMENTATIONS, leg_id)

    def test_reused_pointer_legs_reach_hyprland_with_the_pad_log(self):
        for leg_id in REUSED_LEGS:
            leg = legs.leg_by_id(leg_id)
            self.assertIn("hyprland", leg.tiers, leg_id)
            self.assertIn("applog", leg.oracles, leg_id)

    def test_ref_drag_stays_off_hyprland(self):
        for leg_id in ("ref_actions.drag", "ref_actions.drag_headless_refusal"):
            self.assertNotIn("hyprland", legs.leg_by_id(leg_id).tiers)


class RunningDragTests(unittest.TestCase):
    def test_the_running_drag_inherits_the_interaction_lease_fd(self):
        read_end, write_end = os.pipe()
        self.addCleanup(os.close, read_end)
        self.addCleanup(os.close, write_end)
        tier = mock.Mock(env={})
        with (mock.patch.dict(os.environ, {pointer.product.LEASE_FD_ENV: str(read_end)}),
              mock.patch.object(pointer.product, "_binary_path", return_value="/bin/agent-desktop"),
              mock.patch.object(pointer.subprocess, "Popen") as popen):
            pointer._running_drag(tier, {}, (1, 2), (3, 4))
        kwargs = popen.call_args.kwargs
        self.assertEqual(kwargs["env"][pointer.product.LEASE_FD_ENV], str(read_end))
        self.assertEqual(tuple(kwargs["pass_fds"]), (read_end,))


class LostClickTests(unittest.TestCase):
    def test_a_lost_click_says_whether_the_pointer_reached_the_pad(self):
        self.assertEqual(pointer.missed_click(LOG[4:]), " (presses elsewhere: [('button', 1), ('button', 2)], area motions: 0)")
        self.assertEqual(pointer.missed_click(LOG[:4]), " (presses elsewhere: [], area motions: 2)")


class GlobalLayoutTests(unittest.TestCase):
    """The layout leg clicks the pad again from the other output and puts the nested cursor back."""

    def run_leg(self):
        reads, sent, controls = [], [], []
        ctx = mock.Mock(tier=mock.Mock(), mode="headed")
        for stage in ("before", "after", "restored"):
            getattr(ctx, stage).side_effect = lambda channel, value, stage=stage: reads.append((stage, channel))
        ctx.product.side_effect = lambda argv, env=None: sent.append(argv) or mock.Mock(ok=True, data={}, argv=argv, raw="")
        cursor = {"x": 5, "y": 6}
        monitors = [{"name": "A", "x": 0, "y": 0}, {"name": "B", "x": 1280, "y": 0}]
        client = {"at": [0, 0], "size": [100, 100]}

        @pointer.contextlib.contextmanager
        def pad(*_):
            yield mock.Mock(pid=1), client, Path("/dev/null")

        def query(_tier, _instance, what):
            return monitors if what == "monitors" else dict(cursor)

        def clicked(_ctx, _env, _log, _client, point, count):
            sent.append(("click", point, count))
            return []

        with mock.patch.multiple(
                pointer, nested=mock.Mock(return_value=({"signature": "s"}, True)),
                instance_env=mock.Mock(return_value={}), pads_left=mock.Mock(return_value=[]),
                pointer_pad=pad, monitor_of=mock.Mock(return_value={"name": "A"}),
                layout=mock.Mock(return_value=[]), moved_output=mock.MagicMock(),
                pad_client=mock.Mock(return_value=client), settled_client=mock.Mock(return_value=client),
                cursor_at=mock.Mock(side_effect=[(50, 75), (1320, 40)]), instance_query=query,
                instance_control=mock.Mock(side_effect=lambda _t, _i, argv: controls.append(argv)),
                click_problems=clicked, wheel_problems=mock.Mock(return_value=[]),
                drag_problems=mock.Mock(return_value=[])):
            pointer.global_layout_coordinates(ctx)
        return reads, sent, controls

    def test_it_clicks_back_from_the_other_output_and_restores_the_cursor(self):
        reads, sent, controls = self.run_leg()
        far = sent.index(["mouse-move", "--xy=1320,40"])
        self.assertEqual(sent[far + 1], ("click", (60.0, 70.0), 1))
        self.assertEqual(controls, [pointer.cursor_argv(5, 6, True)])
        self.assertEqual([stage for stage, channel in reads if channel == "hyprland.cursor"],
                         ["before", "after", "restored"])


if __name__ == "__main__":
    unittest.main()


def unproven(**changes) -> Envelope:
    error = {"code": "ACTION_FAILED",
             "message": "The target is an XWayland window, and Hyprland cannot prove which of its surfaces takes"
                        " the pointer; run the ref action without --headed, or aim with --xy",
             "suggestion": "Run the ref action without --headed, such as click @ref, or aim with coordinates"
                           " (mouse-click --xy)",
             "details": {"kind": "receiver_evidence", "check": "hyprland_stack", "cause": "receiver_unproven",
                         "physical_delivery_started": False},
             "disposition": {"delivery": "not_delivered", "retry": "safe"}}
    for key, value in changes.items():
        if key in ("message", "suggestion", "code", "disposition"):
            error[key] = value
        else:
            error["details"][key] = value
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=1.0,
                    argv=["agent-desktop"], exit_code=1)


class UnprovenReceiverTests(unittest.TestCase):
    """Every headed ref pointer action on an XWayland window refuses as receiver_unproven before any
    event, and the refusal names both ways round it."""

    def test_the_refusal_passes_and_each_missing_part_is_a_problem(self):
        self.assertEqual(refs.unproven_problems("c", unproven()), [])
        for changed in (dict(cause="native_wayland_client"), dict(kind="hyprland_pointer"),
                        dict(physical_delivery_started=True), dict(code="ACTION_NOT_SUPPORTED"),
                        dict(disposition={"delivery": "delivered_unverified", "retry": "unsafe"}),
                        dict(suggestion="Run the ref action without --headed"),
                        dict(message="aim with --xy")):
            self.assertEqual(len(refs.unproven_problems("c", unproven(**changed))), 1, changed)
        landed = Envelope(ok=True, data={}, error=None, raw="{}", elapsed_ms=1.0, argv=[], exit_code=0)
        self.assertTrue(refs.unproven_problems("c", landed))

    def refuse(self, logged: list[int], cursors: list[tuple[float, float]], answer=None, earlier=None,
               ctx=None, active=("0xf", "0xf")) -> list[str]:
        ctx = ctx or mock.Mock(tier=mock.Mock())
        ctx.product.return_value = answer or unproven()
        counts, points, actives = iter(logged), iter(cursors), iter(active)
        seen = {"count": 0}

        def reached():
            seen["count"] = next(counts, seen["count"])
            return seen["count"]

        with mock.patch.multiple(refs, cursor_at=mock.Mock(side_effect=lambda *_: next(points)),
                                 active_address=mock.Mock(side_effect=lambda *_: next(actives)),
                                 raw=mock.Mock(poll=lambda check, _timeout: check())):
            return refs.refused_without_input(ctx, {}, {"signature": "n"}, "pad", [["click", "@s:e1"]], reached,
                                              earlier)

    def test_a_refusal_with_no_input_and_a_still_cursor_passes(self):
        self.assertEqual(self.refuse([0, 0, 0], [(10.0, 10.0), (10.0, 10.0)]), [])

    def test_input_the_app_logged_is_a_problem(self):
        self.assertEqual(self.refuse([0, 1, 1], [(10.0, 10.0), (10.0, 10.0)]),
                         ["pad: headed click: the app logged 1 input events"])

    def test_a_cursor_that_moved_is_a_problem(self):
        self.assertEqual(self.refuse([0, 0, 0], [(10.0, 10.0), (320.0, 240.0)]),
                         ["pad: headed click: the cursor moved from (10.0, 10.0) to (320.0, 240.0)"])

    def test_an_activated_target_is_a_problem(self):
        self.assertEqual(self.refuse([0, 0, 0], [(10.0, 10.0)] * 2, active=("0xc0", "0xpad")),
                         ["pad: headed click: the active window changed from 0xc0 to 0xpad"])

    def test_an_activation_refusal_is_a_problem(self):
        activation = unproven(cause="activation_refused", kind=None, activation_sent=True)
        self.assertTrue(self.refuse([0, 0, 0], [(10.0, 10.0)] * 2, activation))

    def test_a_native_cause_is_a_problem(self):
        self.assertEqual(len(self.refuse([0, 0, 0], [(1.0, 1.0)] * 2, unproven(cause="native_wayland_client"))), 1)

    def occluded(self, code="ACTION_FAILED", delivery="not_delivered", **details) -> Envelope:
        error = {"code": code, "message": "Target is not actionable: receives_events (occluded by window)",
                 "details": details or {"checks": [{"check": "receives_events", "status": "fail",
                                                    "occluder": {"kind": "window"}}]},
                 "disposition": {"delivery": delivery, "retry": "safe"}}
        return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=1.0,
                        argv=["agent-desktop"], exit_code=1)

    def test_an_earlier_core_refusal_is_only_an_occluded_hit_test_or_a_drag_that_ran_out_its_deadline(self):
        self.assertTrue(refs.earlier_core_refusal(["click", "@s:e1"], self.occluded()))
        lease = dict(kind="interaction_process_lock_timeout", contention_count=0)
        self.assertTrue(refs.earlier_core_refusal(["drag", "--from", "@s:e1"], self.occluded("TIMEOUT", **lease)))
        self.assertIsNone(refs.earlier_core_refusal(["click", "@s:e1"], self.occluded("TIMEOUT", **lease)))
        stack = dict(kind="receiver_evidence", cause="evidence_timed_out", physical_delivery_started=False)
        self.assertTrue(refs.earlier_core_refusal(["drag", "--from-xy", "1,1"], self.occluded("TIMEOUT", **stack)))
        spent = dict(kind="deadline", elapsed_ms=5000, timeout_ms=5000)
        self.assertTrue(refs.earlier_core_refusal(["drag", "--from-xy", "1,1"], self.occluded("TIMEOUT", **spent)))
        early = dict(spent, elapsed_ms=4000)
        self.assertIsNone(refs.earlier_core_refusal(["drag", "--from-xy", "1,1"], self.occluded("TIMEOUT", **early)))
        started = dict(stack, physical_delivery_started=True)
        self.assertIsNone(refs.earlier_core_refusal(["drag", "--from-xy", "1,1"], self.occluded("TIMEOUT", **started)))
        self.assertIsNone(refs.earlier_core_refusal(["click", "@s:e1"], self.occluded(delivery="delivered_unverified")))
        self.assertIsNone(refs.earlier_core_refusal(["click", "@s:e1"], self.occluded(kind="hyprland_pointer")))
        self.assertIsNone(refs.earlier_core_refusal(["click", "@s:e1"], unproven(cause="native_wayland_client")))

    def test_an_accepted_earlier_refusal_is_a_finding_and_still_wants_no_input(self):
        ctx = mock.Mock(tier=mock.Mock())
        ctx.product.return_value = self.occluded()
        self.assertEqual(self.refuse([0, 0, 0], [(1.0, 1.0)] * 2, self.occluded(), refs.earlier_core_refusal, ctx),
                         [])
        ctx.finding.assert_called_once()
        self.assertEqual(self.refuse([0, 1, 1], [(1.0, 1.0)] * 2, self.occluded(), refs.earlier_core_refusal),
                         ["pad: headed click: the app logged 1 input events"])
        self.assertTrue(self.refuse([0, 0, 0], [(1.0, 1.0)] * 2, self.occluded()))

    def test_the_pad_takes_every_ref_pointer_action_and_both_ends_of_a_drag(self):
        actions = refs.pad_actions("@s:button", "@s:pane", {"at": [0, 0], "size": [100, 100]})
        self.assertEqual([refs.action_label(argv) for argv in actions],
                         ["click", "double-click", "triple-click", "right-click", "scroll --direction down --amount 2",
                          "drag --from --to", "drag --from-xy 50,90 --to"])
        self.assertIn(["drag", "--from", "@s:button", "--to", "@s:pane"], actions)

    def test_the_fixture_takes_every_button_action_and_each_drag_end(self):
        actions = refs.fixture_actions("@s:target", {"at": [0, 0], "size": [100, 100]})
        self.assertEqual([argv[0] for argv in actions],
                         ["click", "double-click", "triple-click", "right-click", "drag", "drag"])
        self.assertEqual(actions[4], ["drag", "--from", "@s:target", "--to-xy", "90,95"])
        self.assertEqual(actions[5], ["drag", "--from-xy", "90,95", "--to", "@s:target"])

    def test_the_overlay_and_popup_fixtures_run_on_xwayland(self):
        opened, labels = [], []

        @refs.contextlib.contextmanager
        def started(_tier, _instance, toolkit, case):
            opened.append((toolkit, case))
            yield mock.Mock(pid=7), {"at": [0, 0], "size": [100, 100]}, Path("/dev/null")

        def refuse(_ctx, _env, _instance, label, argvs, _reached, earlier=None):
            labels.append((label, [argv[0] for argv in argvs], earlier))
            return []

        with mock.patch.multiple(refs, receiver_fixture=started, fixture_ref=mock.Mock(return_value="@s:t"),
                                 refused_without_input=refuse, fixture_events=mock.Mock(return_value=[])):
            self.assertEqual(refs.fixture_refusals(mock.Mock(tier=mock.Mock()), {}, {"signature": "n"}), [])
        self.assertEqual(opened, [("gtk4", "overlay"), ("gtk4", "popup")])
        self.assertEqual([label for label, _, _ in labels], ["gtk4 overlay fixture", "gtk4 popup fixture"])
        self.assertEqual([earlier for _, _, earlier in labels], [None, refs.earlier_core_refusal])

    def test_the_receiver_leg_refuses_every_pad_action_then_the_covers_then_the_fixtures(self):
        order = []

        @refs.contextlib.contextmanager
        def opened(*_):
            yield mock.Mock(pid=1), {"at": [0, 0], "size": [100, 100]}, Path("/dev/null")

        @refs.contextlib.contextmanager
        def cover(*_):
            yield {}

        def ref_of(_ctx, _env, _owned, target=pad.PAD_BUTTON):
            return {pad.PAD_BUTTON: "@s:button", pad.PAD_SCROLL: "@s:pane"}[target]

        def refuse(_ctx, _env, _instance, label, argvs, _reached):
            order.append((label, [argv[0] for argv in argvs]))
            return []

        ctx = mock.Mock(tier=mock.Mock(), mode="headed")
        with mock.patch.multiple(
                refs, nested=mock.Mock(return_value=({"signature": "s"}, True)),
                instance_env=mock.Mock(return_value={}), pads_left=mock.Mock(return_value=[]),
                pointer_pad=opened, pad_ref=ref_of, clients_summary=mock.Mock(return_value=[]),
                refused_without_input=refuse, layer_cover=cover, window_cover=cover,
                covered_click=mock.Mock(side_effect=lambda *args: order.append(args[4]) or []),
                fixture_refusals=mock.Mock(side_effect=lambda *_: order.append("fixtures") or []),
                active_address=mock.Mock(return_value="0xcover"), events=mock.Mock(return_value=[])):
            refs.ref_receiver_evidence(ctx)
        gestures = ["click", "double-click", "triple-click", "right-click", "scroll", "drag", "drag"]
        self.assertEqual(order, [("pad", gestures), "layer_cover", ("floating cover", gestures),
                                 ("pinned cover", gestures), "fixtures"])

    def test_the_headed_ref_gestures_want_the_refusal(self):
        for verb, argv in (("double-click", ["double-click", "@s:e1"]),
                           ("scroll", ["scroll", "@s:e1", "--direction", "down", "--amount", "2"])):
            refusal = mock.Mock(return_value=[])

            @refs.contextlib.contextmanager
            def opened(*_):
                yield mock.Mock(pid=1), {}, Path("/dev/null")

            with mock.patch.multiple(refs, nested=mock.Mock(return_value=({"signature": "s"}, True)),
                                     instance_env=mock.Mock(return_value={}), pads_left=mock.Mock(return_value=[]),
                                     pointer_pad=opened, pad_ref=mock.Mock(return_value="@s:e1"),
                                     refused_without_input=refusal, events=mock.Mock(return_value=[])):
                refs.ref_gesture(mock.Mock(tier=mock.Mock(), mode="headed"), verb)
            self.assertEqual(refusal.call_args.args[4], [argv])


class ReceiverFixtureTests(unittest.TestCase):
    def test_the_fixture_log_keeps_only_its_json_lines(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "fixture.log"
            log.write_text('(python3:12): Gtk-WARNING **: no theme\n{"ev": "mapped", "pid": 9}\n{"ev": "clicked",'
                           ' "widget": "Probe Target"}\n{"ev": \n')
            self.assertEqual([item["ev"] for item in fixture.fixture_events(log)], ["mapped", "clicked"])
            self.assertEqual(fixture.clicked(fixture.fixture_events(log)), ["Probe Target"])
            self.assertEqual(fixture._mapped_pid(log), 9)

    def test_the_fixture_toplevel_is_the_window_with_its_title(self):
        clients = [{"pid": 9, "title": fixture.TITLE, "mapped": True, "address": "0x1"},
                   {"pid": 9, "title": "", "mapped": True, "address": "0x2"},
                   {"pid": 8, "title": fixture.TITLE, "mapped": True, "address": "0x3"}]
        with mock.patch.object(fixture, "instance_query", return_value=clients):
            self.assertEqual(fixture.fixture_client(None, {}, 9)["address"], "0x1")
            self.assertIsNone(fixture.fixture_client(None, {}, 7))


class CoveredClickTests(unittest.TestCase):
    """A refused click sends nothing, but an XWayland pad that appears under the still cursor logs
    the enter motion X sends with it."""

    def covered(self, logged: list[dict]) -> list[str]:
        ctx = mock.Mock()
        with mock.patch.multiple(refs, events=mock.Mock(return_value=[]), unproven_problems=mock.Mock(return_value=[]),
                                 settled=mock.Mock(return_value=logged)):
            return refs.covered_click(ctx, {}, Path("/dev/null"), "@s:button", "layer_cover", "shell layer")

    def test_enter_motion_under_a_cover_is_not_a_delivered_event(self):
        self.assertEqual(self.covered([{"ev": "motion", "widget": "area", "wx": 628, "wy": 362, "t": 1.0}]), [])

    def test_a_press_under_a_cover_is_a_problem_that_says_what_the_pad_saw(self):
        logged = [LOG[0], LOG[4]]
        self.assertEqual(self.covered(logged), ["shell layer: the pad logged an event under the cover"
                                                " (pad saw: motion/area 1, press/button 1)"])


class WheelNotchTests(unittest.TestCase):

    def smooth(self, *dys: float) -> list[dict]:
        return [{"ev": "scroll", "widget": "scroll", "direction": "smooth", "dx": 0.0, "dy": dy, "t": 1.0}
                for dy in dys]

    def test_wheel_notches_name_a_smooth_event_off_one_notch_by_its_deltas(self):
        found = self.smooth(1.0, -1.0, 2.0, 0.0) + [{"ev": "scroll", "widget": "scroll", "direction": "smooth",
                                                     "dx": 1.0, "dy": 0.0, "t": 1.0}]
        self.assertEqual(pad.wheel_notches(found, "scroll"),
                         ["down", "up", "smooth dx=0.0 dy=2.0", "smooth dx=0.0 dy=0.0", "right"])


class MovedOutputTests(unittest.TestCase):
    def test_an_output_that_never_moves_is_reported_with_the_outputs_seen(self):
        seen = [{"name": "WAYLAND-1", "x": 0, "y": 0, "scale": 1.0, "width": 1280, "height": 800}]
        with mock.patch.object(pointer, "instance_query", return_value=seen), \
                mock.patch.object(pointer.hyprland, "control"), mock.patch.object(pointer, "shared"), \
                mock.patch.object(pointer.raw, "poll", side_effect=[False, True]):
            with self.assertRaises(Exception) as caught:
                with pointer.moved_output(None, {"signature": "s"}, False, "WAYLAND-1", (-1024, -640), 1.25):
                    pass
        self.assertIn("WAYLAND-1 0,0 scale 1 1280x800", str(caught.exception))

    def test_an_output_that_never_moves_is_reported_with_the_reply_and_the_dialect(self):
        seen = [{"name": "WAYLAND-1", "x": 0, "y": 0, "scale": 1.0, "width": 1280, "height": 800}]
        with mock.patch.object(pointer, "instance_query", return_value=seen), \
                mock.patch.object(pointer.hyprland, "control", return_value="invalid monitor rule"), \
                mock.patch.object(pointer, "shared"), mock.patch.object(pointer.raw, "poll", side_effect=[False, True]):
            with self.assertRaises(Exception) as caught:
                with pointer.moved_output(None, {"signature": "s"}, False, "WAYLAND-1", (-1024, -640), 1.25):
                    pass
        self.assertIn("legacy config replied 'invalid monitor rule' to keyword monitor "
                      "WAYLAND-1,1280x800@60,-1024x-640,1.25", str(caught.exception))


class PadBackendTests(unittest.TestCase):
    def test_the_x_display_is_the_lock_that_names_the_compositor_or_its_child(self):
        with tempfile.TemporaryDirectory() as work:
            locks, proc = Path(work) / "tmp", Path(work) / "proc"
            locks.mkdir()
            (proc / "71").mkdir(parents=True)
            (proc / "71" / "stat").write_text("71 (Xwayland) S 50 71 71 0 -1 0 0 0")
            (locks / ".X0-lock").write_text("        12\n")
            self.assertEqual(pad.x_display({"pid": 50}, locks, proc), "")
            (locks / ".X3-lock").write_text("        71\n")
            self.assertEqual(pad.x_display({"pid": 50}, locks, proc), ":3")
            self.assertEqual(pad.x_display({"pid": 71}, locks, proc), ":3")
            (locks / ".X3-lock").write_text("garbage")
            self.assertEqual(pad.x_display({"pid": 50}, locks, proc), "")

    def test_the_pad_must_report_the_backend_the_leg_asked_for(self):
        self.assertIsNone(pad.backend_problem({"xwayland": True}, pad.XWAYLAND))
        self.assertIsNone(pad.backend_problem({"xwayland": False}, pad.NATIVE))
        self.assertIn("expected True", pad.backend_problem({"xwayland": False}, pad.XWAYLAND))
        self.assertIn("expected False", pad.backend_problem({"xwayland": True}, pad.NATIVE))
        self.assertIn("None", pad.backend_problem({}, pad.XWAYLAND))

    def test_an_xwayland_pad_launches_on_the_instance_x_display(self):
        tier = mock.Mock(env={"DISPLAY": ":0", "AT_SPI_BUS_ADDRESS": "x", "HYPRLAND_INSTANCE_SIGNATURE": "main"})
        instance = {"pid": 50, "wayland": "wayland-9", "signature": "nested"}
        with mock.patch.object(pad, "x_display", return_value=":3"):
            env = pad.pad_env(tier, instance, pad.XWAYLAND)
        self.assertEqual((env["GDK_BACKEND"], env["DISPLAY"], env["WAYLAND_DISPLAY"]), ("x11", ":3", "wayland-9"))
        self.assertNotIn("AT_SPI_BUS_ADDRESS", env)
        self.assertNotIn("HYPRLAND_INSTANCE_SIGNATURE", env)
        self.assertEqual(pad.pad_env(tier, instance, pad.NATIVE)["GDK_BACKEND"], "wayland")
        with mock.patch.object(pad, "x_display", return_value=""), \
                mock.patch.object(pad.raw, "poll", lambda check, _seconds: check()):
            with self.assertRaises(AssertionError):
                pad.pad_env(tier, instance, pad.XWAYLAND)

    def test_the_pad_app_keeps_a_backend_the_launcher_set(self):
        source = (ROOT / "apps" / "pointer_pad.py").read_text()
        self.assertIn('os.environ.setdefault("GDK_BACKEND", "wayland")', source)
        self.assertLess(source.index("GDK_BACKEND"), source.index("import gi"))


class PadLegBackendTests(unittest.TestCase):
    def run_leg(self, leg, mode="headed", **patches):
        backends = []

        @refs.contextlib.contextmanager
        def opened(_tier, _instance, _label, backend=pad.NATIVE):
            backends.append(backend)
            yield mock.Mock(pid=1), {"at": [0, 0], "size": [100, 100]}, Path("/dev/null")

        @refs.contextlib.contextmanager
        def cover(*_):
            yield {}

        ctx = mock.Mock(tier=mock.Mock(), mode=mode)
        base = dict(nested=mock.Mock(return_value=({"signature": "s"}, True)),
                    instance_env=mock.Mock(return_value={}), pads_left=mock.Mock(return_value=[]),
                    pointer_pad=opened, pad_ref=mock.Mock(return_value="@s:e1"),
                    clients_summary=mock.Mock(return_value=[]), refused_without_input=mock.Mock(return_value=[]),
                    fixture_refusals=mock.Mock(return_value=[]), layer_cover=cover, window_cover=cover,
                    covered_click=mock.Mock(return_value=[]), events=mock.Mock(return_value=[]),
                    settled=mock.Mock(return_value=[]), active_address=mock.Mock(return_value="0xcover"))
        base.update(patches)
        with mock.patch.multiple(refs, **base):
            leg(ctx)
        return backends, ctx

    def test_the_ref_legs_drive_an_xwayland_pad(self):
        for leg in (refs.ref_receiver_evidence, lambda ctx: refs.ref_gesture(ctx, "double-click"),
                    lambda ctx: refs.ref_gesture(ctx, "scroll")):
            backends, _ = self.run_leg(leg)
            self.assertEqual(backends, [pad.XWAYLAND])
        backends, _ = self.run_leg(lambda ctx: refs.ref_gesture(ctx, "triple-click"), mode="headless",
                                   refusal_problems=mock.Mock(return_value=[]))
        self.assertEqual(backends, [pad.XWAYLAND])

    def test_the_cover_refusals_drive_xwayland_pads(self):
        backends, _ = self.run_leg(refs.cover_or_missing_evidence_refused,
                                   bystander=mock.Mock(), cursor=mock.Mock(), restore_desktop=mock.Mock(),
                                   main_instance=mock.Mock(return_value={"signature": "m"}),
                                   _session_refusals=mock.Mock(return_value=[]),
                                   _cover_client=mock.Mock(return_value={}),
                                   cover_changed=mock.Mock(return_value=False))
        self.assertEqual(backends, [pad.XWAYLAND, pad.XWAYLAND])

    def native(self, answers, logged=()):
        product = mock.Mock(side_effect=answers)

        def leg(ctx):
            ctx.product = product
            refs.native_wayland_ref_refused(ctx)
        backends, _ = self.run_leg(leg, settled=mock.Mock(return_value=list(logged)))
        return backends, product

    def test_the_native_leg_wants_every_ref_action_refused_on_a_native_pad(self):
        good = refused("ACTION_FAILED", cause="native_wayland_client", physical_delivery_started=False)
        backends, product = self.native([good] * 4)
        self.assertEqual(backends, [pad.NATIVE])
        verbs = [call.args[0][0] for call in product.call_args_list]
        self.assertEqual(verbs, ["click", "double-click", "triple-click", "scroll"])

    def test_the_native_leg_fails_on_another_cause_a_delivery_or_a_pad_event(self):
        good = refused("ACTION_FAILED", cause="native_wayland_client", physical_delivery_started=False)
        cover = refused("ACTION_FAILED", cause="window_cover", physical_delivery_started=False)
        landed = Envelope(ok=True, data={}, error=None, raw="{}", elapsed_ms=1.0, argv=[], exit_code=0)
        for answers in ([cover] + [good] * 3, [landed] + [good] * 3):
            with self.assertRaises(AssertionError):
                self.native(answers)
        with self.assertRaises(AssertionError):
            self.native([good] * 4, logged=[{"ev": "press", "widget": "button", "n": 1}])

    def test_native_refusal_problems_name_each_missing_part(self):
        good = refused("ACTION_FAILED", cause="native_wayland_client", physical_delivery_started=False)
        self.assertEqual(refs.native_refusal_problems("c", good), [])
        self.assertEqual(len(refs.native_refusal_problems(
            "c", refused("ACTION_FAILED", "delivered_unverified", cause="native_wayland_client",
                         physical_delivery_started=False))), 1)
        self.assertEqual(len(refs.native_refusal_problems(
            "c", refused("ACTION_NOT_SUPPORTED", cause="native_wayland_client", physical_delivery_started=False))), 1)
        self.assertEqual(len(refs.native_refusal_problems("c", refused("ACTION_FAILED", cause="native_wayland_client"))), 1)


if __name__ == "__main__":
    unittest.main()
