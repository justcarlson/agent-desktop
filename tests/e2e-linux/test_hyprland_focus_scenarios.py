from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner.product import Envelope
from scenarios import hyprland_desktop, hyprland_focus, hyprland_identity, hyprland_windows

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
WINDOW = "w-hypr-666-345-18000006"


def focused(*, activated=True, proof="compositor_only", effects=(), window=WINDOW) -> Envelope:
    activation = {"activated": activated, "proof": proof}
    if effects:
        activation["side_effects"] = list(effects)
    data = {"focused": {"id": window, "is_focused": True}, "activation": activation}
    return Envelope(ok=True, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)


def refused(code: str, **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details, "disposition": NOT_DELIVERED}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


SWITCH = {"kind": "workspace_switched", "monitor": "Virtual-1", "from": "1", "to": "7"}
MOVED = {"kind": "cursor_moved", "from": {"x": 322.0, "y": 413.0}, "to": {"x": 957.0, "y": 604.0}}


class ActivationContractTests(unittest.TestCase):
    def check(self, envelope, **overrides):
        expected = dict(window_id=WINDOW, activated=True, proof="compositor_only", effects=[SWITCH],
                        cursor_before={"x": 322, "y": 413}, cursor_after={"x": 957, "y": 604})
        expected.update(overrides)
        return hyprland_focus.activation_problems("x", envelope, **expected)

    def test_a_reported_activation_that_matches_the_compositor_passes(self):
        self.assertEqual(self.check(focused(effects=[SWITCH, MOVED])), [])

    def test_a_proven_focus_that_sent_nothing_passes_without_side_effects(self):
        self.assertEqual(hyprland_focus.activation_problems(
            "x", focused(activated=False), window_id=WINDOW, activated=False, proof="compositor_only"), [])

    def test_each_disagreement_with_the_compositor_is_reported(self):
        cases = {
            "refused": refused("ACTION_FAILED", cause="activation_refused"),
            "window": focused(effects=[SWITCH, MOVED], window="w-hypr-666-345-1"),
            "activated": focused(activated=False, effects=[SWITCH, MOVED]),
            "proof": focused(proof="compositor_and_accessibility", effects=[SWITCH, MOVED]),
            "missing effect": focused(effects=[MOVED]),
            "extra effect": focused(effects=[SWITCH, {"kind": "special_workspace_revealed",
                                                      "monitor": "Virtual-1", "workspace": "special:a"},
                                             MOVED]),
            "unreported cursor": focused(effects=[SWITCH]),
            "wrong cursor": focused(effects=[SWITCH, {**MOVED, "to": {"x": 1.0, "y": 1.0}}]),
        }
        for name, envelope in cases.items():
            with self.subTest(name=name):
                self.assertTrue(self.check(envelope))

    def test_a_cursor_move_the_compositor_did_not_see_is_reported(self):
        problems = self.check(focused(effects=[SWITCH, MOVED]), cursor_after={"x": 322, "y": 413})
        self.assertTrue(problems)

    def test_side_effects_without_an_activation_are_reported(self):
        problems = hyprland_focus.activation_problems(
            "x", focused(activated=False, effects=[SWITCH]), window_id=WINDOW, activated=False,
            proof="compositor_only", effects=[SWITCH])
        self.assertTrue(any("nothing was activated" in problem for problem in problems))


class RefusalDetailTests(unittest.TestCase):
    def test_a_refusal_names_its_cause_and_details(self):
        envelope = refused("ACTION_FAILED", cause="special_workspace_open", physical_delivery_started=False,
                           workspace="special:ad_e2e", monitor="Virtual-1")
        self.assertEqual(hyprland_focus.refusal("x", envelope, "ACTION_FAILED", "special_workspace_open",
                                                workspace="special:ad_e2e", monitor="Virtual-1"), [])
        self.assertTrue(hyprland_focus.refusal("x", envelope, "ACTION_FAILED", "special_workspace_open",
                                               workspace="special:other"))
        self.assertTrue(hyprland_focus.refusal("x", envelope, "POLICY_DENIED", "special_workspace_open"))


UNVERIFIED = {"delivery": "delivered_unverified", "retry": "unsafe"}
REVEALED = {"kind": "special_workspace_revealed", "monitor": "Virtual-1", "workspace": "special:ad_e2e_b"}


def hidden(**changes) -> Envelope:
    details = {"cause": "window_hidden", "activation_sent": True, "in_view": False,
               "compositor_active": True, "physical_delivery_started": False, **changes}
    error = {"code": "ACTION_FAILED", "message": "hidden", "details": details, "disposition": UNVERIFIED}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


class HiddenActivationTests(unittest.TestCase):
    def check(self, envelope, stale=True):
        return hyprland_focus.hidden_activation_problems("x", envelope, stale=stale, monitor="Virtual-1")

    def test_a_reveal_or_a_reported_hidden_refusal_passes(self):
        self.assertEqual(self.check(focused(effects=[REVEALED, MOVED]), stale=False), [])
        self.assertEqual(self.check(focused(effects=[REVEALED])), [])
        self.assertEqual(self.check(hidden()), [])

    def test_a_claim_of_focus_or_an_unexplained_refusal_is_reported(self):
        cases = {
            "already focused": (focused(activated=False), True),
            "no reveal": (focused(effects=[MOVED]), True),
            "not stale": (hidden(), False),
            "not sent": (hidden(activation_sent=False), True),
            "in view": (hidden(in_view=True), True),
            "other cause": (hidden(cause="activation_refused"), True),
            "safe retry": (refused("ACTION_FAILED", cause="window_hidden", activation_sent=True, in_view=False,
                                   compositor_active=True, physical_delivery_started=False), True),
        }
        for name, (envelope, stale) in cases.items():
            with self.subTest(name=name):
                self.assertTrue(self.check(envelope, stale))


CLIENT = {"class": "ad-e2e-listed", "pid": 3109, "title": "omarchy@omarchy-test:~", "mapped": True,
          "hidden": False, "at": [647, 38], "size": [621, 368]}
ENTRY = {"id": WINDOW, "accessible": False, "pid": 3109, "title": "omarchy@omarchy-test:~",
         "app_name": "ad-e2e-listed", "visible": True, "is_focused": False, "process_instance": "ff:868",
         "bounds": {"x": 647.0, "y": 38.0, "width": 621.0, "height": 368.0}}


class CompositorEntryTests(unittest.TestCase):
    def test_an_entry_that_matches_its_toplevel_passes(self):
        self.assertEqual(hyprland_identity.compositor_entry_problems(ENTRY, CLIENT, "w-hypr-666-345-"), [])

    def test_each_field_that_disagrees_with_the_toplevel_is_reported(self):
        cases = {
            "accessible": {"accessible": True}, "pid": {"pid": 1}, "title": {"title": "other"},
            "class": {"app_name": "foot"}, "visible": {"visible": False},
            "bounds": {"bounds": {"x": 0.0, "y": 38.0, "width": 621.0, "height": 368.0}},
            "instance": {"process_instance": ""}, "minimized": {"minimized": False},
            "ref": {"ref_id": "@s1:e1"}, "id": {"id": "w-hypr-1-1-18000006"},
        }
        for name, change in cases.items():
            with self.subTest(name=name):
                self.assertTrue(hyprland_identity.compositor_entry_problems(
                    {**ENTRY, **change}, CLIENT, "w-hypr-666-345-"))


class CompositorControlTests(unittest.TestCase):
    def test_lua_and_classic_controls_name_one_window_by_address(self):
        address = "0x5613e93f4270"
        self.assertEqual(hyprland_windows.focus_argv(address, True),
                         ["dispatch", 'hl.dsp.focus({window="address:0x5613e93f4270"})'])
        self.assertEqual(hyprland_windows.focus_argv(address, False),
                         ["dispatch", "focuswindow", "address:0x5613e93f4270"])
        self.assertEqual(hyprland_windows.move_argv(address, "special:ad_e2e_b", True),
                         ["dispatch", 'hl.dsp.window.move({workspace="special:ad_e2e_b",follow=false,'
                                      'window="address:0x5613e93f4270"})'])
        self.assertEqual(hyprland_windows.move_argv(address, "7", False),
                         ["dispatch", "movetoworkspacesilent", "7,address:0x5613e93f4270"])

    def test_special_workspace_and_cursor_controls(self):
        self.assertEqual(hyprland_windows.toggle_argv("special:ad_e2e", True),
                         ["dispatch", 'hl.dsp.workspace.toggle_special("ad_e2e")'])
        self.assertEqual(hyprland_windows.toggle_argv("special:ad_e2e", False),
                         ["dispatch", "togglespecialworkspace", "ad_e2e"])
        self.assertEqual(hyprland_windows.cursor_argv(640, 400, True),
                         ["dispatch", "hl.dsp.cursor.move({x=640,y=400})"])
        self.assertEqual(hyprland_windows.cursor_argv(640, 400, False),
                         ["dispatch", "movecursor", "640", "400"])

    def test_a_control_refuses_text_that_could_break_out_of_its_string(self):
        for build in (lambda: hyprland_windows.focus_argv('0x1"}) hl.dsp.exit() --', True),
                      lambda: hyprland_windows.move_argv("0x1", 'a"', False),
                      lambda: hyprland_windows.toggle_argv("special:a;b", False)):
            with self.subTest():
                with self.assertRaises(ValueError):
                    build()

    def test_a_terminal_runs_a_command_that_cannot_retitle_it(self):
        self.assertEqual(hyprland_windows.terminal_argv("ad-e2e-listed"),
                         ["foot", "--app-id", "ad-e2e-listed", "--title", "ad-e2e-listed", "sleep", "infinity"])

    def test_a_foreign_id_keeps_the_toplevel_and_names_a_compositor_that_never_ran(self):
        self.assertEqual(hyprland_windows.foreign_id(WINDOW), "w-hypr-1-1-18000006")


class FrameActiveTests(unittest.TestCase):
    def frame(self, path, states):
        return SimpleNamespace(bus=":1.5", path=path, states=frozenset(states))

    def test_the_app_frame_is_read_as_active_only_when_its_state_says_so(self):
        app = SimpleNamespace(owner_names=[":1.5"], windows=["w-1.5-a"])
        for states, expected in ((("ACTIVE", "SHOWING"), True), (("SHOWING",), False)):
            root = SimpleNamespace(children=lambda states=states: [self.frame("/a", states),
                                                                  self.frame("/b", ("ACTIVE",))])
            with self.subTest(states=states), mock.patch.object(hyprland_windows.raw, "app_root",
                                                                return_value=root):
                self.assertIs(hyprland_windows.frame_active(None, app), expected)


class HeadedRefusalTests(unittest.TestCase):
    def test_headed_focus_window_is_no_longer_asserted_as_a_wayland_refusal(self):
        calls = []

        def product(argv, **_):
            calls.append(argv[0])
            return refused("ACTION_NOT_SUPPORTED", cause="wayland_session", physical_delivery_started=False)

        ctx = SimpleNamespace(product=product)
        app = SimpleNamespace(windows=["w-1.5-a"])
        self.assertEqual(hyprland_desktop._headed_refusals(ctx, app, "@s1:e1"), [])
        self.assertNotIn("focus-window", calls)


if __name__ == "__main__":
    unittest.main()
