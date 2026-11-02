import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from hyprland_leg_fakes import FakeContext, delivered, fake_app, nested_patches
from runner.product import Envelope
from runner.verdict import evaluate
from scenarios import hyprland_pointer_ref_refusals as refusals

FALLBACK_TEXT = "Retry without --headed, or use --xy"
UNPROVEN = {"code": "ACTION_FAILED", "message": FALLBACK_TEXT, "suggestion": FALLBACK_TEXT,
            "details": {"kind": "receiver_evidence", "cause": "receiver_unproven", "physical_delivery_started": False},
            "disposition": {"delivery": "not_delivered", "retry": "safe"}}
NATIVE = {"code": "ACTION_FAILED", "message": "native", "details": {"kind": "receiver_evidence",
          "cause": "native_wayland_client", "physical_delivery_started": False},
          "disposition": {"delivery": "not_delivered", "retry": "safe"}}
POLICY = {"code": "POLICY_DENIED", "message": "headed input needs --headed",
          "disposition": {"delivery": "not_delivered", "retry": "safe"}}
MOTION = {"ev": "motion", "widget": "area", "t": 1.0, "wx": 3, "wy": 4}


def refused(error: dict) -> Envelope:
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=30.0,
                    argv=["drag"], exit_code=1)


def by_backend(argv, backend: str) -> Envelope:
    return refused(NATIVE if backend == refusals.NATIVE else UNPROVEN)


def always(envelope: Envelope):
    return lambda argv, backend: envelope


class LegRunTests(unittest.TestCase):
    def run_leg(self, leg, mode: str, answer, lines=(), cursors=None) -> tuple[FakeContext, str | None]:
        cursor_reads = iter(cursors) if cursors else None
        with tempfile.TemporaryDirectory() as directory:
            pads = fake_app(Path(directory), "pad", list(lines))

            shown = {}

            def pad(tier, instance, label, backend):
                shown["backend"] = backend
                return pads()

            patches = {**nested_patches(), "pads_left": lambda tier: [], "pad_ref": lambda *a: "@s1:e1",
                       "pointer_pad": pad, "active_address": lambda *a: "0x1"}
            if cursor_reads:
                patches["cursor_at"] = lambda *a: next(cursor_reads)
            ctx = FakeContext(lambda argv: answer(argv, shown.get("backend")))
            ctx.mode = mode
            with mock.patch.multiple(refusals, **patches), mock.patch.object(refusals, "QUIET_S", 0.0):
                try:
                    leg(ctx)
                except AssertionError as error:
                    return ctx, str(error)
        return ctx, None

    def test_headed_ref_drag_refused_by_receiver_on_both_pads_passes(self):
        ctx, failure = self.run_leg(refusals.ref_drag_refused, "headed", by_backend)
        self.assertIsNone(failure)
        self.assertEqual(evaluate(ctx), [])
        self.assertEqual(set(ctx.reads), {"hyprland.cursor", "applog.pad"})
        self.assertEqual(len(ctx._products), 2 * len(refusals.drag_argvs("@a", "@b", {"at": [0, 0],
                                                                                        "size": [10, 10]})))

    def test_headed_ref_drag_answered_with_the_wrong_cause_fails(self):
        _, failure = self.run_leg(refusals.ref_drag_refused, "headed", always(refused(UNPROVEN)))
        self.assertIn("native_wayland_client", failure or "")

    def test_a_delivered_ref_drag_fails(self):
        _, failure = self.run_leg(refusals.ref_drag_refused, "headed", always(delivered({})))
        self.assertIsNotNone(failure)

    def test_a_refused_drag_the_pad_still_saw_fails(self):
        _, failure = self.run_leg(refusals.ref_drag_refused, "headed", by_backend, lines=[MOTION])
        self.assertIn("the pad logged", failure or "")

    def test_headless_ref_drag_refused_by_policy_passes(self):
        ctx, failure = self.run_leg(refusals.ref_drag_headless_refused, "headless", always(refused(POLICY)))
        self.assertIsNone(failure)
        self.assertEqual(evaluate(ctx), [])

    def test_headless_ref_drag_refused_for_its_receiver_fails(self):
        _, failure = self.run_leg(refusals.ref_drag_headless_refused, "headless", by_backend)
        self.assertIn("POLICY_DENIED", failure or "")

    def test_headless_hover_by_ref_refused_by_policy_passes(self):
        ctx, failure = self.run_leg(refusals.hover_by_ref_headless_refused, "headless",
                                    always(refused(POLICY)))
        self.assertIsNone(failure)
        self.assertEqual(evaluate(ctx), [])
        self.assertTrue(all(envelope.error == POLICY for envelope in ctx._products))

    def test_headless_hover_that_moved_the_cursor_fails(self):
        cursors = [(1.0, 1.0)] + [(1.0, 1.0), (1.0, 1.0), (9.0, 9.0)] * 4
        _, failure = self.run_leg(refusals.hover_by_ref_headless_refused, "headless",
                                  always(refused(POLICY)), cursors=cursors)
        self.assertIn("cursor moved", failure or "")

    def test_every_drag_names_a_ref_end(self):
        for argv in refusals.drag_argvs("@s1:e1", "@s1:e2", {"at": [0, 0], "size": [100, 100]}):
            self.assertEqual(argv[0], "drag")
            self.assertTrue(any(part.startswith("@") for part in argv), argv)


if __name__ == "__main__":
    unittest.main()
