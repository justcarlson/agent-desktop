from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import wayland_outputs


def monitor(name: str, x: int, y: int, *, width: int = 1280, height: int = 800, scale: float = 1.0,
            transform: int = 0, disabled: bool = False) -> dict:
    return {"name": name, "x": x, "y": y, "width": width, "height": height, "scale": scale,
            "transform": transform, "disabled": disabled}


def answer(*displays: dict) -> Envelope:
    data = [{"id": item["id"], "is_primary": item["is_primary"], "scale": item["scale"],
             "bounds": {key: item[key] for key in ("x", "y", "width", "height")}} for item in displays]
    return Envelope(ok=True, data=data, error=None, raw="[]", elapsed_ms=1.0, argv=[], exit_code=0)


def variant(kind: str, value) -> dict:
    return {"type": kind, "data": value}


def mutter_reply(layout_mode: int, scale: float, primary_connector: str = "Meta-0") -> dict:
    def physical(connector: str, width: int, height: int) -> list:
        modes = [["other", 640, 480, 60.0, 1.0, [1.0], {}],
                 ["current", width, height, 60.0, 1.0, [1.0], {"is-current": variant("b", True)}]]
        return [[connector, "MetaProducts Inc.", "virtual", "0x00"], modes, {}]

    logical = [[0, 0, scale, 0, primary_connector == "Meta-0", [["Meta-0", "", "", ""]], {}],
               [1280, 0, 1.0, 0, primary_connector == "Meta-1", [["Meta-1", "", "", ""]], {}]]
    return {"type": "u...", "data": [7, [physical("Meta-0", 2560, 1600), physical("Meta-1", 1024, 768)],
                                     logical, {"layout-mode": variant("u", layout_mode)}]}


class HyprlandExpectationTests(unittest.TestCase):
    def test_the_origin_output_is_primary_and_listed_first_whatever_its_name(self):
        expected = wayland_outputs.expected_from_monitors(
            [monitor("AGENT-DESKTOP-1", 1280, 0), monitor("WAYLAND-1", 0, 0)])
        self.assertEqual([(item["id"], item["is_primary"]) for item in expected],
                         [("WAYLAND-1", True), ("AGENT-DESKTOP-1", False)])

    def test_the_origin_output_wins_over_one_placed_above_it(self):
        expected = wayland_outputs.expected_from_monitors(
            [monitor("DP-1", 1280, -200), monitor("eDP-1", 0, 0)])
        self.assertEqual([(item["id"], item["is_primary"]) for item in expected],
                         [("eDP-1", True), ("DP-1", False)])

    def test_with_nothing_at_the_origin_the_top_left_output_is_primary(self):
        expected = wayland_outputs.expected_from_monitors(
            [monitor("DP-2", 1280, 100), monitor("DP-1", 2560, -200), monitor("HDMI-A-1", 0, 100)])
        self.assertEqual([(item["id"], item["is_primary"]) for item in expected],
                         [("DP-1", True), ("DP-2", False), ("HDMI-A-1", False)])

    def test_mirrored_outputs_at_the_origin_make_the_lowest_name_primary(self):
        expected = wayland_outputs.expected_from_monitors([monitor("eDP-1", 0, 0), monitor("DP-1", 0, 0)])
        self.assertEqual(expected[0]["id"], "DP-1")
        self.assertTrue(expected[0]["is_primary"])

    def test_bounds_are_logical_and_turn_with_a_quarter_transform(self):
        flat, turned = wayland_outputs.expected_from_monitors(
            [monitor("A-1", 0, 0, scale=1.25), monitor("B-1", 1024, 0, scale=1.25, transform=1)])
        self.assertEqual((flat["width"], flat["height"], flat["scale"]), (1024.0, 640.0, 1.25))
        self.assertEqual((turned["width"], turned["height"]), (640.0, 1024.0))

    def test_a_disabled_output_is_not_a_display(self):
        expected = wayland_outputs.expected_from_monitors([monitor("A-1", 0, 0), monitor("B-1", 0, 0, disabled=True)])
        self.assertEqual([item["id"] for item in expected], ["A-1"])


class MutterExpectationTests(unittest.TestCase):
    def test_a_logical_layout_divides_the_current_mode_by_the_scale(self):
        primary, other = wayland_outputs.expected_from_mutter(mutter_reply(1, 2.0))
        self.assertEqual((primary["id"], primary["is_primary"], primary["width"], primary["height"],
                          primary["scale"]), ("Meta-0", True, 1280.0, 800.0, 2.0))
        self.assertEqual((other["id"], other["x"], other["width"]), ("Meta-1", 1280.0, 1024.0))

    def test_a_physical_layout_keeps_the_current_mode(self):
        primary, _ = wayland_outputs.expected_from_mutter(mutter_reply(2, 2.0))
        self.assertEqual((primary["width"], primary["height"]), (2560.0, 1600.0))

    def test_mutter_names_the_primary_even_away_from_the_origin(self):
        expected = wayland_outputs.expected_from_mutter(mutter_reply(1, 1.0, primary_connector="Meta-1"))
        self.assertEqual([(item["id"], item["is_primary"]) for item in expected],
                         [("Meta-1", True), ("Meta-0", False)])


class DisplayProblemTests(unittest.TestCase):
    def setUp(self):
        self.expected = wayland_outputs.expected_from_monitors(
            [monitor("WAYLAND-1", 0, 0), monitor("AGENT-DESKTOP-1", 1280, 0, scale=1.25)])

    def test_a_matching_answer_has_no_problem(self):
        self.assertEqual(wayland_outputs.display_problems("x", answer(*self.expected), self.expected), [])

    def test_a_rounded_logical_size_is_within_tolerance(self):
        near = [dict(item, width=item["width"] + 1) for item in self.expected]
        self.assertEqual(wayland_outputs.display_problems("x", answer(*near), self.expected), [])

    def test_each_disagreement_is_a_problem(self):
        first, second = self.expected
        for name, displays in {
            "order": (second, first),
            "primary": (dict(first, is_primary=False), second),
            "extra": (first, second, dict(second, id="X-1")),
            "position": (first, dict(second, x=1281.0)),
            "size": (first, dict(second, width=second["width"] + 2)),
            "scale": (first, dict(second, scale=1.0)),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(wayland_outputs.display_problems("x", answer(*displays), self.expected))

    def test_a_refusal_or_an_empty_oracle_is_never_a_match(self):
        refused = Envelope(False, None, {"code": "PLATFORM_NOT_SUPPORTED"}, "{}", 1.0, [], exit_code=1)
        self.assertTrue(wayland_outputs.display_problems("x", refused, self.expected))
        self.assertTrue(wayland_outputs.display_problems("x", answer(), []))


class OracleCommandTests(unittest.TestCase):
    def test_the_hyprland_oracle_is_hyprctl_against_one_instance(self):
        with mock.patch.object(wayland_outputs.hyprland, "control", return_value="[]") as control:
            self.assertEqual(wayland_outputs.monitors(Path("/run/user/1000"), "sig"), [])
        control.assert_called_once_with(Path("/run/user/1000"), "sig", ["-j", "monitors"])

    def test_the_mutter_oracle_asks_the_tier_bus_through_busctl(self):
        result = SimpleNamespace(returncode=0, stdout='{"type": "u", "data": []}', stderr="")
        env = {"DBUS_SESSION_BUS_ADDRESS": "unix:path=/tmp/bus"}
        with mock.patch.object(wayland_outputs.subprocess, "run", return_value=result) as run:
            wayland_outputs.mutter_state(env)
        argv = run.call_args.args[0]
        self.assertEqual(argv[:3], ["busctl", "--address=unix:path=/tmp/bus", "--json=short"])
        self.assertEqual(argv[-1], "GetCurrentState")

    def test_a_silent_mutter_fails_the_leg(self):
        result = SimpleNamespace(returncode=1, stdout="", stderr="no such name")
        with mock.patch.object(wayland_outputs.subprocess, "run", return_value=result):
            with self.assertRaises(AssertionError):
                wayland_outputs.mutter_state({"DBUS_SESSION_BUS_ADDRESS": "unix:path=/tmp/bus"})

    def test_the_scale_command_follows_the_config_language(self):
        self.assertEqual(wayland_outputs.scale_command(False, 1.25),
                         ["keyword", "monitor", "AGENT-DESKTOP-1,1280x800@60,1280x0,1.25"])
        self.assertIn("scale = 1.25", wayland_outputs.scale_command(True, 1.25)[1])

    def test_hyprland_like_variables_keep_the_tier_environment(self):
        envs = wayland_outputs.hyprland_like_envs({"WAYLAND_DISPLAY": "wayland-0"})
        self.assertTrue(all(env["WAYLAND_DISPLAY"] == "wayland-0" for env in envs.values()))
        self.assertEqual(envs["both"]["XDG_CURRENT_DESKTOP"], "Hyprland")


class FakeInstances:
    """`hyprctl` for the session and a second instance whose nested output starts at its window size."""

    def __init__(self, settles: bool = True):
        self.settles = settles
        self.commands = []
        self.env = {"WAYLAND_DISPLAY": "wayland-1", "HYPRLAND_INSTANCE_SIGNATURE": "main"}
        self.state = {"main": [monitor("Virtual-1", 0, 0)],
                      "nested": [monitor("WAYLAND-1", 0, 0, width=1256, height=750),
                                 monitor("AGENT-DESKTOP-1", 1280, 0)]}

    def control(self, runtime, signature, argv):
        if argv == ["-j", "monitors"]:
            return json.dumps(self.state[signature])
        self.commands.append(argv)
        name, mode, _, scale = argv[2].split(",")
        width, height = (int(side) for side in mode.split("@")[0].split("x"))
        for item in self.state[signature]:
            if item["name"] == name and (self.settles or name != "WAYLAND-1"):
                item.update(width=width, height=height, scale=float(scale))
        return "ok"

    def product(self, argv, env=None, headed=None):
        signature = (env or self.env)["HYPRLAND_INSTANCE_SIGNATURE"]
        return answer(*wayland_outputs.expected_from_monitors(self.state[signature]))


class RecordingContext:
    def __init__(self, fake: FakeInstances):
        self.tier = SimpleNamespace(env=fake.env)
        self.product = fake.product
        self.reads = {"before": {}, "after": {}, "restored": {}}

    def before(self, channel, value):
        self.reads["before"][channel] = value

    def after(self, channel, value):
        self.reads["after"][channel] = value

    def restored(self, channel, value):
        self.reads["restored"][channel] = value


class LegBaselineTests(unittest.TestCase):
    def run_leg(self, fake: FakeInstances) -> RecordingContext:
        ctx = RecordingContext(fake)
        with mock.patch.object(wayland_outputs, "shared", return_value=Path("/run/user/1000")), \
                mock.patch.object(wayland_outputs, "main_instance", return_value={"signature": "main"}), \
                mock.patch.object(wayland_outputs.second_instance, "ensure",
                                  return_value={"signature": "nested", "wayland": "wayland-2"}), \
                mock.patch.object(wayland_outputs.hyprland, "uses_lua", return_value=False), \
                mock.patch.object(wayland_outputs.hyprland, "control", side_effect=fake.control):
            wayland_outputs.hyprland_displays_match_monitors(ctx)
        return ctx

    def test_the_baseline_is_read_after_the_nested_output_takes_the_config_mode(self):
        fake = FakeInstances()
        ctx = self.run_leg(fake)
        before = ctx.reads["before"]["hyprland.second_monitors"]
        self.assertEqual([(item["name"], item["width"], item["height"]) for item in before],
                         [("WAYLAND-1", 1280, 800), ("AGENT-DESKTOP-1", 1280, 800)])
        self.assertEqual(fake.commands[0], ["keyword", "monitor", "WAYLAND-1,1280x800@60,0x0,1"])
        self.assertEqual(ctx.reads["restored"]["hyprland.second_monitors"], before)

    def test_outputs_that_never_settle_are_a_fixture_error_with_the_monitors(self):
        fake = FakeInstances(settles=False)
        with mock.patch.object(wayland_outputs.time, "sleep"), \
                mock.patch.object(wayland_outputs.time, "monotonic", side_effect=[0.0, 0.0, 11.0]), \
                mock.patch.object(wayland_outputs.hyprland, "control", side_effect=fake.control):
            with self.assertRaises(wayland_outputs.OutputsNotSettled) as raised:
                wayland_outputs.settle_outputs(Path("/run/user/1000"), "nested", False)
        self.assertNotIsInstance(raised.exception, AssertionError)
        self.assertIn('"width": 1256', str(raised.exception))

    def test_settled_outputs_take_no_rule(self):
        fake = FakeInstances()
        fake.state["nested"][0].update(width=1280, height=800)
        with mock.patch.object(wayland_outputs.hyprland, "control", side_effect=fake.control):
            wayland_outputs.settle_outputs(Path("/run/user/1000"), "nested", True)
        self.assertEqual(fake.commands, [])


class RegistrationTests(unittest.TestCase):
    def test_each_leg_runs_only_on_its_tier_with_its_oracle(self):
        hyprland = legs.leg_by_id("observation.hyprland_displays_match_monitors")
        generic = legs.leg_by_id("session.other_wayland_route_unchanged")
        self.assertEqual((hyprland.tiers, hyprland.oracles), (("hyprland",), ("hyprland",)))
        self.assertEqual((generic.tiers, generic.oracles), (("wayland",), ("dbus",)))
        implemented = legs.implementations()
        self.assertIs(implemented[hyprland.id], wayland_outputs.hyprland_displays_match_monitors)
        self.assertIs(implemented[generic.id], wayland_outputs.other_wayland_route_unchanged)

    def test_the_second_instance_exists_before_the_display_leg_runs(self):
        ids = [leg.id for leg in legs.REGISTRY]
        self.assertLess(ids.index("non_interference.headed_changes_only_target"),
                        ids.index("observation.hyprland_displays_match_monitors"))


if __name__ == "__main__":
    unittest.main()
