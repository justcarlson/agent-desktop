from __future__ import annotations

import contextlib
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import hyprland_desktop, hyprland_focus, hyprland_focus_controls as controls

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
FOCUS_LEGS = (
    "focus.hyprland_other_workspace",
    "focus.hyprland_twin_after_sizes_differ",
    "focus.hyprland_same_size_twins_ambiguous",
    "focus.hyprland_closed_window_not_found",
)


def envelope(ok: bool, *, code: str | None = None, exit_code: int = 1, disposition=None, **details) -> Envelope:
    if ok:
        return Envelope(ok=True, data={"focused": {}}, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)
    error = {"code": code, "message": "refused", "details": details,
             "disposition": NOT_DELIVERED if disposition is None else disposition}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=exit_code)


def toplevel(address: str, size: tuple[int, int], workspace: str = "31") -> dict:
    return {"address": address, "size": list(size), "workspace": {"name": workspace}, "pid": 40}


class FocusRegistryTests(unittest.TestCase):
    def test_every_focus_leg_is_headed_hyprland_reads_the_compositor_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in FOCUS_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headed",), "pass"))
                self.assertIn("hyprland", leg.oracles)
                self.assertIn(leg_id, implemented)


class ControlArgvTests(unittest.TestCase):
    def test_each_control_has_a_lua_and_a_classic_form(self):
        self.assertEqual(controls.focus_argv("0xa1", True), ["dispatch", 'hl.dsp.focus({window="address:0xa1"})'])
        self.assertEqual(controls.focus_argv("0xa1", False), ["dispatch", "focuswindow", "address:0xa1"])
        self.assertEqual(controls.move_argv("0xa1", "31", True),
                         ["dispatch", 'hl.dsp.window.move({workspace="31",follow=false,window="address:0xa1"})'])
        self.assertEqual(controls.move_argv("0xa1", "31", False),
                         ["dispatch", "movetoworkspacesilent", "31,address:0xa1"])
        self.assertEqual(controls.toggle_argv("special:ad_e2e", True),
                         ["dispatch", 'hl.dsp.workspace.toggle_special("ad_e2e")'])
        self.assertEqual(controls.toggle_argv("special:ad_e2e", False),
                         ["dispatch", "togglespecialworkspace", "ad_e2e"])
        self.assertEqual(controls.cursor_argv(12.6, 30.0, True), ["dispatch", "hl.dsp.cursor.move({x=12,y=30})"])
        self.assertEqual(controls.cursor_argv(12.6, 30.0, False), ["dispatch", "movecursor", "12", "30"])

    def test_a_control_refuses_text_it_cannot_embed(self):
        for build in (lambda lua: controls.focus_argv('0xa1") os.exit(', lua),
                      lambda lua: controls.move_argv("0xa1", "31;rm", lua)):
            for lua in (True, False):
                with self.subTest(lua=lua), self.assertRaises(ValueError):
                    build(lua)

    def test_spare_workspaces_skip_every_workspace_in_use(self):
        self.assertEqual(controls.spare_workspaces({"1", "31", "special:ad_e2e"}, 2), ["32", "33"])
        with self.assertRaises(AssertionError):
            controls.spare_workspaces(set(controls.SPARE_WORKSPACES), 1)


class SizeBindingTests(unittest.TestCase):
    def test_same_size_needs_both_toplevels_and_frames_at_one_size(self):
        pair = [toplevel("0xa", (800, 600)), toplevel("0xb", (800, 600))]
        self.assertTrue(controls.same_size(pair, {"w-1": (800, 600), "w-2": (801, 600)}))
        self.assertFalse(controls.same_size(pair, {"w-1": (800, 600), "w-2": (400, 600)}))
        self.assertFalse(controls.same_size([pair[0], toplevel("0xb", (400, 600))],
                                            {"w-1": (800, 600), "w-2": (400, 600)}))

    def test_distinct_sizes_bind_each_frame_to_its_own_toplevel(self):
        pair = [toplevel("0xa", (800, 600)), toplevel("0xb", (400, 600))]
        self.assertEqual(controls.bound_by_size(pair, {"w-1": (400, 600), "w-2": (800, 600)}),
                         {"w-1": "0xb", "w-2": "0xa"})

    def test_a_frame_that_matches_no_toplevel_or_both_binds_nothing(self):
        pair = [toplevel("0xa", (800, 600)), toplevel("0xb", (400, 600))]
        self.assertIsNone(controls.bound_by_size(pair, {"w-1": (400, 600), "w-2": (600, 600)}))
        self.assertIsNone(controls.bound_by_size(pair, {"w-1": (400, 600), "w-2": (400, 600)}))
        same = [toplevel("0xa", (800, 600)), toplevel("0xb", (800, 600))]
        self.assertIsNone(controls.bound_by_size(same, {"w-1": (800, 600), "w-2": (800, 600)}))


class FocusVerdictTests(unittest.TestCase):
    GOOD = {"active": "0xa", "active_pid": 40, "pid": 40, "workspace": "31", "in_view": True}

    def test_the_compositor_must_show_the_toplevel_active_on_its_workspace_and_in_view(self):
        self.assertEqual(hyprland_focus.focus_problems("t", self.GOOD, "0xa", "31"), [])
        for name, change in {"active": {"active": "0xb"}, "pid": {"active_pid": 41},
                             "workspace": {"workspace": "32"}, "view": {"in_view": False}}.items():
            with self.subTest(name=name):
                self.assertTrue(hyprland_focus.focus_problems("t", {**self.GOOD, **change}, "0xa", "31"))

    def focused(self, answer: Envelope, view: dict) -> list[str]:
        ctx = SimpleNamespace(tier=None, product=lambda argv: answer, after=lambda name, value: None)
        with mock.patch.object(hyprland_focus, "focused_view", return_value=view), \
                mock.patch.object(hyprland_focus, "FOCUS_SETTLE_S", 0.0):
            return hyprland_focus.focused(ctx, "t", "w-1", "0xa", "31")

    def test_the_verdict_comes_from_the_compositor_not_the_command(self):
        wrong = {**self.GOOD, "active": "0xb"}
        self.assertTrue(self.focused(envelope(True), wrong))
        self.assertEqual(self.focused(envelope(False, code="ACTION_FAILED"), self.GOOD), [])

    def test_ambiguous_twins_name_every_toplevel_and_its_workspace(self):
        expected = {"0xa": "31", "0xb": "32"}
        good = envelope(False, code="AMBIGUOUS_TARGET",
                        candidates=[{"address": "0xa", "workspace": "31"}, {"address": "0xb", "workspace": "32"}])
        self.assertEqual(hyprland_focus.candidate_problems("t", good, expected), [])
        for name, answer in {
            "one": envelope(False, code="AMBIGUOUS_TARGET", candidates=[{"address": "0xa", "workspace": "31"}]),
            "workspace": envelope(False, code="AMBIGUOUS_TARGET",
                                  candidates=[{"address": "0xa", "workspace": "31"},
                                              {"address": "0xb", "workspace": "31"}]),
            "missing": envelope(False, code="AMBIGUOUS_TARGET"),
            "code": envelope(False, code="WINDOW_NOT_FOUND", candidates=good.error["details"]["candidates"]),
            "ok": envelope(True),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(hyprland_focus.candidate_problems("t", answer, expected))


class HeadlessFocusRefusalTests(unittest.TestCase):
    def test_the_expected_cause_follows_the_compositor_placement(self):
        self.assertEqual(hyprland_desktop.expected_focus_refusal("special:ad_e2e", {"1", ""}),
                         ("hidden_special_workspace", "special:ad_e2e"))
        self.assertEqual(hyprland_desktop.expected_focus_refusal("special:ad_e2e", {"1", "special:ad_e2e"}),
                         ("window_not_focused", None))
        self.assertEqual(hyprland_desktop.expected_focus_refusal("3", {"1", ""}), ("window_not_focused", None))

    def problems(self, answer: Envelope) -> list[str]:
        reads = {"clients": [{"pid": 40, "workspace": {"name": "special:ad_e2e"}}],
                 "monitors": [{"activeWorkspace": {"name": "1"}, "specialWorkspace": {"name": ""}}]}
        ctx = SimpleNamespace(tier=None, product=lambda argv: answer)
        app = SimpleNamespace(pid=40, windows=["w-1"])
        with mock.patch.object(hyprland_desktop, "main_instance", return_value={"signature": "s", "pid": 1}), \
                mock.patch.object(hyprland_desktop, "shared", return_value=Path("/run")), \
                mock.patch.object(hyprland_desktop.hyprland, "query", side_effect=lambda r, s, name, pid: reads[name]):
            return hyprland_desktop.headless_focus_problems(ctx, app)

    def test_a_hidden_special_window_is_refused_by_name_without_delivery(self):
        good = envelope(False, code="POLICY_DENIED", cause="hidden_special_workspace",
                        workspace="special:ad_e2e", physical_delivery_started=False)
        self.assertEqual(self.problems(good), [])
        for name, answer in {
            "cause": envelope(False, code="POLICY_DENIED", cause="window_not_focused",
                              physical_delivery_started=False),
            "workspace": envelope(False, code="POLICY_DENIED", cause="hidden_special_workspace",
                                  workspace="special:other", physical_delivery_started=False),
            "activated": envelope(True),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(self.problems(answer))

    def test_headed_refusals_no_longer_send_focus_window(self):
        sent = []
        gate = envelope(False, code="ACTION_NOT_SUPPORTED", cause="wayland_session", physical_delivery_started=False)
        ctx = SimpleNamespace(product=lambda argv: sent.append(argv[0]) or gate)
        self.assertEqual(hyprland_desktop._headed_refusals(ctx, "@s1:e1"), [])
        self.assertNotIn("focus-window", sent)


class FocusLegAccountingTests(unittest.TestCase):
    """Each focus leg, run against a fake compositor, satisfies the runner's oracle accounting."""

    HOLDER = {"address": "0xby", "pid": 1}
    DESKTOP = {"active_window": HOLDER}

    def run_leg(self, leg_id: str) -> list[str]:
        places = {}
        calculator = SimpleNamespace(pid=50, windows=["w-calc"])
        twins = SimpleNamespace(pid=70, windows=["w-a", "w-b"])
        pair = [toplevel("0xa", (800, 600)), toplevel("0xb", (800, 600))]

        @contextlib.contextmanager
        def yielding(value):
            yield value

        def clients_of(tier, pid):
            if pid == twins.pid:
                return pair
            if pid == calculator.pid and leg_id != "focus.hyprland_closed_window_not_found":
                return [{"address": "0xcalc"}]
            return []

        def view(tier, address):
            return {"active": address, "active_pid": 1, "pid": 1, "workspace": places.get(address),
                    "in_view": True}

        def product(argv):
            if leg_id == "focus.hyprland_closed_window_not_found":
                return envelope(False, code="WINDOW_NOT_FOUND")
            return envelope(True)

        ctx = LegContext(SimpleNamespace(), "headed", leg=legs.leg_by_id(leg_id), member_reader=lambda tier: frozenset())
        ctx.product = product
        sampler = SimpleNamespace(changes=[])
        patches = {
            "main_instance": mock.Mock(return_value={}),
            "bystander": mock.Mock(return_value=self.HOLDER),
            "cursor": mock.Mock(return_value={"x": 0, "y": 0}),
            "desktop": mock.Mock(side_effect=lambda tier, main: dict(self.DESKTOP)),
            "keeps_focus": mock.Mock(return_value=[]),
            "restore_desktop": mock.Mock(),
            "managed": mock.Mock(side_effect=lambda ctx, name: yielding(calculator)),
            "terminal": mock.Mock(side_effect=lambda tier, app_id: yielding(
                (SimpleNamespace(pid=60), {"address": "0xfoot"}))),
            "focus_twins": mock.Mock(side_effect=lambda tier: yielding((twins, pair))),
            "clients_of": mock.Mock(side_effect=clients_of),
            "listed_ids": mock.Mock(return_value=["w-foot"]),
            "spare_workspaces": mock.Mock(return_value=["31", "32"]),
            "workspaces_in_use": mock.Mock(return_value=set()),
            "move": mock.Mock(side_effect=lambda tier, address, workspace: places.__setitem__(address, workspace)),
            "frame_sizes": mock.Mock(return_value={}),
            "bound_by_size": mock.Mock(return_value={"w-a": "0xa", "w-b": "0xb"}),
            "same_size": mock.Mock(return_value=True),
            "focused_view": mock.Mock(side_effect=view),
            "_unfocused": mock.Mock(),
            "candidate_problems": mock.Mock(return_value=[]),
            "boundary_problems": mock.Mock(return_value=[]),
            "Sampler": mock.Mock(return_value=mock.MagicMock(
                __enter__=mock.Mock(return_value=sampler))),
        }
        with contextlib.ExitStack() as stack:
            for name, value in patches.items():
                stack.enter_context(mock.patch.object(hyprland_focus, name, value))
            stack.enter_context(mock.patch.object(hyprland_focus.raw, "poll", lambda check, timeout: check()))
            stack.enter_context(mock.patch.object(hyprland_focus, "FOCUS_SETTLE_S", 0.0))
            legs.implementations()[leg_id](ctx)
        return evaluate(ctx)

    def test_every_recorded_oracle_has_one_before_after_and_restored_read(self):
        import scenarios  # noqa: F401

        for leg_id in FOCUS_LEGS:
            with self.subTest(leg=leg_id):
                self.assertEqual(self.run_leg(leg_id), [])


if __name__ == "__main__":
    unittest.main()
