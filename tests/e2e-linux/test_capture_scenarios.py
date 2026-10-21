import base64
import unittest
from contextlib import contextmanager, ExitStack
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from runner import legs, verdict
from runner.product import Envelope
from runner.verdict import LegContext
from scenarios import capture


class CaptureScenarioTests(unittest.TestCase):
    @contextmanager
    def fixture(self, leg_id, tier_name="x11-openbox", corrupt_restore=False, cover_above=True):
        tier = SimpleNamespace(name=tier_name, runtime_dir=Path("/private/test"), env={})
        contract = legs.Leg(leg_id, "Capture and restore independent image state", "fake window",
                            (tier_name,), ("headless",), ("pixels", "x11"), "pass")
        ctx = LegContext(tier, "headless", contract, member_reader=lambda _: frozenset())
        app = SimpleNamespace(windows=["w-editor"])
        initial = capture.pixels.Image(2, 2, bytes((1, 2, 3, 4, 5, 6) * 2))
        changed = capture.pixels.Image(2, 2, bytes((170, 180, 190, 200, 210, 220) * 2))
        state = {"text": "capture baseline", "minimized": False, "raw_reads": 0, "cover": False}
        def image():
            return initial if state["text"] == "capture baseline" else changed
        def xwd(*args):
            state["raw_reads"] += 1
            rendered = changed if corrupt_restore and state["raw_reads"] >= 3 else image()
            if args[-1] is None and state["cover"]:
                rendered = changed
            return capture.pixels.encode_png(rendered)
        def launch(*args):
            state["cover"] = True
            return SimpleNamespace(windows=["w-cover"])
        def stop(app):
            if app.windows == ["w-cover"]:
                state["cover"] = False
        def product(tier, argv, **kw):
            if argv[0] == "type":
                state["text"] += argv[-1]
            elif argv[0] == "clear":
                state["text"] = ""
            elif argv[0] in ("minimize", "restore"):
                state["minimized"] = argv[0] == "minimize"
            if argv[0] == "screenshot":
                if state["minimized"] or ("--screen" in argv and tier_name == "x11-mutter"):
                    code = "TIMEOUT" if state["minimized"] else "PLATFORM_NOT_SUPPORTED"
                    return Envelope(False, None, {"code": code,
                        "details": {"reason": "compositing"}}, "{}", 0, argv)
                data = {"data": base64.b64encode(capture.pixels.encode_png(image())).decode(), "format": "png"}
                return Envelope(True, data, None, "{}", 0, argv)
            return Envelope(True, {}, None, "{}", 0, argv)
        with ExitStack() as stack:
            replacements = [
                (capture, "_editor", lambda _: (app, "w-editor")),
                (capture.apps, "launch", launch),
                (capture.apps, "stop", stop),
                (capture, "ref_for", lambda *a, **kw: "@s:e1"),
                (capture, "live_root", lambda *a: app),
                (capture, "longest_text", lambda *a: state["text"]),
                (capture, "_x11_id", lambda _, window: "0x2" if window == "w-cover" else "0x1"),
                (capture, "_xwd", xwd),
                (capture, "frame_extents", lambda *a: (0, 0, 0, 0)),
                (capture, "_minimized", lambda *a: state["minimized"]),
                (capture.pixels, "decode_xwd", capture.pixels.decode_png),
                (capture.x11, "geometry", lambda *a: (0, 0, 2, 2)),
                (capture.x11, "stacking_xids", lambda *a: (["0x1", "0x2"] if cover_above
                    else ["0x2", "0x1"]) if state["cover"] else ["0x1"]),
                (capture.atspi_raw, "poll", lambda read, *a, **kw: read()),
            ]
            for owner, name, value in replacements:
                stack.enter_context(patch.object(owner, name, value, create=True))
            stack.enter_context(patch("runner.verdict.run_product", side_effect=product))
            yield ctx, state

    def test_window_oracle_excludes_independently_reported_client_shadows(self):
        image = capture.pixels.Image(5, 6, bytes(range(90)))
        ctx = SimpleNamespace(tier=SimpleNamespace())
        with patch.object(capture, "_xwd", return_value=b"xwd"), \
            patch.object(capture, "_x11_id", return_value="0x1"), \
            patch.object(capture.pixels, "decode_xwd", return_value=image), \
            patch.object(capture, "frame_extents", return_value=(1, 1, 2, 1), create=True):
            result = capture._oracle_image(ctx, "w-editor")
        self.assertEqual((result.width, result.height), (3, 3))
        self.assertEqual(result.rgb, image.rgb[33:42] + image.rgb[48:57] + image.rgb[63:72])

    def test_window_capture_observations_without_final_reads_are_rejected(self):
        for name in ("window_pixels_match_xwd", "stable_at_idle", "covered_window_capture"):
            with self.subTest(leg=name), self.fixture("capture." + name) as (ctx, state):
                getattr(capture, name)(ctx)
                channel = {"window_pixels_match_xwd": "window_dimensions", "stable_at_idle": "idle",
                           "covered_window_capture": "covered_dimensions"}[name]
                self.assertEqual(verdict.evaluate(ctx), [
                    f"oracle pixels.{channel} needs exactly one restored read; got 0"])
                self.assertGreaterEqual(state["raw_reads"], 2)

    def test_content_change_restores_document_and_fresh_xwd_pixels(self):
        for corrupt in (False, True):
            with self.subTest(corrupt=corrupt), self.fixture(
                    "capture.content_change_moves_region", corrupt_restore=corrupt) as (ctx, state):
                if corrupt:
                    with self.assertRaises(AssertionError):
                        capture.content_change_moves_region(ctx)
                else:
                    capture.content_change_moves_region(ctx)
                    self.assertEqual(verdict.evaluate(ctx), [])
                    self.assertEqual(state["text"], "capture baseline")
                    self.assertGreaterEqual(state["raw_reads"], 3)

    def test_a_cover_below_the_target_cannot_satisfy_the_mutter_leg(self):
        with self.fixture("capture.covered_window_capture", "x11-mutter",
                          cover_above=False) as (ctx, state):
            with self.assertRaises(AssertionError):
                capture.covered_window_capture(ctx)

    def test_minimized_capture_restores_and_reobserves_the_window(self):
        with self.fixture("capture.minimised_window_capture") as (ctx, state):
            capture.minimised_window_capture(ctx)
            self.assertEqual(verdict.evaluate(ctx), [])
            self.assertFalse(state["minimized"])

    def test_display_capture_asserts_the_mutter_compositing_refusal(self):
        for tier in ("x11-openbox", "x11-mutter"):
            with self.subTest(tier=tier), self.fixture("capture.display_capture", tier) as (ctx, state):
                capture.display_capture(ctx)
                self.assertEqual(verdict.evaluate(ctx), [
                    "oracle pixels.display_dimensions needs exactly one restored read; got 0"])
                self.assertGreaterEqual(state["raw_reads"], 2)
