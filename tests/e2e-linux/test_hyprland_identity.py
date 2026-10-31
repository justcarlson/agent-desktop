from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import hyprland
from runner import apps, attached, attached_session, legs
from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import hyprland_identity as identity
from scenarios import hyprland_windows

LEG_IDS = ("identity.hyprland_accessible_listed_once", "identity.hyprland_twins_by_size_or_ambiguous",
           "identity.hyprland_closed_window_not_found", "identity.hyprland_compositor_only_id")
NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
PREFIX = "w-hypr-900-77-"


def refused(code: str, *, exit_code: int = 1, message: str = "refused", disposition=None, **details) -> Envelope:
    error = {"code": code, "message": message, "details": details,
             "disposition": NOT_DELIVERED if disposition is None else disposition}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=exit_code)


def foot() -> dict:
    return {"address": "0x5a10", "pid": 4242, "class": "ad-e2e-listed", "title": "ad-e2e-listed",
            "at": [10, 20], "size": [640, 480], "mapped": True, "hidden": False, "xwayland": False,
            "workspace": {"name": "special:ad_e2e"}, "floating": False, "fullscreen": 0}


def entry() -> dict:
    return {"id": PREFIX + "3", "accessible": False, "pid": 4242, "title": "ad-e2e-listed",
            "app_name": "ad-e2e-listed", "visible": True, "process_instance": "4242:991",
            "bounds": {"x": 10.0, "y": 20.0, "width": 640.0, "height": 480.0}}


def twin(address: str, size: list[int], workspace: str = "special:ad_e2e_twin_a") -> dict:
    return dict(foot(), address=address, title="AD Twin", size=size, workspace={"name": workspace})


class RegistryTests(unittest.TestCase):
    def test_each_identity_leg_is_a_headless_hyprland_leg_with_an_implementation(self):
        for leg_id in LEG_IDS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "pass"))
                self.assertIn("hyprland", leg.oracles)
                self.assertIn(leg_id, legs.implementations())

    def test_the_twin_app_launches_from_its_script(self):
        self.assertEqual(apps.SCRIPTS["Twin Windows"], "twin_windows.py")
        self.assertTrue((ROOT / "apps" / "twin_windows.py").is_file())
        self.assertIn("twin_windows.py", apps.ALIASES["Twin Windows"])

    def test_the_twin_app_forces_native_wayland_and_one_title(self):
        source = (ROOT / "apps" / "twin_windows.py").read_text()
        self.assertIn('os.environ["GDK_BACKEND"] = "wayland"', source)
        self.assertLess(source.index("GDK_BACKEND"), source.index("import gi"))
        self.assertIn('TITLE = "AD Twin"', source)

    def test_twin_workspaces_are_special_and_safe_to_embed(self):
        for name in identity.TWIN_SPECIALS:
            self.assertTrue(name.startswith(hyprland_windows.SPECIAL))
            hyprland_windows.move_argv("0x1", name, True)
            hyprland_windows.move_argv("0x1", name, False)


class CompositorEntryTests(unittest.TestCase):
    def test_an_entry_that_matches_its_toplevel_passes(self):
        self.assertEqual(identity.compositor_entry_problems(entry(), foot(), PREFIX), [])

    def test_each_wrong_field_is_named(self):
        cases = {
            "accessible": True, "pid": 1, "title": "other", "app_name": "other", "visible": False,
            "bounds": {"x": 0.0, "y": 0.0, "width": 640.0, "height": 480.0},
        }
        for key, value in cases.items():
            with self.subTest(key=key):
                problems = identity.compositor_entry_problems(dict(entry(), **{key: value}), foot(), PREFIX)
                self.assertTrue(any(f"{key} is" in problem for problem in problems), problems)

    def test_a_hidden_toplevel_must_list_as_not_visible(self):
        hidden = dict(foot(), hidden=True)
        problems = identity.compositor_entry_problems(entry(), hidden, PREFIX)
        self.assertTrue(any("visible is" in problem for problem in problems), problems)

    def test_an_id_of_another_compositor_fails(self):
        problems = identity.compositor_entry_problems(dict(entry(), id="w-hypr-1-1-3"), foot(), PREFIX)
        self.assertTrue(any("does not start with" in problem for problem in problems), problems)

    def test_a_missing_process_instance_minimized_or_ref_fails(self):
        for change in ({"process_instance": None}, {"minimized": False}, {"ref": "@s1:e1"},
                       {"snapshot_id": "s1"}):
            with self.subTest(change=change):
                self.assertNotEqual(identity.compositor_entry_problems(dict(entry(), **change), foot(), PREFIX), [])


class ListedOnceTests(unittest.TestCase):
    frames = ["w-1_9-org-a11y-atspi-accessible-1", "w-1_9-org-a11y-atspi-accessible-2"]

    def rows(self, *ids, accessible=True):
        return [{"id": item, "accessible": accessible} for item in ids]

    def test_each_frame_once_and_accessible_passes(self):
        self.assertEqual(identity.listed_once_problems("twins", self.rows(*self.frames), self.frames, 2), [])

    def test_a_duplicate_compositor_entry_fails(self):
        rows = self.rows(*self.frames) + [{"id": PREFIX + "7", "accessible": False}]
        problems = identity.listed_once_problems("twins", rows, self.frames, 2)
        self.assertTrue(any("compositor-only" in problem for problem in problems), problems)
        self.assertTrue(any("listed" in problem for problem in problems), problems)

    def test_a_missing_frame_or_a_frame_listed_twice_fails(self):
        for rows in (self.rows(self.frames[0]), self.rows(self.frames[0], *self.frames)):
            with self.subTest(rows=rows):
                self.assertNotEqual(identity.listed_once_problems("twins", rows, self.frames, 2), [])

    def test_a_frame_that_is_not_accessible_fails(self):
        problems = identity.listed_once_problems("twins", self.rows(*self.frames, accessible=False),
                                                 self.frames, 2)
        self.assertNotEqual(problems, [])

    def test_more_toplevels_than_frames_fails(self):
        self.assertNotEqual(identity.listed_once_problems("twins", self.rows(*self.frames), self.frames, 3), [])


class TwinSizeTests(unittest.TestCase):
    def test_native_wayland_twins_pass_and_xwayland_fails(self):
        self.assertEqual(identity.native_problems([twin("0x1", [400, 300])]), [])
        self.assertNotEqual(identity.native_problems([dict(twin("0x1", [400, 300]), xwayland=True)]), [])
        self.assertNotEqual(identity.native_problems([{"address": "0x1"}]), [])

    def test_same_size_needs_one_size_on_both_toplevels_and_frames(self):
        clients = [twin("0x1", [800, 600]), twin("0x2", [800, 600])]
        self.assertTrue(identity.same_size(clients, [(800, 600), (801, 599)]))
        self.assertFalse(identity.same_size(clients, [(800, 600), (803, 600)]))
        self.assertFalse(identity.same_size([twin("0x1", [800, 600]), twin("0x2", [400, 600])],
                                            [(800, 600), (400, 600)]))
        self.assertFalse(identity.same_size(clients[:1], [(800, 600)]))

    def test_distinct_sizes_need_each_frame_to_match_exactly_one_toplevel(self):
        clients = [twin("0x1", [800, 600]), twin("0x2", [400, 600])]
        self.assertTrue(identity.distinct_sizes(clients, [(400, 600), (800, 600)]))
        self.assertFalse(identity.distinct_sizes(clients, [(800, 600), (800, 600)]))
        self.assertFalse(identity.distinct_sizes(clients, [(400, 600), (600, 600)]))

    def test_sizes_within_twice_the_tolerance_are_not_distinct(self):
        clients = [twin("0x1", [800, 600]), twin("0x2", [802, 600])]
        self.assertFalse(identity.distinct_sizes(clients, [(800, 600), (802, 600)]))

    def test_twin_state_ignores_order_and_sees_a_move(self):
        a, b = twin("0x1", [800, 600]), twin("0x2", [800, 600], "special:ad_e2e_twin_b")
        self.assertEqual(identity.twin_state([a, b]), identity.twin_state([b, a]))
        moved = dict(b, workspace={"name": "1"})
        self.assertNotEqual(identity.twin_state([a, b]), identity.twin_state([a, moved]))
        self.assertNotEqual(identity.twin_state([a, b]), identity.twin_state([a, dict(b, hidden=True)]))


class CandidateTests(unittest.TestCase):
    expected = {"0x1": "special:ad_e2e_twin_a", "0x2": "special:ad_e2e_twin_b"}

    def ambiguous(self, candidates, **kw) -> Envelope:
        return refused("AMBIGUOUS_TARGET", candidates=candidates, **kw)

    def test_both_twins_with_their_workspaces_pass(self):
        candidates = [{"address": address, "workspace": name} for address, name in self.expected.items()]
        self.assertEqual(identity.candidate_problems("twin", self.ambiguous(candidates), self.expected), [])

    def test_a_wrong_workspace_missing_or_extra_candidate_fails(self):
        cases = (
            [{"address": "0x1", "workspace": "special:ad_e2e_twin_a"}],
            [{"address": "0x1", "workspace": "special:ad_e2e_twin_b"},
             {"address": "0x2", "workspace": "special:ad_e2e_twin_b"}],
            [{"address": "0x1", "workspace": "special:ad_e2e_twin_a"},
             {"address": "0x2", "workspace": "special:ad_e2e_twin_b"},
             {"address": "0x2", "workspace": "special:ad_e2e_twin_b"}],
        )
        for candidates in cases:
            with self.subTest(candidates=candidates):
                self.assertNotEqual(identity.candidate_problems("twin", self.ambiguous(candidates),
                                                                self.expected), [])

    def test_no_candidates_or_another_code_fails(self):
        self.assertNotEqual(identity.candidate_problems("twin", refused("AMBIGUOUS_TARGET"), self.expected), [])
        candidates = [{"address": address, "workspace": name} for address, name in self.expected.items()]
        self.assertNotEqual(identity.candidate_problems(
            "twin", refused("ACTION_FAILED", candidates=candidates), self.expected), [])

    def test_a_delivered_refusal_fails(self):
        candidates = [{"address": address, "workspace": name} for address, name in self.expected.items()]
        envelope = self.ambiguous(candidates, disposition={"delivery": "delivered_unverified", "retry": "unsafe"})
        self.assertNotEqual(identity.candidate_problems("twin", envelope, self.expected), [])


class NoTreeTests(unittest.TestCase):
    message = "'w-hypr-900-77-3' is a compositor window with no accessibility tree, so it has no elements"

    def test_invalid_args_naming_the_missing_tree_passes(self):
        self.assertEqual(identity.no_tree_problems("snapshot", refused("INVALID_ARGS", message=self.message)), [])

    def test_another_code_message_or_exit_fails(self):
        for envelope in (refused("WINDOW_NOT_FOUND", message=self.message),
                         refused("INVALID_ARGS", message="bad id"),
                         refused("INVALID_ARGS", message=self.message, exit_code=2)):
            with self.subTest(envelope=envelope.error):
                self.assertNotEqual(identity.no_tree_problems("snapshot", envelope), [])

    def test_a_tree_read_fails(self):
        envelope = Envelope(ok=True, data={"tree": {}}, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)
        self.assertNotEqual(identity.no_tree_problems("snapshot", envelope), [])


class TwinPlacementTests(unittest.TestCase):
    """Hyprland unregisters an exec rule on the first window its process maps, so the second twin
    needs the class rule, enabled only while the twins map."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.work = Path(directory.name)
        record = attached.new_record(slice_name="ad-e2e-1.slice", shared_runtime=str(self.work),
                                     signature="sig_1", mirror=self.work / "missing" / "mirror.json")
        attached.save(self.work, record)
        self.tier = SimpleNamespace(runtime_dir=self.work)
        self.calls = []
        self.workspaces = ["special:ad_e2e", "special:ad_e2e"]

    def control(self, runtime, signature, argv, **_):
        self.calls.append(argv)
        return "ok"

    def clients(self, runtime, signature, name, **_):
        self.assertEqual(name, "clients")
        bystander = {"address": "0x1", "class": "foot", "workspace": {"name": "1"}}
        if not any(argv[0] == "dispatch" for argv in self.calls):
            return [bystander]
        return [bystander] + [{"address": f"0x{index + 2}", "class": "twin_windows.py",
                               "workspace": {"name": name}} for index, name in enumerate(self.workspaces)]

    def spawn(self, lua: bool = True):
        with mock.patch.object(hyprland, "uses_lua", return_value=lua), \
                mock.patch.object(hyprland, "control", side_effect=self.control), \
                mock.patch.object(hyprland, "query", side_effect=self.clients):
            attached_session.spawn(self.tier, ["python3", "twin_windows.py"], {}, self.work / "log",
                                   toplevels=apps.TOPLEVELS["Twin Windows"], timeout_s=0.2)

    def test_the_twin_app_maps_two_toplevels_of_its_program_class(self):
        source = (ROOT / "apps" / "twin_windows.py").read_text()
        self.assertEqual(apps.TOPLEVELS["Twin Windows"], ("twin_windows.py", 2))
        self.assertIn('GLib.set_prgname("twin_windows.py")', source)
        self.assertIn("range(2)", source)

    def test_the_class_rule_is_enabled_before_the_exec_and_disabled_after_both_twins_map(self):
        self.spawn()
        self.assertEqual([argv[0] for argv in self.calls], ["eval", "dispatch", "eval"])
        self.assertIn("enabled = true", self.calls[0][1])
        self.assertIn('class = "^twin_windows[.]py$"', self.calls[0][1])
        self.assertIn("enabled = false", self.calls[2][1])
        self.assertEqual(attached.load(self.work)["rules"], [])

    def test_a_twin_on_the_active_workspace_fails_and_the_rule_is_still_disabled(self):
        self.workspaces = ["special:ad_e2e", "1"]
        with self.assertRaisesRegex(RuntimeError, "outside special:ad_e2e"):
            self.spawn()
        self.assertIn("enabled = false", self.calls[-1][1])
        self.assertEqual(attached.load(self.work)["rules"], [])

    def test_a_missing_twin_times_out_and_the_rule_is_still_disabled(self):
        self.workspaces = ["special:ad_e2e"]
        with self.assertRaisesRegex(RuntimeError, "1 of 2"):
            self.spawn()
        self.assertIn("enabled = false", self.calls[-1][1])

    def test_a_legacy_config_refuses_before_it_starts_anything(self):
        with self.assertRaisesRegex(RuntimeError, "legacy"):
            self.spawn(lua=False)
        self.assertEqual(self.calls, [])
        self.assertNotIn("rules", attached.load(self.work))

    def test_teardown_disables_a_rule_the_run_left_enabled_and_reports_one_it_could_not(self):
        record = attached.load(self.work)
        record["rules"] = [attached_session.RULE]
        attached.save(self.work, record)
        with mock.patch.object(hyprland, "disable_rule", side_effect=RuntimeError("gone")):
            self.assertEqual(len(attached_session._disable_rules(self.work)), 1)
        self.assertEqual(attached.load(self.work)["rules"], [attached_session.RULE])
        with mock.patch.object(hyprland, "disable_rule") as disable:
            self.assertEqual(attached_session._disable_rules(self.work), [])
        disable.assert_called_once_with(self.work, "sig_1", attached_session.RULE)
        self.assertEqual(attached.load(self.work)["rules"], [])


class TwinPhaseReadTests(unittest.TestCase):
    def test_two_twin_phases_leave_every_oracle_with_one_before_after_and_restored_read(self):
        tier = SimpleNamespace(runtime_dir=Path("/nonexistent"))
        ctx = LegContext(tier, "headless", legs.leg_by_id("identity.hyprland_twins_by_size_or_ambiguous"),
                         member_reader=lambda _: frozenset())
        twins = SimpleNamespace(pid=4242)
        ctx.before("hyprland.desktop", {"workspace": "1"})
        with mock.patch.object(identity, "clients_of", return_value=[twin("0x1", [400, 300])]):
            problems = identity._unchanged_twins(ctx, twins, lambda: [])
            problems += identity._unchanged_twins(ctx, twins, lambda: [])
        ctx.after("hyprland.desktop", {"workspace": "1"})
        ctx.restored("hyprland.desktop", {"workspace": "1"})
        self.assertEqual(problems, [])
        self.assertEqual(evaluate(ctx), [])

    def test_a_twin_that_changes_during_a_phase_is_a_problem(self):
        tier = SimpleNamespace(runtime_dir=Path("/nonexistent"))
        ctx = LegContext(tier, "headless", member_reader=lambda _: frozenset())
        states = iter([[twin("0x1", [400, 300])], [twin("0x1", [400, 300], "1")]])
        with mock.patch.object(identity, "clients_of", side_effect=lambda *_: next(states)):
            problems = identity._unchanged_twins(ctx, SimpleNamespace(pid=4242), lambda: [])
        self.assertEqual(len(problems), 1)


if __name__ == "__main__":
    unittest.main()
