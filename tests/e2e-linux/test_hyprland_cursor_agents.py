from __future__ import annotations

import os
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_cursor as cursor
from scenarios import hyprland_cursor_agents as agents

LEG_IDS = ("cursor.agent_identity_and_profiles", "cursor.missing_agent_id_refused",
           "cursor.arrival_ack_before_dispatch")


def envelope(ok: bool, error: dict | None, exit_code: int) -> Envelope:
    return Envelope(ok=ok, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=exit_code)


def surface(pid: int, x: int, y: int, size: tuple[int, int] = (24, 32)) -> dict:
    return {"namespace": "agent-desktop-cursor", "pid": pid, "x": x, "y": y, "w": size[0], "h": size[1]}


class CursorAgentLegTests(unittest.TestCase):
    def test_every_leg_is_registered_on_hyprland_with_an_implementation(self):
        for leg_id in LEG_IDS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
            self.assertIn("hyprland", leg.oracles)
            self.assertIn(leg_id, legs.implementations())
        self.assertEqual(legs.leg_by_id("cursor.missing_agent_id_refused").disposition, "asserted-refusal")

    def test_cursor_surfaces_keep_geometry_and_skip_other_namespaces(self):
        answer = {"DP-1": {"levels": {
            "2": [{"namespace": "waybar", "pid": 3, "x": 0, "y": 0, "w": 1920, "h": 30}],
            "3": [surface(41, 499, 199), surface(41, 299, 399, (46, 62))],
        }}}
        found = agents.cursor_surfaces(answer)
        self.assertEqual([(item["x"], item["y"], item["w"]) for item in found], [(299, 399, 46), (499, 199, 24)])
        self.assertTrue(all(item["level"] == "3" for item in found))
        self.assertEqual(agents.cursor_surfaces({}), [])

    def test_a_cursor_is_found_by_its_tip_one_pixel_inside_the_surface(self):
        found = agents.cursor_surfaces({"DP-1": {"levels": {"3": [surface(41, 299, 399), surface(41, 499, 199)]}}})
        self.assertEqual(agents.surface_at(found, (300.0, 400.0))["x"], 299)
        self.assertIsNone(agents.surface_at(found, (320.0, 400.0)))
        self.assertEqual(agents.placed(found, [(300.0, 400.0), (500.0, 200.0)], 2), found)
        self.assertIsNone(agents.placed(found, [(300.0, 400.0)], 3))

    def test_only_an_invalid_args_exit_one_counts_as_the_refusal(self):
        refused = envelope(False, {"code": "INVALID_ARGS"}, 1)
        self.assertEqual(agents.refusal_problems(refused), [])
        self.assertEqual(len(agents.refusal_problems(envelope(True, None, 0))), 1)
        self.assertEqual(len(agents.refusal_problems(envelope(False, {"code": "ACTION_FAILED"}, 1))), 1)

    def test_surfaces_must_all_belong_to_the_recorded_live_renderer(self):
        owner = {"pid": os.getpid(), "start_time": cursor.start_time(os.getpid())}
        mine = agents.cursor_surfaces({"DP-1": {"levels": {"3": [surface(os.getpid(), 1, 1)]}}})
        self.assertEqual(agents.owned_problems("x", mine, owner), [])
        theirs = agents.cursor_surfaces({"DP-1": {"levels": {"3": [surface(1, 1, 1)]}}})
        self.assertEqual(len(agents.owned_problems("x", theirs, owner)), 1)
        self.assertEqual(len(agents.owned_problems("x", mine, None)), 1)

    def test_the_arrival_watch_judges_the_layer_read_after_the_display_changes(self):
        reads = iter(["7", "7", "73"])
        arrived = [surface(41, 99, 99)]
        with agents.ArrivalWatch(lambda: next(reads, "73"), lambda: agents.cursor_surfaces(
                {"M": {"levels": {"3": arrived}}}), "7", (100.0, 100.0)) as watch:
            pass
        self.assertEqual(watch.verdict[0], True)
        reads = iter(["7", "73"])
        with agents.ArrivalWatch(lambda: next(reads, "73"), lambda: agents.cursor_surfaces(
                {"M": {"levels": {"3": [surface(41, 500, 500)]}}}), "7", (100.0, 100.0)) as watch:
            pass
        self.assertEqual(watch.verdict[0], False)


if __name__ == "__main__":
    unittest.main()
