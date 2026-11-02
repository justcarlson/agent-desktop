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
from scenarios import hyprland_focus_reports as proof

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
PROOF_LEGS = {
    "focus.hyprland_activation_proof_reported": ("hyprland", "atspi"),
    "focus.hyprland_in_view_no_invented_switch": ("hyprland",),
    "focus.hyprland_special_reveal_and_close_reported": ("hyprland",),
}
SPECIAL_A, SPECIAL_B = proof.SPECIALS


def monitor(active: str, special: str = "", name: str = "Virtual-1") -> dict:
    return {"name": name, "activeWorkspace": {"name": active}, "specialWorkspace": {"name": special}}


def scene(*monitors: dict, cursor=(640, 413)) -> dict:
    return {"monitors": list(monitors), "cursor": {"x": cursor[0], "y": cursor[1]}}


def focused(activated: bool, proof_name: str, effects: list[dict] | None = None) -> Envelope:
    activation = {"activated": activated, "proof": proof_name}
    if effects:
        activation["side_effects"] = effects
    data = {"focused": {"id": "w-1"}, "activation": activation}
    return Envelope(ok=True, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)


def refused(code: str, **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details, "disposition": NOT_DELIVERED}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def switched(to: str, was: str = "1") -> dict:
    return {"kind": "workspace_switched", "monitor": "Virtual-1", "from": was, "to": to}


def revealed(workspace: str) -> dict:
    return {"kind": "special_workspace_revealed", "monitor": "Virtual-1", "workspace": workspace}


def closed(workspace: str) -> dict:
    return {"kind": "special_workspace_closed", "monitor": "Virtual-1", "workspace": workspace}


class ProofRegistryTests(unittest.TestCase):
    def test_every_proof_leg_is_headed_hyprland_reads_what_it_declares_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id, oracles in PROOF_LEGS.items():
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headed",), "pass"))
                self.assertEqual(leg.oracles, oracles)
                self.assertIn(leg_id, implemented)


class ExpectedEffectTests(unittest.TestCase):
    def test_nothing_changed_gives_no_effect(self):
        self.assertEqual(proof.expected_effects(scene(monitor("1")), scene(monitor("1"))), [])

    def test_a_regular_workspace_change_is_a_switch_on_that_monitor(self):
        self.assertEqual(proof.expected_effects(scene(monitor("1")), scene(monitor("31"))), [switched("31")])

    def test_a_special_workspace_shown_or_swapped_is_named_closed_then_revealed(self):
        self.assertEqual(proof.expected_effects(scene(monitor("1")), scene(monitor("1", SPECIAL_A))),
                         [revealed(SPECIAL_A)])
        self.assertEqual(proof.expected_effects(scene(monitor("1", SPECIAL_A)), scene(monitor("1", SPECIAL_B))),
                         [closed(SPECIAL_A), revealed(SPECIAL_B)])
        self.assertEqual(proof.expected_effects(scene(monitor("1", SPECIAL_B)), scene(monitor("1"))),
                         [closed(SPECIAL_B)])

    def test_a_cursor_move_comes_last_in_float_points(self):
        moved = proof.expected_effects(scene(monitor("1"), cursor=(1, 2)), scene(monitor("31"), cursor=(3, 4)))
        self.assertEqual(moved, [switched("31"),
                                 {"kind": "cursor_moved", "from": {"x": 1.0, "y": 2.0}, "to": {"x": 3.0, "y": 4.0}}])

    def test_a_monitor_seen_on_one_side_only_is_not_diffed(self):
        before = scene(monitor("1"))
        after = scene(monitor("1"), monitor("5", SPECIAL_A, name="AD-E2E-1"))
        self.assertEqual(proof.expected_effects(before, after), [])

    def test_landed_keeps_only_workspace_effects_by_where_they_land(self):
        effects = [switched("31"), closed(SPECIAL_A), revealed(SPECIAL_B),
                   {"kind": "cursor_moved", "from": {"x": 1, "y": 2}, "to": {"x": 3, "y": 4}}]
        self.assertEqual(proof.landed(effects), [("workspace_switched", "31"),
                                                 ("special_workspace_closed", SPECIAL_A),
                                                 ("special_workspace_revealed", SPECIAL_B)])


class ActivationReportTests(unittest.TestCase):
    EFFECTS = [switched("31"), {"kind": "cursor_moved", "from": {"x": 1.0, "y": 2.0}, "to": {"x": 3.0, "y": 4.0}}]

    def test_the_report_must_name_the_proof_the_activation_and_every_effect_the_compositor_showed(self):
        good = focused(True, proof.PROOF_COMPOSITOR,
                       [switched("31"), {"kind": "cursor_moved", "from": {"x": 1, "y": 2}, "to": {"x": 3, "y": 4}}])
        self.assertEqual(proof.activation_problems("t", good, proof.PROOF_COMPOSITOR, True, self.EFFECTS), [])
        cases = {
            "proof": focused(True, proof.PROOF_BOTH, self.EFFECTS),
            "activated": focused(False, proof.PROOF_COMPOSITOR, self.EFFECTS),
            "missing effect": focused(True, proof.PROOF_COMPOSITOR, self.EFFECTS[:1]),
            "invented effect": focused(True, proof.PROOF_COMPOSITOR, self.EFFECTS + [revealed(SPECIAL_A)]),
            "no effects": focused(True, proof.PROOF_COMPOSITOR),
            "refused": refused("ACTION_FAILED", cause="activation_refused"),
        }
        for name, answer in cases.items():
            with self.subTest(case=name):
                self.assertTrue(proof.activation_problems("t", answer, proof.PROOF_COMPOSITOR, True, self.EFFECTS))

    def test_an_already_focused_window_reports_no_activation_and_no_effects(self):
        self.assertEqual(proof.activation_problems("t", focused(False, proof.PROOF_COMPOSITOR),
                                                   proof.PROOF_COMPOSITOR, False, []), [])
        self.assertTrue(proof.activation_problems("t", focused(True, proof.PROOF_COMPOSITOR),
                                                  proof.PROOF_COMPOSITOR, False, []))

    def test_a_frame_must_gain_active_from_a_read_without_it(self):
        self.assertEqual(proof.frame_problems("t", frozenset({"VISIBLE"}), frozenset({"VISIBLE", "ACTIVE"})), [])
        self.assertTrue(proof.frame_problems("t", frozenset({"ACTIVE"}), frozenset({"ACTIVE"})))
        self.assertTrue(proof.frame_problems("t", frozenset(), frozenset({"VISIBLE"})))

    def test_held_needs_the_active_toplevel_its_pid_and_its_workspace_shown_without_cover(self):
        view = {"active": "0xa", "active_pid": 7, "pid": 7, "workspace": SPECIAL_A,
                "shown": {"Virtual-1": ("1", SPECIAL_A)}}
        self.assertEqual(proof.held_problems("t", view, "0xa", SPECIAL_A), [])
        regular = {**view, "workspace": "31", "shown": {"Virtual-1": ("31", "")}}
        self.assertEqual(proof.held_problems("t", regular, "0xa", "31"), [])
        for name, change in {"active": {"active": "0xb"}, "pid": {"active_pid": 8},
                             "workspace": {"workspace": SPECIAL_B},
                             "hidden": {"shown": {"Virtual-1": ("1", "")}}}.items():
            with self.subTest(case=name):
                self.assertTrue(proof.held_problems("t", {**view, **change}, "0xa", SPECIAL_A))
        covered = {**regular, "shown": {"Virtual-1": ("31", SPECIAL_A)}}
        self.assertTrue(proof.held_problems("t", covered, "0xa", "31"))

    def test_a_window_under_an_open_special_workspace_is_refused_by_name_before_activation(self):
        good = refused("ACTION_FAILED", cause="special_workspace_open", workspace=SPECIAL_A,
                       monitor="Virtual-1", physical_delivery_started=False)
        self.assertEqual(proof.special_open_problems("t", good, SPECIAL_A), [])
        for name, answer in {
            "workspace": refused("ACTION_FAILED", cause="special_workspace_open", workspace=SPECIAL_B,
                                 physical_delivery_started=False),
            "cause": refused("ACTION_FAILED", cause="activation_refused", workspace=SPECIAL_A,
                             physical_delivery_started=False),
            "focused": focused(True, proof.PROOF_COMPOSITOR),
        }.items():
            with self.subTest(case=name):
                self.assertTrue(proof.special_open_problems("t", answer, SPECIAL_A))


class ProofLegAccountingTests(unittest.TestCase):
    """Each proof leg, run against a fake compositor that does what the plan says, passes the runner's
    oracle accounting; a report that drops an effect fails it."""

    HOLDER = {"address": "0xby", "pid": 1}

    def run_leg(self, leg_id: str, drop_effects: bool = False) -> tuple[list[str], list[Envelope]]:
        sent: list[Envelope] = []
        state = {"active": "0xby", "regular": "1", "special": "", "places": {"0xby": "1"}}
        frames = {"w-calc": frozenset({"VISIBLE"})}
        counter = iter(range(100, 200))
        calculator = SimpleNamespace(pid=50, windows=["w-calc"], owner_names=[":1.5"])

        @contextlib.contextmanager
        def terminal(tier, app_id):
            pid = next(counter)
            address = f"0x{pid}"
            state["places"][address] = "special:ad_e2e"
            yield SimpleNamespace(pid=pid), {"address": address}

        @contextlib.contextmanager
        def managed(ctx, name):
            state["places"]["0xcalc"] = "special:ad_e2e"
            yield calculator

        def scene(tier):
            return {"monitors": [monitor(state["regular"], state["special"])], "cursor": {"x": 0, "y": 0}}

        def view(tier, address):
            return {"active": state["active"], "active_pid": 1, "pid": 1, "workspace": state["places"].get(address),
                    "shown": {"Virtual-1": (state["regular"], state["special"])}}

        def product(argv):
            answer = answer_for(argv)
            if argv[0] == "focus-window":
                sent.append(answer)
            return answer

        def answer_for(argv):
            if argv[0] == "list-windows":
                return Envelope(ok=True, data=[], error=None, raw="[]", elapsed_ms=1.0, argv=argv, exit_code=0)
            address = "0xcalc" if argv[-1] == "w-calc" else "0x" + argv[-1].removeprefix("w-")
            workspace = state["places"][address]
            if state["special"] and not workspace.startswith("special:"):
                return refused("ACTION_FAILED", cause="special_workspace_open", workspace=state["special"],
                               physical_delivery_started=False)
            before = scene(None)
            already = state["active"] == address
            if workspace.startswith("special:"):
                state["special"] = workspace
            else:
                state["special"] = ""
                state["regular"] = workspace
            state["active"] = address
            if address == "0xcalc":
                frames["w-calc"] = frozenset({"VISIBLE", "ACTIVE"})
            effects = [] if drop_effects else proof.expected_effects(before, scene(None))
            kind = proof.PROOF_BOTH if address == "0xcalc" else proof.PROOF_COMPOSITOR
            return focused(not already, kind, effects)

        def move(tier, address, workspace):
            state["places"][address] = workspace

        def listed(envelope, pid):
            return [f"w-{pid}"]

        def restore(tier, holder, point):
            state.update(active="0xby", regular="1", special="")

        ctx = LegContext(SimpleNamespace(), "headed", leg=legs.leg_by_id(leg_id), member_reader=lambda tier: frozenset())
        ctx.product = product
        patches = {
            "main_instance": mock.Mock(return_value={}),
            "bystander": mock.Mock(return_value=self.HOLDER),
            "cursor": mock.Mock(return_value={"x": 0, "y": 0}),
            "desktop": mock.Mock(side_effect=lambda tier, main: {"active": state["active"],
                                                                 "special": state["special"]}),
            "restore_desktop": mock.Mock(side_effect=restore),
            "managed": mock.Mock(side_effect=managed),
            "terminal": mock.Mock(side_effect=terminal),
            "clients_of": mock.Mock(return_value=[{"address": "0xcalc"}]),
            "client": mock.Mock(side_effect=lambda tier, address: {
                "address": address, "workspace": {"name": state["places"][address]}}),
            "listed_ids": mock.Mock(side_effect=listed),
            "spare_workspaces": mock.Mock(return_value=["31", "32"]),
            "workspaces_in_use": mock.Mock(return_value=set()),
            "move": mock.Mock(side_effect=move),
            "scene": mock.Mock(side_effect=scene),
            "focus_view": mock.Mock(side_effect=view),
            "frame_states": mock.Mock(side_effect=lambda tier, app, window_id: frames[window_id]),
        }
        with contextlib.ExitStack() as stack:
            for name, value in patches.items():
                stack.enter_context(mock.patch.object(proof, name, value))
            stack.enter_context(mock.patch.object(proof.raw, "poll", lambda check, timeout: check()))
            stack.enter_context(mock.patch.object(proof, "FOCUS_SETTLE_S", 0.0))
            implementation = legs.implementations()[leg_id]
            try:
                implementation(ctx)
            except AssertionError as error:
                return [str(error)], sent
        return evaluate(ctx), sent

    def test_every_leg_passes_against_a_compositor_that_does_what_the_plan_says(self):
        import scenarios  # noqa: F401

        for leg_id in PROOF_LEGS:
            with self.subTest(leg=leg_id):
                self.assertEqual(self.run_leg(leg_id)[0], [])

    def test_a_report_that_drops_the_compositor_effects_fails_the_legs_that_cause_them(self):
        import scenarios  # noqa: F401

        for leg_id in ("focus.hyprland_activation_proof_reported", "focus.hyprland_special_reveal_and_close_reported"):
            with self.subTest(leg=leg_id):
                self.assertNotEqual(self.run_leg(leg_id, drop_effects=True)[0], [])

    def test_the_special_leg_reveals_swaps_returns_and_is_refused_under_the_open_special(self):
        import scenarios  # noqa: F401

        problems, answers = self.run_leg("focus.hyprland_special_reveal_and_close_reported")
        self.assertEqual(problems, [])
        kinds = [proof.landed((envelope.data or {}).get("activation", {}).get("side_effects", []))
                 for envelope in answers if envelope.ok]
        self.assertEqual(kinds, [[("special_workspace_revealed", SPECIAL_A)],
                                 [("special_workspace_closed", SPECIAL_A), ("special_workspace_revealed", SPECIAL_B)],
                                 [("special_workspace_closed", SPECIAL_B), ("special_workspace_revealed", SPECIAL_A)]])
        self.assertEqual([envelope.error["details"]["cause"] for envelope in answers if not envelope.ok],
                         ["special_workspace_open"])


if __name__ == "__main__":
    unittest.main()
