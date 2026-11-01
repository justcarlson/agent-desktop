from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import pixels
from runner import legs
from scenarios import hyprland_cursor as cursor
from scenarios import hyprland_cursor_placement as placement

LEG_ID = "cursor.layout_placement_and_hidden_targets"
WHITE = (255, 255, 255)
BLACK = (0, 0, 0)
HOME = {"name": "HEADLESS-1", "x": 0, "y": 0, "width": 1280, "height": 800, "scale": 1.0}
EXTRA = {"name": "AD-E2E-1", "x": -1024, "y": -640, "width": 1280, "height": 800, "scale": 1.25}


def with_arrow(width: int, height: int, box: tuple[int, int, int, int]) -> pixels.Image:
    image = bytearray(pixels.solid(width, height, WHITE).rgb)
    x, y, box_width, box_height = box
    for row in range(y, y + box_height):
        for column in range(x, x + box_width):
            offset = (row * width + column) * 3
            image[offset:offset + 3] = bytes(BLACK)
    return pixels.Image(width=width, height=height, rgb=bytes(image))


def surface(pid: int, monitor: str, x: int, y: int) -> dict:
    return {"pid": pid, "monitor": monitor, "x": x, "y": y}


class CursorPlacementLegTests(unittest.TestCase):
    def test_the_leg_is_registered_on_hyprland_headless_with_an_implementation(self):
        leg = legs.leg_by_id(LEG_ID)
        self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
        for oracle in ("hyprland", "pixels"):
            self.assertIn(oracle, leg.oracles)
        self.assertIn(LEG_ID, legs.implementations())

    def test_the_drawn_box_is_the_region_that_differs(self):
        bare = pixels.solid(64, 64, WHITE)
        drawn = with_arrow(64, 64, (20, 30, 5, 7))
        self.assertEqual(placement.drawn_box(drawn, bare, (0, 0, 64, 64)), (20, 30, 5, 7))
        self.assertIsNone(placement.drawn_box(drawn, bare, (40, 0, 24, 64)))
        self.assertIsNone(placement.drawn_box(bare, bare, (0, 0, 64, 64)))

    def test_the_tip_must_sit_on_the_point_in_physical_pixels(self):
        point = (-1024 + 100, -640 + 40)
        self.assertEqual(placement.tip_problems("t", (125, 50, 30, 40), EXTRA, point), [])
        self.assertEqual(placement.tip_problems("t", (126, 56, 30, 40), EXTRA, point), [])
        self.assertEqual(len(placement.tip_problems("t", (125, 46, 30, 40), EXTRA, point)), 1)
        self.assertEqual(len(placement.tip_problems("t", (125, 58, 30, 40), EXTRA, point)), 1)
        self.assertEqual(len(placement.tip_problems("t", (129, 50, 30, 40), EXTRA, point)), 1)
        self.assertEqual(len(placement.tip_problems("t", (100, 40, 30, 40), EXTRA, point)), 1)
        self.assertEqual(len(placement.tip_problems("t", None, EXTRA, point)), 1)

    def test_the_search_region_starts_near_the_tip_and_stays_inside_the_image(self):
        region = placement.search_region(EXTRA, (-1024 + 100, -640 + 40), (1280, 800))
        self.assertEqual(region, (119, 44, 60, 60))
        edge = placement.search_region(HOME, (1270, 795), (1280, 800))
        self.assertEqual(edge[0] + edge[2], 1280)
        self.assertEqual(edge[1] + edge[3], 800)

    def test_the_arrow_size_must_follow_each_output_scale(self):
        right = {"AD-E2E-1": ((0, 0, 30, 40), 1.25), "HEADLESS-1": ((0, 0, 24, 32), 1.0)}
        self.assertEqual(placement.scale_problems(right), [])
        wrong = {"AD-E2E-1": ((0, 0, 24, 32), 1.25), "HEADLESS-1": ((0, 0, 24, 32), 1.0)}
        self.assertEqual(len(placement.scale_problems(wrong)), 2)
        with self.assertRaises(AssertionError):
            placement.scale_problems({"A": ((0, 0, 24, 32), 1.0), "B": ((0, 0, 24, 32), 1.0)})

    def test_the_cursor_surface_must_be_on_the_named_output_alone(self):
        point = (100.0, 200.0)
        right = [surface(7, "HEADLESS-1", 99, 199), surface(8, "AD-E2E-1", 99, 199)]
        self.assertEqual(placement.layer_problems("l", right, 7, "HEADLESS-1", point), [])
        self.assertEqual(len(placement.layer_problems("l", right, 7, "AD-E2E-1", point)), 1)
        twice = right + [surface(7, "AD-E2E-1", 99, 199)]
        self.assertEqual(len(placement.layer_problems("l", twice, 7, "HEADLESS-1", point)), 1)
        self.assertEqual(len(placement.layer_problems("l", [surface(7, "HEADLESS-1", 40, 40)], 7, "HEADLESS-1",
                                                      point)), 1)

    def test_a_hidden_target_must_draw_nothing_on_the_shown_output(self):
        bare = pixels.solid(200, 200, WHITE)
        self.assertEqual(placement.hidden_problems("h", bare, bare, HOME, [(50, 50), (150, 150)]), [])
        drawn = with_arrow(200, 200, (150, 150, 10, 10))
        found = placement.hidden_problems("h", drawn, bare, HOME, [(50, 50), (150, 150)])
        self.assertEqual(len(found), 1)
        self.assertIn("(150, 150)", found[0])

    def test_only_points_on_the_monitor_are_read_from_its_image(self):
        self.assertTrue(placement.on_monitor(HOME, (0, 0)))
        self.assertFalse(placement.on_monitor(HOME, (-1, 10)))
        self.assertTrue(placement.on_monitor(EXTRA, (-1, -1)))
        self.assertFalse(placement.on_monitor(EXTRA, (0, -1)))

    def test_a_cursor_left_on_the_output_shows_in_the_leftover_boxes(self):
        bare = pixels.solid(200, 200, WHITE)
        left = with_arrow(200, 200, (151, 151, 10, 10))
        self.assertEqual(placement.leftover_boxes(bare, bare, HOME, [(50, 50), (150, 150)]), [None, None])
        self.assertEqual(placement.leftover_boxes(bare, left, HOME, [(50, 50), (150, 150)]),
                         [None, (151, 151, 10, 10)])

    def test_the_os_pointer_must_not_move(self):
        self.assertEqual(placement.pointer_problems("p", {"x": 1, "y": 2}, {"x": 1, "y": 2}), [])
        self.assertEqual(len(placement.pointer_problems("p", {"x": 1, "y": 2}, {"x": 3, "y": 2})), 1)

    def test_a_monitor_is_focused_by_name_in_both_config_languages(self):
        self.assertEqual(cursor.focus_monitor_argv("AD-E2E-1", False), ["dispatch", "focusmonitor", "AD-E2E-1"])
        lua = cursor.focus_monitor_argv("AD-E2E-1", True)
        self.assertEqual(lua[0], "dispatch")
        self.assertIn("hl.dsp.focus", lua[1])
        self.assertIn('"AD-E2E-1"', lua[1])


if __name__ == "__main__":
    unittest.main()
