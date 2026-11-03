from __future__ import annotations

import inspect
import json
import os
import stat
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import pixels
from runner import legs
from runner.product import Envelope
from scenarios import hyprland_cursor as cursor
from scenarios import hyprland_cursor_hide as hide

HEADED_STOPPED = "cursor.travel_failure_proceeds_hide_failure_refuses"
HEADLESS_STOPPED = "cursor.travel_failure_proceeds_headless"
LEG_IDS = ("cursor.hide_ack_suppression_until_show", HEADED_STOPPED, HEADLESS_STOPPED,
           "pointer.ref_receiver_evidence_under_cursor", "pointer.cursor_pass_through")
PAD_MOTION = {"ev": "motion", "widget": "area", "wx": 10.0, "wy": 10.0, "t": 1.0}
PAD_PRESS = {"ev": "press", "widget": "area", "n": 1, "wx": 10.0, "wy": 10.0, "t": 1.0}


def refused(kind: str, delivery: str = "not_delivered", retry: str = "safe", started: bool = False) -> Envelope:
    error = {"code": "TIMEOUT", "details": {"kind": kind, "physical_delivery_started": started},
             "disposition": {"delivery": delivery, "retry": retry}}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def shell_refusal(delivery: str = "not_delivered", retry: str = "safe") -> Envelope:
    check = {"check": "receives_events", "status": "fail", "occluder": {"name": "omarchy-notifications"}}
    error = {"code": "ACTION_FAILED", "details": {"checks": [check]},
             "disposition": {"delivery": delivery, "retry": retry}}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def native_refusal(retry: str = "safe", started: bool = False, cause: str = "native_wayland_client",
                   delivery: str = "not_delivered") -> Envelope:
    error = {"code": "ACTION_FAILED", "details": {"kind": "receiver_evidence", "cause": cause,
                                                  "physical_delivery_started": started},
             "disposition": {"delivery": delivery, "retry": retry}}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


class Findings:
    def __init__(self):
        self.found = []

    def finding(self, text: str) -> None:
        self.found.append(text)


class Reads:
    def __init__(self):
        self.restored_reads = {}
        self.found = []
        self.tier = None

    def restored(self, oracle: str, value) -> None:
        self.restored_reads.setdefault(oracle, []).append(value)

    def finding(self, text: str) -> None:
        self.found.append(text)


NESTED_SPACE = "special:ad_e2e_nested"
GHOST = [{"monitor": "WAYLAND-1", "level": "3", "pid": -1}]


APP_WINDOW, NESTED_WINDOW = "0xapp", "0xnested"


class Outer:
    """The outer session: it shows the nested window's special workspace on a toggle it obeys, and only a
    shown nested window gets the frames that let the nested compositor drop a closed layer."""

    def __init__(self, obeys: bool = True, ghost_frames: int = 3, shown: set[str] | None = None,
                 raises_once: bool = False):
        self.obeys, self.ghost_frames, self.raises_once = obeys, ghost_frames, raises_once
        self.shown, self.toggles, self.framed = set(shown or ()), [], 0
        self.focused, self.focuses = APP_WINDOW, []

    def toggle(self, tier, name: str) -> None:
        """One monitor: showing a special workspace replaces the one open there and focuses its window."""
        self.toggles.append(name)
        if self.obeys:
            self.shown = set() if name in self.shown else {name}
            self.focused = NESTED_WINDOW if name == NESTED_SPACE and self.shown else ""
        if self.raises_once:
            self.raises_once = False
            raise RuntimeError("hyprctl timed out after the dispatch took effect")

    def focus(self, tier, address: str) -> None:
        self.focuses.append(address)
        self.focused = address

    def layers_of(self, runtime, instance):
        if NESTED_SPACE in self.shown:
            self.framed += 1
        return GHOST if self.framed < self.ghost_frames else []


def arrow_in(image: pixels.Image, box: tuple[int, int, int, int], color=(66, 153, 255)) -> pixels.Image:
    """`image` with an arrow-like wedge of `color` from the box's top left."""
    x, y, width, height = box
    rgb = bytearray(image.rgb)
    for row in range(3, height):
        for column in range(1, max(1, row * 2 // 3)):
            offset = ((y + row) * image.width + x + column) * 3
            rgb[offset:offset + 3] = bytes(color)
    return pixels.Image(width=image.width, height=image.height, rgb=bytes(rgb))


def ended_reads(outer: Outer) -> tuple[list[str], list]:
    """The problems and the restored layer reads after session end, against the fake outer session."""
    ctx = Reads()
    owner = {"pid": 5, "start_time": 1}
    window = {"workspace": {"id": -98, "name": NESTED_SPACE}}
    with mock.patch.object(cursor, "layers_of", outer.layers_of), \
            mock.patch.object(hide, "layers_of", outer.layers_of), \
            mock.patch.object(cursor, "alive", lambda owner: False), \
            mock.patch.object(cursor, "record_of", lambda runtime, session: None), \
            mock.patch.object(cursor, "SETTLE_S", 0.2), \
            mock.patch.object(hide, "SHOW_SETTLE_S", 0.2), \
            mock.patch.object(hide, "outer_window", lambda tier, instance: window, create=True), \
            mock.patch.object(hide, "outer_shown", lambda tier: set(outer.shown), create=True), \
            mock.patch.object(hide, "toggle_outer", outer.toggle, create=True), \
            mock.patch.object(hide, "outer_focus", lambda tier: outer.focused, create=True), \
            mock.patch.object(hide, "focus_outer", outer.focus, create=True):
        problems = hide.ended_layers(ctx, Path("/nonexistent"), {"signature": "s", "pid": 1}, "run-1", owner)
    return problems, ctx.restored_reads.get("hyprland.layers", [])


def dead_click(envelope: Envelope, shown: str, expected: str = "7") -> tuple[list[str], list[str]]:
    ctx = Findings()
    with mock.patch.object(hide, "display", lambda tree: tree["shown"]):
        problems = hide._dead_click_problems(ctx, envelope, ["omarchy-notifications"], {"shown": shown}, expected)
    return problems, ctx.found


class CursorHideLegTests(unittest.TestCase):
    def test_every_leg_is_registered_on_hyprland_with_an_implementation(self):
        for leg_id in LEG_IDS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
            self.assertIn("hyprland", leg.oracles)
            self.assertIn(leg_id, legs.implementations())

    def test_a_hide_refusal_must_be_undelivered_and_safe(self):
        self.assertEqual(hide.hide_refusal_problems("x", refused("cursor_hide_timeout")), [])
        self.assertEqual(hide.hide_refusal_problems("x", refused("cursor_hide_failed")), [])
        self.assertEqual(len(hide.hide_refusal_problems("x", refused("receiver_evidence"))), 1)
        self.assertEqual(len(hide.hide_refusal_problems("x", refused("cursor_hide_timeout", retry="unsafe"))), 1)
        self.assertEqual(len(hide.hide_refusal_problems("x", refused("cursor_hide_timeout", started=True))), 1)
        accepted = Envelope(ok=True, data={}, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)
        self.assertEqual(len(hide.hide_refusal_problems("x", accepted)), 2)

    def test_the_registry_is_the_directory_of_the_sessions_record(self):
        with tempfile.TemporaryDirectory() as work:
            runtime = Path(work)
            base = runtime / "agent-desktop" / "cursor" / "key"
            (base / "holders").mkdir(parents=True)
            (base / "r.json").write_text(json.dumps({"session_id": "one", "pid": 5}))
            self.assertEqual(hide.registry_of(runtime, "one"), base)
            with self.assertRaises(AssertionError):
                hide.registry_of(runtime, "two")

    def test_a_harness_holder_is_a_private_live_file_removed_on_exit(self):
        with tempfile.TemporaryDirectory() as work:
            registry = Path(work)
            with hide.harness_holder(registry, hold_ms=5_000) as body:
                found = hide.holders_in(registry)
                self.assertEqual(found, [body])
                self.assertEqual(body["pid"], os.getpid())
                self.assertGreater(body["expiry"], int(time.time() * 1000))
                path = registry / "holders" / f"{body['nonce']}.json"
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
                self.assertEqual(stat.S_IMODE((registry / "holders").stat().st_mode) & 0o077, 0)
            self.assertEqual(hide.holders_in(registry), [])

    def test_the_keep_drawn_flag_is_a_private_file_removed_on_exit(self):
        with tempfile.TemporaryDirectory() as work:
            registry = Path(work)
            with hide.keep_drawn(registry) as flag:
                self.assertTrue(flag.is_file())
                self.assertEqual(flag.name, "keep-drawn-for-test")
                self.assertEqual(stat.S_IMODE(flag.stat().st_mode), 0o600)
            self.assertFalse(flag.exists())

    def test_only_a_cursor_layer_overlapping_the_client_counts(self):
        client = {"at": [100, 100], "size": [200, 100]}
        over = {"namespace": "agent-desktop-cursor", "x": 290, "y": 190, "w": 32, "h": 32}
        apart = {"namespace": "agent-desktop-cursor", "x": 400, "y": 400, "w": 32, "h": 32}
        foreign = {"namespace": "selection", "x": 100, "y": 100, "w": 200, "h": 100}
        self.assertTrue(hide.cursor_layer_over({"DP-1": {"levels": {"3": [over]}}}, client))
        self.assertFalse(hide.cursor_layer_over({"DP-1": {"levels": {"3": [apart, foreign]}}}, client))
        self.assertFalse(hide.cursor_layer_over({}, client))

    def test_dpms_follows_the_config_language(self):
        self.assertEqual(hide.dpms_argv("off", False), ["dispatch", "dpms", "off"])
        self.assertEqual(hide.dpms_argv("on", True)[0], "dispatch")
        self.assertIn("hl.dsp.dpms", hide.dpms_argv("on", True)[1])

    def test_a_dead_click_refused_by_a_prior_shell_cover_passes_only_undelivered_and_unchanged(self):
        problems, found = dead_click(shell_refusal(), "7")
        self.assertEqual(problems, [])
        self.assertEqual(len(found), 1)

    def test_a_dead_click_shell_refusal_that_started_delivery_is_a_problem(self):
        for delivery, retry in (("delivered_unverified", "unsafe"), ("uncertain", "unsafe"), ("not_delivered", "unsafe")):
            problems, found = dead_click(shell_refusal(delivery, retry), "7")
            self.assertEqual(len(problems), 1, (delivery, retry))
            self.assertEqual(found, [])

    def test_a_dead_click_shell_refusal_that_changed_the_display_is_a_problem(self):
        problems, found = dead_click(shell_refusal(), "77")
        self.assertEqual(len(problems), 1)
        self.assertIn("77", problems[0])
        self.assertEqual(found, [])

    def test_a_dead_click_native_wayland_refusal_passes_only_undelivered_and_unchanged(self):
        problems, found = dead_click(native_refusal(), "7")
        self.assertEqual(problems, [])
        self.assertEqual(len(found), 1)
        self.assertIn("native Wayland", found[0])
        self.assertIn("before the hide", found[0])
        self.assertNotIn("passed its hide", found[0])
        self.assertEqual(len(dead_click(native_refusal(retry="unsafe"), "7")[0]), 1)
        self.assertEqual(len(dead_click(native_refusal(), "77")[0]), 1)
        self.assertEqual(len(dead_click(native_refusal(started=True), "7")[0]), 1)

    def test_a_dead_click_receiver_unproven_refusal_passes_only_undelivered_and_unchanged(self):
        problems, found = dead_click(native_refusal(cause="receiver_unproven"), "7")
        self.assertEqual(problems, [])
        self.assertIn("before the hide", found[0])
        self.assertEqual(len(dead_click(native_refusal(cause="receiver_unproven"), "77")[0]), 1)

    def test_the_headed_click_under_a_stopped_renderer_refuses_before_any_hide(self):
        self.assertEqual(hide.pre_hide_refusal_problems("x", native_refusal()), [])
        self.assertEqual(hide.pre_hide_refusal_problems("x", native_refusal(cause="receiver_unproven")), [])
        self.assertNotEqual(hide.pre_hide_refusal_problems("x", refused("cursor_hide_timeout")), [])
        self.assertNotEqual(hide.pre_hide_refusal_problems("x", refused("cursor_hide_failed")), [])
        self.assertNotEqual(hide.pre_hide_refusal_problems("x", native_refusal(cause="occluded")), [])
        self.assertEqual(len(hide.pre_hide_refusal_problems("x", native_refusal(started=True))), 1)
        self.assertEqual(len(hide.pre_hide_refusal_problems("x", native_refusal(retry="unsafe"))), 1)
        self.assertEqual(len(hide.pre_hide_refusal_problems("x", native_refusal(delivery="uncertain"))), 1)
        accepted = Envelope(ok=True, data={}, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)
        self.assertNotEqual(hide.pre_hide_refusal_problems("x", accepted), [])

    def test_the_stopped_renderer_leg_checks_its_headed_click_as_a_pre_hide_refusal(self):
        source = inspect.getsource(hide.travel_failure_proceeds_hide_failure_refuses)
        self.assertIn('pre_hide_refusal_problems("headed click under a stopped renderer", clicked)', source)
        self.assertNotIn('hide_refusal_problems("headed click', source.replace("pre_hide_refusal_problems", ""))

    def test_the_stopped_renderer_halves_are_split_by_what_each_proves(self):
        headed, headless = legs.leg_by_id(HEADED_STOPPED), legs.leg_by_id(HEADLESS_STOPPED)
        self.assertEqual((headed.modes, headed.disposition), (legs.HEADED, "asserted-refusal"))
        self.assertEqual((headless.modes, headless.disposition), (legs.HEADLESS, "pass"))
        self.assertIn("applog", headed.oracles)
        self.assertIn("mouse-click --xy", headed.requirement)
        self.assertIn("mouse-click", headed.reason)
        self.assertNotIn("headless", headed.reason)
        self.assertNotIn("refuse", headless.requirement)

    def test_a_headed_xy_click_under_a_stopped_renderer_must_refuse_at_the_hide_with_no_event(self):
        still = ((5.0, 5.0), (5.0, 5.0))
        self.assertEqual(hide.xy_refusal_problems("x", refused("cursor_hide_timeout"), [], still), [])
        self.assertEqual(hide.xy_refusal_problems("x", refused("cursor_hide_failed"), [], still), [])
        self.assertEqual(len(hide.xy_refusal_problems("x", refused("cursor_hide_timeout"), [PAD_MOTION], still)), 1)
        self.assertEqual(len(hide.xy_refusal_problems("x", refused("cursor_hide_timeout"), [PAD_PRESS], still)), 1)
        moved = ((5.0, 5.0), (6.0, 5.0))
        self.assertEqual(len(hide.xy_refusal_problems("x", refused("cursor_hide_timeout"), [], moved)), 1)
        self.assertEqual(len(hide.xy_refusal_problems("x", refused("cursor_hide_timeout", started=True), [], still)), 1)
        self.assertNotEqual(hide.xy_refusal_problems("x", native_refusal(), [], still), [])
        accepted = Envelope(ok=True, data={}, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)
        self.assertNotEqual(hide.xy_refusal_problems("x", accepted, [PAD_PRESS], moved), [])

    def test_a_headed_xy_refusal_must_say_no_physical_delivery_started(self):
        still = ((5.0, 5.0), (5.0, 5.0))
        unstated = refused("cursor_hide_timeout")
        del unstated.error["details"]["physical_delivery_started"]
        self.assertEqual(len(hide.xy_refusal_problems("x", unstated, [], still)), 1)
        unstated.error["details"]["physical_delivery_started"] = None
        self.assertEqual(len(hide.xy_refusal_problems("x", unstated, [], still)), 1)

    def test_a_recovered_xy_click_counts_a_late_duplicate_press(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "pad.jsonl"
            log.write_text("")

            def write(count: int) -> None:
                staged = log.with_suffix(".new")
                staged.write_text((json.dumps(PAD_PRESS) + "\n") * count)
                os.replace(staged, log)

            def clicked(late: bool):
                def product(argv):
                    write(1)
                    if late:
                        threading.Timer(0.3, write, (2,)).start()
                    return Envelope(ok=True, data={}, error=None, raw="{}", elapsed_ms=5.0, argv=argv, exit_code=0)
                return mock.Mock(product=product)

            pad = hide.PadAim("s", log, (5, 5))
            with mock.patch.object(hide, "apart_from_last_click"):
                self.assertEqual(hide._landed_xy(clicked(False), pad, "x"), [])
                log.write_text("")
                self.assertEqual(len(hide._landed_xy(clicked(True), pad, "x")), 1)

    def test_the_headed_leg_sends_its_xy_click_while_the_renderer_is_stopped(self):
        source = inspect.getsource(hide.travel_failure_proceeds_hide_failure_refuses)
        stopped = source[source.index("with _stopped(owner):"):source.index("_screen_after_resume")]
        self.assertIn("_refused_xy(", stopped)
        self.assertIn("_landed_xy(", source[source.index("_screen_after_resume"):])

    def test_the_pad_legs_under_the_cursor_drive_a_shown_xwayland_pad_on_the_session_instance(self):
        source = inspect.getsource(hide._click_under_cursor)
        self.assertIn("main = main_instance(tier)", source)
        self.assertIn("pointer_pad(tier, main, label, XWAYLAND)", source)
        self.assertIn("app_shown(tier, owned.pid)", source)
        self.assertIn('gone_problems("session end", runtime, main, session, owner)', source)
        self.assertNotIn("nested", source)

    def test_the_arrow_shows_by_its_fill_and_never_by_a_changed_button(self):
        box = (2, 2, 14, 14)
        normal = pixels.solid(24, 24, (222, 222, 222))
        pressed = pixels.solid(24, 24, (190, 190, 190))
        focused = arrow_in(normal, box, (53, 132, 228))
        self.assertFalse(hide.arrow_shown(normal, box))
        self.assertFalse(hide.arrow_shown(pressed, box))
        self.assertFalse(hide.arrow_shown(focused, box))
        self.assertTrue(hide.arrow_shown(arrow_in(normal, box), box))
        self.assertTrue(hide.arrow_shown(arrow_in(pressed, box), box))

    def test_a_pad_leg_reads_restored_layers_after_the_closed_cursor_layer_leaves(self):
        outer = Outer()
        problems, restored = ended_reads(outer)
        self.assertEqual(problems, [])
        self.assertEqual(restored, [[]])

    def test_the_closed_layer_on_the_hidden_nested_output_leaves_only_once_the_outer_session_shows_it(self):
        outer = Outer()
        problems, restored = ended_reads(outer)
        self.assertEqual(problems, [])
        self.assertGreaterEqual(outer.framed, outer.ghost_frames)
        self.assertEqual(outer.toggles, [NESTED_SPACE, NESTED_SPACE])
        self.assertEqual(outer.shown, set())

    def test_a_nested_window_the_outer_session_does_not_show_fails_without_a_second_toggle(self):
        outer = Outer(obeys=False)
        with self.assertRaises(AssertionError) as raised:
            ended_reads(outer)
        self.assertIn("did not show", str(raised.exception))
        self.assertEqual(outer.toggles, [NESTED_SPACE])

    def test_a_special_workspace_the_nested_one_displaced_is_shown_again_with_the_focus_it_had(self):
        outer = Outer(shown={"special:scratch"})
        problems, _ = ended_reads(outer)
        self.assertEqual(problems, [])
        self.assertEqual(outer.shown, {"special:scratch"})
        self.assertEqual(outer.focused, APP_WINDOW)

    def test_a_toggle_that_takes_effect_and_then_raises_still_hides_the_nested_window(self):
        outer = Outer(raises_once=True)
        with self.assertRaises(RuntimeError):
            ended_reads(outer)
        self.assertEqual(outer.shown, set())
        self.assertEqual(outer.focused, APP_WINDOW)

    def test_a_cursor_layer_that_outlives_session_end_is_a_problem_and_its_restored_read_drifts(self):
        outer = Outer(ghost_frames=10**9)
        problems, restored = ended_reads(outer)
        self.assertEqual(len(problems), 1)
        self.assertIn("still lists", problems[0])
        self.assertEqual(restored, [GHOST])
        self.assertEqual(outer.shown, set())

if __name__ == "__main__":
    unittest.main()
