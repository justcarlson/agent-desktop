from __future__ import annotations

import contextlib
import signal
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import pixels
from runner import legs, tier_record
from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import hyprland_capture_loss as loss

LEG_ID = "capture.compositor_loss_refused"
HOLDER = {"address": "0xby", "pid": 1}
DESKTOP = {"active_window": HOLDER, "workspace": "1", "monitors": [("M-1", True, "1", "")]}
NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
INSTANCE = {"signature": "sig-nested", "pid": 4242, "start": 77, "wayland": "wayland-9"}


def painted_image(paint: str = loss.PAINT, size: int = 40) -> pixels.Image:
    rgb = bytes(int(paint[index:index + 2], 16) for index in (0, 2, 4))
    return pixels.Image(width=size, height=size, rgb=rgb * size * size)


def refused(code: str = "PLATFORM_NOT_SUPPORTED", disposition=None, elapsed_ms: float = 40.0) -> Envelope:
    error = {"code": code, "message": "the compositor is gone", "disposition": disposition or NOT_DELIVERED}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=elapsed_ms, argv=[], exit_code=1)


def captured(path: str = "/tmp/frame.png") -> Envelope:
    return Envelope(ok=True, data={"path": path}, error=None, raw="{}", elapsed_ms=10.0, argv=[], exit_code=0)


class LossRefusalTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.path = Path(self.directory.name) / "capture-loss.png"

    def tearDown(self):
        self.directory.cleanup()

    def test_a_typed_not_delivered_refusal_with_no_file_passes(self):
        for code in loss.REFUSAL_CODES:
            with self.subTest(code=code):
                self.assertEqual(loss.loss_problems("stopped", refused(code), self.path), [])

    def test_a_frame_after_the_loss_fails(self):
        problems = loss.loss_problems("stopped", captured(str(self.path)), self.path)
        self.assertTrue(any("frame" in problem for problem in problems), problems)

    def test_a_written_file_fails_even_with_a_refusal(self):
        self.path.write_bytes(b"png")
        problems = loss.loss_problems("killed", refused(), self.path)
        self.assertTrue(any("wrote" in problem for problem in problems), problems)

    def test_an_untyped_or_late_or_delivered_refusal_fails(self):
        rows = {
            "internal": refused("INTERNAL"),
            "timeout": refused("TIMEOUT"),
            "slow": refused(elapsed_ms=loss.REFUSAL_BUDGET_MS + 1),
            "uncertain": refused(disposition={"delivery": "delivery_uncertain", "retry": "unsafe"}),
        }
        for label, envelope in rows.items():
            with self.subTest(case=label):
                self.assertTrue(loss.loss_problems("killed", envelope, self.path))

    def test_the_first_capture_must_show_the_paint(self):
        self.assertEqual(loss.rendered_problems("stopped", painted_image()), [])
        self.assertTrue(loss.rendered_problems("stopped", painted_image("000000")))


@unittest.skipUnless(tier_record.installed(), "the attached tier is fork-only")
class ExactStopTests(unittest.TestCase):
    def test_a_kill_signals_only_the_recorded_pid_when_its_start_matches(self):
        with mock.patch.object(loss, "nested_running", side_effect=[True, False]), \
                mock.patch.object(loss.os, "kill") as kill:
            loss.lose(SimpleNamespace(), dict(INSTANCE), "killed")
        kill.assert_called_once_with(INSTANCE["pid"], signal.SIGKILL)

    def test_a_kill_refuses_a_pid_whose_start_changed(self):
        with mock.patch.object(loss, "nested_running", return_value=False), \
                mock.patch.object(loss.os, "kill") as kill:
            with self.assertRaises(AssertionError):
                loss.lose(SimpleNamespace(), dict(INSTANCE), "killed")
        kill.assert_not_called()

    def test_a_stop_goes_through_the_record_only_for_the_recorded_instance(self):
        tier = SimpleNamespace(runtime_dir=Path("/nonexistent"))
        with mock.patch.object(loss.second_instance, "current", return_value=dict(INSTANCE)), \
                mock.patch("runner.attached.load", return_value={"instances": [INSTANCE]}), \
                mock.patch.object(loss, "nested_running", return_value=False), \
                mock.patch.object(loss.second_instance, "stop") as stop:
            loss.lose(tier, dict(INSTANCE), "stopped")
        stop.assert_called_once_with(tier)
        other = dict(INSTANCE, pid=5151)
        with mock.patch.object(loss.second_instance, "current", return_value=other), \
                mock.patch("runner.attached.load", return_value={"instances": [other]}), \
                mock.patch.object(loss.second_instance, "stop") as stop:
            with self.assertRaises(AssertionError):
                loss.lose(tier, dict(INSTANCE), "stopped")
        stop.assert_not_called()


class LegFlowTests(unittest.TestCase):
    def run_leg(self, after_loss, first=None) -> tuple[LegContext, list]:
        lost: list[str] = []
        calls: list[tuple[str, list[str]]] = []
        state = {"lost": False, "shown": False}
        self.shown: list[tuple[str, bool]] = []

        def nested(tier):
            state["lost"] = False
            return dict(INSTANCE), True

        def lose(tier, instance, how):
            state["lost"] = True
            lost.append(how)

        def product(argv, env=None, **kw):
            if argv[0] == "list-windows":
                return Envelope(ok=True, data=[{"id": "w-hypr-1", "pid": 600}], error=None, raw="[]",
                                elapsed_ms=5.0, argv=argv, exit_code=0)
            calls.append(("lost" if state["lost"] else "live", argv))
            self.shown.append((calls[-1][0], state["shown"]))
            if state["lost"]:
                return after_loss(Path(argv[-1]))
            Path(argv[-1]).write_bytes(b"png")
            return first or captured(argv[-1])

        @contextlib.contextmanager
        def frames(ctx, instance):
            state["shown"] = True
            try:
                yield
            finally:
                state["shown"] = False

        with tempfile.TemporaryDirectory() as runtime:
            tier = SimpleNamespace(runtime_dir=Path(runtime), env={})
            ctx = LegContext(tier, "headless", leg=legs.leg_by_id(LEG_ID), member_reader=lambda tier: frozenset())
            ctx.product = product
            patches = {
                "main_instance": mock.Mock(return_value={}),
                "bystander": mock.Mock(return_value=HOLDER),
                "cursor": mock.Mock(return_value={"x": 0, "y": 0}),
                "desktop": mock.Mock(side_effect=lambda tier, main: dict(DESKTOP)),
                "keeps_focus": mock.Mock(return_value=[]),
                "restore_desktop": mock.Mock(),
                "nested": mock.Mock(side_effect=nested),
                "instance_env": mock.Mock(return_value={"WAYLAND_DISPLAY": "wayland-9"}),
                "nested_painted": mock.Mock(side_effect=lambda *a: contextlib.nullcontext(
                    (SimpleNamespace(pid=600), {"pid": 600}))),
                "decode": mock.Mock(return_value=painted_image()),
                "lose": mock.Mock(side_effect=lose),
                "nested_frames": frames,
            }
            with contextlib.ExitStack() as stack:
                for name, value in patches.items():
                    stack.enter_context(mock.patch.object(loss, name, value))
                stop = stack.enter_context(mock.patch.object(loss.second_instance, "stop"))
                try:
                    loss.compositor_loss_refused(ctx)
                    error = None
                except AssertionError as raised:
                    error = raised
                self.stopped = stop.call_count
            return ctx, calls, lost, error

    def test_both_losses_refuse_and_the_oracles_balance(self):
        ctx, calls, lost, error = self.run_leg(lambda path: refused())
        self.assertIsNone(error)
        self.assertEqual(lost, list(loss.LOSSES))
        self.assertEqual([stage for stage, _ in calls], ["live", "lost"] * len(loss.LOSSES))
        self.assertEqual(evaluate(ctx), [])
        self.assertGreaterEqual(self.stopped, 1)

    def test_the_live_capture_runs_while_the_outer_session_shows_the_instance(self):
        self.run_leg(lambda path: refused())
        self.assertEqual(self.shown, [("live", True), ("lost", False)] * len(loss.LOSSES))

    def test_a_frame_after_the_loss_fails_the_leg(self):
        def replay(path):
            path.write_bytes(b"png")
            return captured(str(path))

        _, _, _, error = self.run_leg(replay)
        self.assertIsNotNone(error)
        self.assertIn("frame", str(error))

    def test_a_first_capture_that_fails_stops_before_any_loss(self):
        _, _, lost, error = self.run_leg(lambda path: refused(), first=refused("ACTION_FAILED"))
        self.assertIsNotNone(error)
        self.assertEqual(lost, [])
        self.assertIn("did not render", str(error))


class RegistryTests(unittest.TestCase):
    def test_the_leg_is_a_headless_hyprland_refusal(self):
        leg = legs.leg_by_id(LEG_ID)
        self.assertEqual(leg.tiers, ("hyprland",))
        self.assertEqual(leg.modes, ("headless",))
        self.assertEqual(leg.disposition, "asserted-refusal")
        self.assertTrue({"hyprland", "pixels", "proc"} <= set(leg.oracles))

    def test_the_leg_runs_the_loss_scenario(self):
        self.assertEqual(legs.LAZY_IMPLEMENTATIONS[LEG_ID],
                         ("scenarios.hyprland_capture_loss", "compositor_loss_refused"))
        self.assertIn(LEG_ID, legs.implementations())

    def test_the_leg_runs_before_the_shared_zoo_rows(self):
        order = [leg.id for leg in legs.REGISTRY if "hyprland" in leg.tiers]
        self.assertLess(order.index(LEG_ID), order.index("ref_contract.zoo_gtk3"))


if __name__ == "__main__":
    unittest.main()
