import unittest
import argparse
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import scenarios
from runner import legs, verdict
from runner.product import Envelope
from runner.verdict import LegContext
from scenarios import capture, clipboard, headed_actions, lifecycle, text_entry
from scenarios import pointer

LANE_PREFIXES = (
    "headed_actions.",
    "pointer.",
    "occlusion.",
    "modal.",
    "text_entry.",
    "non_interference.",
    "lifecycle.",
    "capture.",
    "clipboard.",
)

TREE = {
    "role": "window",
    "name": "Calculator",
    "children": [
        {"ref_id": "@e1", "role": "button", "name": "7"},
        {"ref_id": "@s1:e2", "role": "button", "name": "7"},
        {"ref_id": "@s1:e3", "role": "textfield", "name": "Display"},
        {"role": "group", "name": "Empty", "children": [{"ref_id": "@s1:e4", "role": "switch", "name": "Mode"}]},
    ],
}


class TreeHelperTests(unittest.TestCase):
    def test_find_ref_returns_the_first_qualified_match(self):
        self.assertEqual(headed_actions.find_ref(TREE, name="7"), "@s1:e2")

    def test_find_ref_filters_by_role(self):
        self.assertEqual(headed_actions.find_ref(TREE, role="switch"), "@s1:e4")
        self.assertEqual(headed_actions.find_ref(TREE, name="7", role="textfield"), None)

    def test_find_ref_ignores_a_bare_storage_ref(self):
        tree = {"children": [{"ref_id": "@e9", "role": "button", "name": "Send"}]}
        self.assertIsNone(headed_actions.find_ref(tree, name="Send"))

    def test_count_name_reaches_nested_children(self):
        self.assertEqual(headed_actions.count_name(TREE, "7"), 2)
        self.assertEqual(headed_actions.count_name(TREE, "Absent"), 0)

    def test_walk_flattens_the_whole_tree(self):
        self.assertEqual(len(headed_actions.walk(TREE)), 6)


class PointerMathTests(unittest.TestCase):
    def test_pointer_observation_is_checked_after_recording(self):
        tier = SimpleNamespace(runtime_dir=Path("/nonexistent/private"))
        ctx = LegContext(tier, "headed", member_reader=lambda _: frozenset())
        response = Envelope(True, {}, None, "{}", 0, [])
        with patch.object(pointer.x11, "geometry", return_value=(0, 0, 100, 100)), \
            patch.object(pointer.x11, "pointer", return_value=(20, 30)), \
            patch.object(pointer, "_button_point", return_value=(20, 30)), \
            patch("runner.verdict.run_product", return_value=response):
            pointer._extents_at(ctx, {"window": "w-test", "name": "gtk4"}, 0)
        self.assertEqual(ctx.reads["x11.pointer.gtk4"]["after"], [(20, 30)])


    def test_frame_extents_are_read_in_left_right_top_bottom_order(self):
        self.assertEqual(pointer.parse_frame_extents("_GTK_FRAME_EXTENTS(CARDINAL) = 61, 61, 55, 67"), (61, 61, 55, 67))

    def test_missing_frame_extents_are_zero(self):
        self.assertEqual(pointer.parse_frame_extents("no such atom"), (0, 0, 0, 0))

    def test_expected_point_adds_origin_frame_and_centre(self):
        point = pointer.expected_point((100, 200), (61, 61, 55, 67), (10, 20, 30, 40))
        self.assertEqual(point, (100 + 61 + 10 + 15, 200 + 55 + 20 + 20))

    def test_expected_point_without_a_shadow_is_origin_plus_centre(self):
        self.assertEqual(pointer.expected_point((5, 7), (0, 0, 0, 0), (0, 0, 10, 10)), (10, 12))

    def test_within_tolerates_two_pixels_only(self):
        self.assertTrue(pointer.within((102, 98), (100, 100)))
        self.assertFalse(pointer.within((103, 100), (100, 100)))

    def test_activation_count_counts_non_empty_lines(self):
        self.assertEqual(pointer.activation_count("Press me\nPress me\n\n"), 2)


class DeliveryTests(unittest.TestCase):
    def test_a_parser_refusal_is_not_an_outside_screen_measurement(self):
        response = SimpleNamespace(error={"code": "INVALID_ARGS", "message": "unexpected argument",
            "disposition": {"delivery": "not_delivered", "retry": "safe"}})
        with self.assertRaisesRegex(AssertionError, "screen dimensions"):
            pointer.error_delivery(response)

    def test_outside_screen_probes_reach_argument_validation(self):
        parser = argparse.ArgumentParser()
        parser.add_argument("verb")
        for flag in ("--xy", "--from-xy", "--to-xy", "--x", "--y"):
            parser.add_argument(flag)
        for verb in ("drag", "mouse-wheel", "mouse-click"):
            with self.subTest(verb=verb):
                parsed = parser.parse_args(pointer._refused_argv(verb))
                self.assertEqual(parsed.verb, verb)
                self.assertTrue(any(str(value).startswith("-40") for value in vars(parsed).values()))

    def test_pointer_report_is_checked_beside_independent_pointer_reads(self):
        tier = SimpleNamespace(runtime_dir=Path("/nonexistent/private"))
        ctx = LegContext(tier, "headed", legs.leg_by_id("pointer.mouse_click_delivery_report"),
                         member_reader=lambda _: frozenset())
        app = SimpleNamespace(windows=["w-test"])
        delivered = Envelope(True, {"disposition": {"delivery": "delivered_unverified", "retry": "unsafe"},
                                   "details": {"delivered_events": 3}, "steps": [{"label": "press"}]}, None, "{}", 0, [])
        refused = Envelope(False, None, {"code": "INVALID_ARGS", "details": {"screen_width": 320, "screen_height": 240}, "disposition": {
            "delivery": "not_delivered", "retry": "safe"}}, "{}", 0, [])
        position = [(10, 10)]
        def product(*args, **kw):
            position[0] = (20, 30)
            return delivered if not ctx.products else refused
        def restore(tier, argv):
            self.assertEqual(argv, ["xdotool", "mousemove", "10", "10"])
            position[0] = (10, 10)
        with patch.object(pointer.apps, "launch", return_value=app), \
            patch.object(pointer.apps, "stop"), \
            patch.object(pointer, "_button_point", return_value=(20, 30)), \
            patch.object(pointer.x11, "pointer", side_effect=lambda _: position[0]), \
            patch.object(pointer.x11, "tool", side_effect=restore), \
            patch("runner.verdict.run_product", side_effect=product):
            pointer.mouse_click_delivery_report(ctx)
        self.assertEqual(set(ctx.reads), {"x11.pointer"})
        self.assertEqual(len(ctx.products), 2)
        self.assertEqual(verdict.evaluate(ctx), [])

    def test_delivery_fields_read_the_report(self):
        data = {
            "disposition": {"delivery": "delivered_unverified", "retry": "unsafe"},
            "details": {"delivered_events": 3},
            "steps": [{"label": "press"}],
        }
        fields = pointer.delivery_fields(data)
        self.assertEqual(fields["delivery"], "delivered_unverified")
        self.assertEqual(fields["events"], 3)
        self.assertEqual(len(fields["steps"]), 1)

    def test_a_report_without_a_disposition_fails(self):
        with self.assertRaises(AssertionError):
            pointer.delivery_fields({"details": {}})

    def test_error_delivery_reads_the_error_disposition(self):
        class Envelope:
            error = {"code": "INVALID_ARGS", "details": {"screen_width": 320, "screen_height": 240}, "disposition": {"delivery": "not_delivered", "retry": "safe"}}

        self.assertEqual(pointer.error_delivery(Envelope())["delivery"], "not_delivered")


class WindowStateTests(unittest.TestCase):
    def test_hidden_state_is_parsed(self):
        text = "_NET_WM_STATE(ATOM) = _NET_WM_STATE_HIDDEN"
        self.assertTrue(lifecycle.is_hidden(text))
        self.assertEqual(lifecycle.parse_wm_state(text), {"_NET_WM_STATE_HIDDEN"})

    def test_maximised_state_is_parsed(self):
        text = "_NET_WM_STATE(ATOM) = _NET_WM_STATE_MAXIMIZED_VERT, _NET_WM_STATE_MAXIMIZED_HORZ"
        self.assertTrue(lifecycle.is_maximised(text))
        self.assertFalse(lifecycle.is_hidden(text))

    def test_member_pids_keeps_only_pids(self):
        self.assertEqual(lifecycle.member_pids([(10, 1, "a"), (11, 2, "b")]), {10, 11})


class CaptureHelperTests(unittest.TestCase):
    def test_editor_setup_resolves_the_wire_textfield_ref(self):
        app = SimpleNamespace(windows=["w-test"])
        calls = []

        def product(argv):
            calls.append(argv)
            return Envelope(True, {"tree": TREE}, None, "{}", 0, argv)

        ctx = SimpleNamespace(tier=object(), product=product)
        with patch.object(capture.apps, "launch", return_value=app), \
            patch.object(capture.atspi_raw, "poll", side_effect=lambda read, *_args, **_kw: read()), \
            patch.object(capture, "live_root"), \
            patch.object(capture, "longest_text", return_value="capture baseline"):
            self.assertEqual(capture._editor(ctx), (app, "w-test"))
        self.assertIn(["type", "@s1:e3", "capture baseline"], calls)

    def test_window_line_matches_the_exact_title(self):
        lines = ["0x04000007  0 1234 host Text Editor", "0x04000008  0 1235 host Calculator"]
        self.assertEqual(capture.match_window_line(lines, "Calculator"), "0x04000008")
        self.assertIsNone(capture.match_window_line(lines, "Editor"))

    def test_a_solid_image_is_blank(self):
        self.assertFalse(capture.non_blank(capture.pixels.solid(2, 2, (1, 2, 3))))

    def test_a_two_colour_image_is_not_blank(self):
        image = capture.pixels.Image(width=2, height=1, rgb=bytes((1, 2, 3, 4, 5, 6)))
        self.assertTrue(capture.non_blank(image))


class TextEntryHelperTests(unittest.TestCase):
    def test_modifier_state_reports_held_modifiers(self):
        text = "Shift: off\nControl: on\nAlt: off\nSuper: off"
        self.assertEqual(text_entry.held_modifiers(text), ["Control"])

    def test_all_released_is_empty(self):
        self.assertEqual(text_entry.held_modifiers("Shift: off\nControl: off"), [])


class ClipboardHelperTests(unittest.TestCase):
    def test_text_of_reads_a_text_payload(self):
        self.assertEqual(clipboard.text_of({"type": "text", "text": "value"}), "value")

    def test_text_of_reads_an_empty_clipboard(self):
        self.assertIsNone(clipboard.text_of({"found": False}))


class RegistryCoverageTests(unittest.TestCase):
    def test_every_lane_leg_has_an_implementation(self):
        registered = legs.implementations()
        missing = [leg.id for leg in legs.REGISTRY if leg.id.startswith(LANE_PREFIXES) and leg.id not in registered]
        self.assertEqual(missing, [])

    def test_no_implementation_is_registered_twice(self):
        registered = legs.implementations()
        self.assertEqual(len(registered), len(set(registered)))


if __name__ == "__main__":
    unittest.main()
