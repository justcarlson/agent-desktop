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

from runner import legs
from runner.product import Envelope
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


if __name__ == "__main__":
    unittest.main()
