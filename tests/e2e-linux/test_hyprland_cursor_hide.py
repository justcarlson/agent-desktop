from __future__ import annotations

import json
import os
import stat
import sys
import tempfile
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

LEG_IDS = ("cursor.hide_ack_suppression_until_show", "cursor.travel_failure_proceeds_hide_failure_refuses",
           "pointer.ref_receiver_evidence_under_cursor", "pointer.cursor_pass_through")


def refused(kind: str, delivery: str = "not_delivered", retry: str = "safe", started: bool = False) -> Envelope:
    error = {"code": "TIMEOUT", "details": {"kind": kind, "physical_delivery_started": started},
             "disposition": {"delivery": delivery, "retry": retry}}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def shell_refusal(delivery: str = "not_delivered", retry: str = "safe") -> Envelope:
    check = {"check": "receives_events", "status": "fail", "occluder": {"name": "omarchy-notifications"}}
    error = {"code": "ACTION_FAILED", "details": {"checks": [check]},
             "disposition": {"delivery": delivery, "retry": retry}}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def native_refusal(retry: str = "safe", started: bool = False) -> Envelope:
    error = {"code": "ACTION_FAILED", "details": {"kind": "receiver_evidence", "cause": "native_wayland_client",
                                                  "physical_delivery_started": started},
             "disposition": {"delivery": "not_delivered", "retry": retry}}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


class Findings:
    def __init__(self):
        self.found = []

    def finding(self, text: str) -> None:
        self.found.append(text)


class Reads:
    def __init__(self):
        self.restored_reads = {}

    def restored(self, oracle: str, value) -> None:
        self.restored_reads.setdefault(oracle, []).append(value)


def arrow_in(image: pixels.Image, box: tuple[int, int, int, int], color=(66, 153, 255)) -> pixels.Image:
    """`image` with an arrow-like wedge of `color` from the box's top left."""
    x, y, width, height = box
    rgb = bytearray(image.rgb)
    for row in range(3, height):
        for column in range(1, max(1, row * 2 // 3)):
            offset = ((y + row) * image.width + x + column) * 3
            rgb[offset:offset + 3] = bytes(color)
    return pixels.Image(width=image.width, height=image.height, rgb=bytes(rgb))


def ended_reads(answers: list) -> tuple[list[str], list]:
    """The problems and the restored layer reads after session end, while `layers_of` gives `answers`."""
    ctx, pending = Reads(), iter(answers)
    last = []

    def layers_of(runtime, instance):
        nonlocal last
        last = next(pending, last)
        return last

    owner = {"pid": 5, "start_time": 1}
    with mock.patch.object(cursor, "layers_of", layers_of), mock.patch.object(hide, "layers_of", layers_of), \
            mock.patch.object(cursor, "alive", lambda owner: False), \
            mock.patch.object(cursor, "record_of", lambda runtime, session: None), \
            mock.patch.object(cursor, "SETTLE_S", 0.2):
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
        self.assertEqual(len(dead_click(native_refusal(retry="unsafe"), "7")[0]), 1)
        self.assertEqual(len(dead_click(native_refusal(), "77")[0]), 1)
        self.assertEqual(len(dead_click(native_refusal(started=True), "7")[0]), 1)

    def test_the_pad_legs_under_the_cursor_drive_an_xwayland_pad(self):
        source = (Path(hide.__file__)).read_text()
        self.assertIn("pointer_pad(tier, instance, label, XWAYLAND)", source)

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
        ghost = [{"monitor": "WAYLAND-1", "level": "3", "pid": -1}]
        problems, restored = ended_reads([ghost, ghost, ghost, []])
        self.assertEqual(problems, [])
        self.assertEqual(restored, [[]])

    def test_a_cursor_layer_that_outlives_session_end_is_a_problem_and_its_restored_read_drifts(self):
        ghost = [{"monitor": "WAYLAND-1", "level": "3", "pid": -1}]
        problems, restored = ended_reads([ghost] * 400)
        self.assertEqual(len(problems), 1)
        self.assertIn("still lists", problems[0])
        self.assertEqual(restored, [ghost])


if __name__ == "__main__":
    unittest.main()
