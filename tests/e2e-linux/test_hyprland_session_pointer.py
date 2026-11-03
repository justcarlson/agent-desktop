import contextlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from hyprland_leg_fakes import CLIENT, GONE, FakeContext
from runner import legs
from runner.product import Envelope
from runner.verdict import evaluate
from scenarios import hyprland_pointer as pointer
from scenarios import hyprland_pointer_ref_refusals as ref_refusals
from scenarios import hyprland_session_pointer as session

MAIN = {"signature": "main", "pid": 10, "wayland": "wayland-1"}
SAFE = {"delivery": "not_delivered", "retry": "safe"}
UNPROVEN = {"code": "ACTION_FAILED", "message": "no receiver proof; retry without --headed or with --xy",
            "suggestion": "Run it without --headed, or use --xy", "disposition": SAFE,
            "details": {"kind": "receiver_evidence", "check": "hyprland_stack", "cause": "receiver_unproven",
                        "physical_delivery_started": False}}
HOVER = {"code": "ACTION_NOT_SUPPORTED", "message": "Hover by ref has no receiver evidence on Hyprland",
         "disposition": SAFE,
         "details": {"kind": "hyprland_pointer", "cause": "hover_unsupported", "physical_delivery_started": False}}
POLICY = {"code": "POLICY_DENIED", "message": "action requires cursor movement but policy denies it",
          "disposition": SAFE}
MOTION = {"ev": "motion", "widget": "area", "wx": 280.0, "wy": 315.0, "t": 1.0}
SESSION_LEGS = ("pointer.session_ref_pointer_refusals", "pointer.session_hover_coordinates",
                "pointer.ref_receiver_evidence_under_cursor", "pointer.cursor_pass_through")


def refused(error: dict) -> Envelope:
    return Envelope(ok=False, data=None, error=error, raw=json.dumps({"error": error}), elapsed_ms=40.0,
                    argv=["agent-desktop"], exit_code=1)


def hovered(point: tuple[int, int]) -> Envelope:
    data = {"x": point[0], "y": point[1], "disposition": {"delivery": "delivered_unverified"}}
    return Envelope(ok=True, data=data, error=None, raw=json.dumps({"data": data}), elapsed_ms=40.0,
                    argv=["agent-desktop"], exit_code=0)


def headed_answer(argv: list[str]) -> Envelope:
    return refused(HOVER if argv[0] == "hover" else UNPROVEN)


class JudgeTests(unittest.TestCase):
    def test_a_headless_refusal_without_a_cause_passes(self):
        self.assertEqual(session.exact(session.headless_problems)("drag", refused(POLICY)), [])

    def test_a_headless_refusal_that_names_a_cause_fails(self):
        error = {**POLICY, "details": {"cause": "receiver_unproven"}}
        problems = session.headless_problems("drag", refused(error))
        self.assertTrue(any("receiver_unproven" in problem for problem in problems), problems)

    def test_a_headless_refusal_of_another_code_fails(self):
        self.assertTrue(session.headless_problems("drag", refused(UNPROVEN)))

    def test_a_refusal_with_unsafe_or_missing_retry_fails(self):
        for disposition in ({"delivery": "not_delivered", "retry": "unsafe"}, {"delivery": "not_delivered"}):
            with self.subTest(disposition=disposition):
                envelope = refused({**UNPROVEN, "disposition": disposition})
                self.assertEqual(session.unproven_problems("scroll", envelope), [])
                self.assertEqual(len(session.exact(session.unproven_problems)("scroll", envelope)), 1)

    def test_headed_actions_send_every_ref_pointer_verb_with_its_judge(self):
        actions = session.headed_actions("@s:e1", "@s:e2", CLIENT)
        verbs = [argv[0] for argv, _ in actions]
        self.assertEqual(verbs, ["drag", "drag", "drag", "scroll", "double-click", "triple-click", "hover"])
        judges = {argv[0]: judge for argv, judge in actions}
        self.assertIs(judges["hover"], session.ref_refusal_problems)
        self.assertIs(judges["scroll"], session.unproven_problems)
        self.assertEqual([argv for argv, _ in actions[:3]], ref_refusals.drag_argvs("@s:e1", "@s:e2", CLIENT))

    def test_headless_actions_leave_scroll_and_multi_click_to_the_shared_headless_leg(self):
        actions = session.headless_actions("@s:e1", "@s:e2", CLIENT)
        self.assertEqual([argv[0] for argv, _ in actions], ["drag", "drag", "drag", "hover"])
        self.assertEqual({judge for _, judge in actions}, {session.headless_problems})


class HoverPadTests(unittest.TestCase):
    def test_motion_that_ends_on_the_aim_passes(self):
        self.assertEqual(session.pad_hover_problems([MOTION], CLIENT, (380, 415)), [])

    def test_motion_elsewhere_or_none_fails(self):
        self.assertTrue(session.pad_hover_problems([dict(MOTION, wx=10.0)], CLIENT, (380, 415)))
        self.assertTrue(session.pad_hover_problems([], CLIENT, (380, 415)))

    def test_a_press_with_the_hover_fails(self):
        press = {"ev": "press", "widget": "area", "n": 1, "wx": 280.0, "wy": 315.0}
        problems = session.pad_hover_problems([MOTION, press], CLIENT, (380, 415))
        self.assertTrue(any("press" in problem for problem in problems), problems)


class LegReadTests(unittest.TestCase):
    def run_leg(self, leg, mode: str, answer, sampled=(), cursor=(640.0, 400.0), still=()):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "pad.log"
            log.write_text("")
            pads = []

            @contextlib.contextmanager
            def opened(_tier, instance, label, backend="native"):
                pads.append((instance["signature"], label, backend))
                yield GONE, CLIENT, log

            @contextlib.contextmanager
            def shown(*_):
                yield "Virtual-1"

            @contextlib.contextmanager
            def beside(*_):
                yield list(sampled)

            reads = dict(main_instance=lambda tier: MAIN, instance_env=lambda *a: {}, pad_ref=lambda *a: "@s1:e1",
                         beside_bystander=beside, app_shown=shown, session_client=lambda *a: CLIENT,
                         cursor_at=lambda *a: cursor, held_still=lambda *a: list(still),
                         _cursor_on=lambda *a: (380.0, 415.0), settled=lambda *a, **k: [])
            ctx = FakeContext(answer)
            ctx.mode = mode
            with mock.patch.multiple(session, pointer_pad=opened, **reads), \
                    mock.patch.multiple(pointer, pointer_pad=opened, app_shown=shown, session_client=lambda *a: CLIENT,
                                        main_instance=lambda tier: MAIN, bystander_box=lambda *a: {"address": "0xb"},
                                        cursor_at=lambda *a: cursor, pads_left=lambda tier: []), \
                    mock.patch.multiple(ref_refusals, cursor_at=lambda *a: cursor, active_address=lambda *a: "0xb",
                                        settled=lambda log, check, seconds=0: []):
                leg(ctx)
        return ctx, pads

    def test_the_headed_refusal_leg_drives_one_xwayland_session_pad_and_reads_each_oracle_once(self):
        ctx, pads = self.run_leg(session.session_ref_pointer_refusals, "headed", headed_answer)
        self.assertEqual(pads, [("main", "session-refs-headed", session.XWAYLAND)])
        self.assertEqual([envelope.error["code"] for envelope in ctx.products],
                         ["ACTION_FAILED"] * 6 + ["ACTION_NOT_SUPPORTED"])
        self.assertEqual(evaluate(ctx), [])
        self.assertEqual(set(ctx.reads), {"applog.pad", "hyprland.bystander", "hyprland.cursor"})

    def test_the_headless_refusal_leg_wants_policy_denied_for_each_drag_and_the_hover(self):
        ctx, _ = self.run_leg(session.session_ref_pointer_refusals, "headless", lambda argv: refused(POLICY))
        self.assertEqual(len(ctx.products), 4)
        self.assertEqual(evaluate(ctx), [])
        with self.assertRaises(AssertionError):
            self.run_leg(session.session_ref_pointer_refusals, "headless", headed_answer)

    def test_the_refusal_leg_fails_on_a_bystander_change(self):
        with self.assertRaises(AssertionError) as caught:
            self.run_leg(session.session_ref_pointer_refusals, "headed", headed_answer,
                         sampled=["bystander changed to None"])
        self.assertIn("bystander changed to None", str(caught.exception))

    def test_the_hover_leg_drives_a_native_session_pad_and_reads_each_oracle_once(self):
        ctx, pads = self.run_leg(session.session_hover_coordinates, "headed", lambda argv: hovered((380, 415)),
                                 still=[MOTION])
        self.assertEqual(pads, [("main", "session-hover", "native")])
        self.assertEqual(evaluate(ctx), [])
        self.assertEqual(set(ctx.reads), {"applog.pad", "hyprland.bystander", "hyprland.cursor"})

    def test_the_hover_leg_fails_when_the_pad_sees_no_motion_at_the_aim(self):
        with self.assertRaises(AssertionError) as caught:
            self.run_leg(session.session_hover_coordinates, "headed", lambda argv: hovered((380, 415)))
        self.assertIn("last motions", str(caught.exception))

    def test_the_hover_leg_fails_when_the_pad_logs_a_press(self):
        press = {"ev": "press", "widget": "area", "n": 1, "wx": 280.0, "wy": 315.0}
        with self.assertRaises(AssertionError) as caught:
            self.run_leg(session.session_hover_coordinates, "headed", lambda argv: hovered((380, 415)),
                         still=[MOTION, press])
        self.assertIn("press", str(caught.exception))


class RegistryTests(unittest.TestCase):
    def test_each_session_leg_names_the_session_instance_and_registers_lazily(self):
        for leg_id in SESSION_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
                self.assertIn("session instance", leg.requirement)
                self.assertNotIn("nested", leg.requirement + leg.target)
                self.assertIn("hyprland", leg.oracles)

    def test_the_refusal_leg_runs_in_both_modes_and_the_hover_leg_headed(self):
        self.assertEqual(legs.leg_by_id("pointer.session_ref_pointer_refusals").modes, ("headless", "headed"))
        self.assertEqual(legs.leg_by_id("pointer.session_hover_coordinates").modes, ("headed",))
        for leg_id in SESSION_LEGS[:2]:
            self.assertEqual(legs.LAZY_IMPLEMENTATIONS[leg_id][0], "scenarios.hyprland_session_pointer")


if __name__ == "__main__":
    unittest.main()
