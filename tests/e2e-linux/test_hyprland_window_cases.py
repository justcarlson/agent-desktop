from __future__ import annotations

import contextlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import (hyprland_window_app as app, hyprland_window_checks as checks,
                       hyprland_window_legacy as legacy, hyprland_window_nested as nested,
                       hyprland_window_ops as ops, hyprland_window_refusals as refusals)

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}


def failed(code: str, cause: str | None = None) -> Envelope:
    details = {"physical_delivery_started": False} | ({"cause": cause} if cause else {})
    error = {"code": code, "message": "refused", "details": details, "disposition": NOT_DELIVERED}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def done(delivery: str) -> Envelope:
    data = {"disposition": {"delivery": delivery, "retry": "unsafe"}}
    return Envelope(ok=True, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)


class Recorder:
    def __init__(self, *envelopes: Envelope):
        self.tier = SimpleNamespace()
        self.envelopes = list(envelopes)
        self.calls: list[tuple[list[str], dict | None]] = []

    def product(self, argv, env=None, **_):
        self.calls.append((argv, env))
        return self.envelopes[0] if len(self.envelopes) == 1 else self.envelopes.pop(0)


def still(value: dict):
    return lambda: dict(value)


class MissingWindowTests(unittest.TestCase):
    def test_every_operation_on_a_gone_id_is_not_found_and_touches_no_window(self):
        ctx = Recorder(failed("WINDOW_NOT_FOUND"))
        reads = {"target": still({"at": [1, 2]}), "bystander": still({"at": [3, 4]})}
        self.assertEqual(checks.missing(ctx, "closed", "w-hypr-9", reads, env={"A": "1"}), [])
        self.assertEqual([argv[0] for argv, _ in ctx.calls],
                         ["move-window", "resize-window", "minimize", "maximize", "restore"])
        self.assertTrue(all(argv[1:3] == ["--window-id", "w-hypr-9"] and env == {"A": "1"}
                            for argv, env in ctx.calls))

    def test_a_fallback_to_another_window_or_another_code_fails(self):
        moved = iter([{"at": [3, 4]}, {"at": [9, 9]}] * 5)
        cases = {
            "fallback": (failed("WINDOW_NOT_FOUND"), {"bystander": lambda: next(moved)}),
            "code": (failed("ACTION_NOT_SUPPORTED", "tiled_window"), {"target": still({})}),
            "ok": (done("delivered_verified"), {"target": still({})}),
        }
        for name, (envelope, reads) in cases.items():
            with self.subTest(case=name):
                self.assertNotEqual(checks.missing(Recorder(envelope), "closed", "w-1", reads), [])


class EnvironmentTests(unittest.TestCase):
    def test_every_verdict_runs_the_product_in_the_given_environment(self):
        env = {"HYPRLAND_INSTANCE_SIGNATURE": "nested"}
        read = still(checks.placement({"at": [40, 60]}))
        with mock.patch.object(checks.raw, "poll", return_value=None):
            for run, envelope in ((lambda c: checks.operated(c, "m", ["move-window"], read, {"at": [40, 60]},
                                                             env=env), done("delivered_verified")),
                                  (lambda c: checks.satisfied(c, "m", ["move-window"], read, env=env),
                                   done("not_delivered")),
                                  (lambda c: checks.refusal(c, "m", ["move-window"], read, "tiled_window", env=env),
                                   failed("ACTION_NOT_SUPPORTED", "tiled_window"))):
                ctx = Recorder(envelope)
                with self.subTest(envelope=envelope.raw):
                    self.assertEqual(run(ctx), [])
                    self.assertEqual(ctx.calls[0][1], env)


class SizerLogTests(unittest.TestCase):
    def setUp(self):
        self.log = Path(tempfile.mkdtemp()) / "sizer.events"

    def write(self, *lines: dict) -> None:
        self.log.write_text("".join(json.dumps(line) + "\n" for line in lines))

    def test_the_last_configure_must_match_the_compositor_size_within_the_tolerance(self):
        self.write({"ev": "configure", "w": 420, "h": 300}, {"ev": "configure", "w": 501, "h": 399})
        with mock.patch.object(app.raw, "poll", return_value=None):
            self.assertEqual(app.size_problems("resize", self.log, [500, 400]), [])
            self.assertNotEqual(app.size_problems("resize", self.log, [420, 300]), [])
            self.write()
            self.assertNotEqual(app.size_problems("resize", self.log, [500, 400]), [])

    def test_the_last_state_must_say_maximized_exactly_when_asked(self):
        self.write({"ev": "state", "maximized": True}, {"ev": "configure", "w": 1, "h": 1})
        with mock.patch.object(app.raw, "poll", return_value=None):
            self.assertEqual(app.state_problems("maximize", self.log, True), [])
            self.assertNotEqual(app.state_problems("restore", self.log, False), [])
            self.write()
            self.assertNotEqual(app.state_problems("restore", self.log, False), [])

    def test_a_refusal_must_leave_the_log_quiet(self):
        self.write({"ev": "configure", "w": 1, "h": 1})
        self.assertEqual(app.settled_count(self.log, quiet_s=0.0), 1)
        self.assertEqual(app.quiet_problems("tiled move", self.log, 1), [])
        self.write({"ev": "configure", "w": 1, "h": 1}, {"ev": "state", "maximized": True})
        self.assertNotEqual(app.quiet_problems("tiled move", self.log, 1), [])


class FakeSized:
    def __init__(self, now: dict, log: Path):
        self.now, self.log, self.steps, self.argvs = now, log, [], []

    def read(self):
        return dict(self.now)

    def argv(self, verb, *extra):
        return [verb, "--window-id", "w-1", *extra]

    def operated(self, label, argv, expected):
        self.steps.append(("operated", argv[0]))
        self.argvs.append(argv)
        return []

    def satisfied(self, label, argv):
        self.steps.append(("satisfied", argv[0]))
        self.argvs.append(argv)
        return []


class NestedPlacementTests(unittest.TestCase):
    def test_a_move_to_where_the_window_is_must_be_satisfied_and_any_other_move_verified(self):
        log = Path(tempfile.mkdtemp()) / "sizer.events"
        log.write_text(json.dumps({"ev": "configure", "w": 480, "h": 360}) + "\n")
        with mock.patch.object(app.raw, "poll", return_value=None):
            here = FakeSized({"at": [-984, -580]}, log)
            self.assertEqual(nested.placed(here, "scaled", [-984, -580], [480, 360]), [])
            self.assertEqual(here.steps, [("satisfied", "move-window"), ("operated", "resize-window")])
            there = FakeSized({"at": [0, 0]}, log)
            self.assertEqual(nested.placed(there, "scaled", [-984, -580], [480, 360]), [])
            self.assertEqual(there.steps, [("operated", "move-window"), ("operated", "resize-window")])
            self.assertEqual(there.argvs[0][-2:], ["--x=-984", "--y=-580"])

    def test_a_point_is_written_in_the_form_a_negative_value_parses_in(self):
        self.assertEqual(checks.point(-50, -20), ["--x=-50", "--y=-20"])
        self.assertEqual(checks.point(40, 60), ["--x=40", "--y=60"])

    def test_inside_reads_the_monitor_in_logical_pixels(self):
        monitor = {"x": -1024, "y": -640, "width": 1280, "height": 800, "scale": 1.25}
        self.assertTrue(nested.inside({"at": [-984, -580], "size": [480, 360]}, monitor))
        self.assertFalse(nested.inside({"at": [-984, -580], "size": [1000, 360]}, monitor))
        self.assertFalse(nested.inside({"at": [0, 0], "size": [10, 10]}, monitor))


class LegacyInstanceTests(unittest.TestCase):
    def patched(self, found):
        tier = SimpleNamespace(runtime_dir=Path(tempfile.mkdtemp()), env={"PATH": "/usr/bin"})
        patches = {
            "stop": mock.patch.object(legacy.second_instance, "stop"),
            "find": mock.patch.object(legacy.second_instance, "find_new", return_value=found),
            "wayland": mock.patch.object(legacy.second_instance, "wayland_of", return_value="wayland-9"),
            "spawn": mock.patch.object(legacy.attached_session, "spawn"),
            "load": mock.patch.object(legacy.attached, "load", return_value={}),
            "record": mock.patch.object(legacy.attached, "record_instance"),
            "shared": mock.patch.object(legacy, "shared", return_value=Path("/run/x")),
            "instances": mock.patch.object(legacy.hyprland, "instances", return_value=[]),
            "lua": mock.patch.object(legacy.hyprland, "uses_lua", return_value=False),
            "control": mock.patch.object(legacy.hyprland, "control", return_value="ok"),
            "settle": mock.patch.object(legacy, "settle_outputs"),
            "poll": mock.patch.object(legacy.raw, "poll", side_effect=lambda read, _s: read()),
        }
        return tier, patches

    def test_it_replaces_the_second_instance_from_a_legacy_config_and_stops_it_after(self):
        tier, patches = self.patched({"signature": "legacy", "pid": 7, "start": 1, "wayland": None})
        started = {name: patch.start() for name, patch in patches.items()}
        self.addCleanup(mock.patch.stopall)
        with legacy.legacy_instance(tier) as (instance, lua):
            self.assertEqual(started["stop"].call_count, 1)
            self.assertEqual((instance["signature"], instance["wayland"], lua), ("legacy", "wayland-9", False))
        self.assertEqual(started["stop"].call_count, 2)
        self.assertIn("Hyprland", started["spawn"].call_args.args[1])
        self.assertNotIn("hl.", (tier.runtime_dir / legacy.CONFIG).read_text())

    def test_an_instance_that_never_answers_still_stops_the_second_instance(self):
        tier, patches = self.patched(None)
        started = {name: patch.start() for name, patch in patches.items()}
        self.addCleanup(mock.patch.stopall)
        with self.assertRaises(Exception):
            with legacy.legacy_instance(tier):
                pass
        self.assertEqual(started["stop"].call_count, 2)


class ForeignTests(unittest.TestCase):
    def test_a_window_another_tool_hid_is_refused_for_each_state_change_and_put_back(self):
        moves = []
        ctx = Recorder(failed("ACTION_NOT_SUPPORTED", "foreign_special_workspace"))
        with mock.patch.object(ops, "move", side_effect=lambda _t, _a, ws: moves.append(ws)):
            self.assertEqual(ops._foreign(ctx, still({}), "w-1", "0xa1", "31"), [])
        self.assertEqual(moves, [ops.FOREIGN, "31"])
        self.assertEqual([argv[0] for argv, _ in ctx.calls], ["minimize", "maximize", "restore"])

    def test_the_window_is_put_back_even_when_a_refusal_breaks(self):
        moves = []
        ctx = SimpleNamespace(tier=None, product=mock.Mock(side_effect=RuntimeError("gone")))
        with mock.patch.object(ops, "move", side_effect=lambda _t, _a, ws: moves.append(ws)), \
                self.assertRaises(RuntimeError):
            ops._foreign(ctx, still({}), "w-1", "0xa1", "31")
        self.assertEqual(moves, [ops.FOREIGN, "31"])


class RefusalLegTests(unittest.TestCase):
    def test_the_parked_sizer_compares_the_bystander_with_its_own_placement(self):
        tier = SimpleNamespace()
        ctx = SimpleNamespace(tier=tier, product=lambda *_a, **_k: None)
        settled = {"at": [1, 2], "size": [3, 4], "floating": False, "fullscreen": 0, "workspace": "1"}

        @contextlib.contextmanager
        def fake_sizer(*_a):
            yield SimpleNamespace(pid=7), {"address": "0xa1"}, Path("/dev/null")

        with mock.patch.object(refusals, "located", return_value=still(settled)), \
                mock.patch.object(refusals, "sizer", fake_sizer), \
                mock.patch.object(refusals, "listed_ids", return_value=["w-1"]), \
                mock.patch.object(refusals, "workspaces_in_use", return_value=set()), \
                mock.patch.object(refusals, "spare_workspaces", return_value=["31"]), \
                mock.patch.object(refusals, "move"), mock.patch.object(refusals, "restore_desktop"), \
                mock.patch.object(refusals, "main_instance"), \
                mock.patch.object(refusals.raw, "poll", side_effect=lambda read, _s: read()):
            with refusals.parked_sizer(ctx, {"address": "0xb2"}, {}) as target:
                self.assertEqual(target, ("w-1", "0xa1", "31", 7, Path("/dev/null")))

    def test_every_oracle_the_leg_reads_is_read_before_after_and_restored(self):
        ctx = LegContext(SimpleNamespace(), "headless", member_reader=lambda _tier: frozenset())
        target = ("w-1", "0xa1", "31", 7, Path("/dev/null"))

        @contextlib.contextmanager
        def parked(*_a):
            yield target

        @contextlib.contextmanager
        def quiet(*_a):
            yield SimpleNamespace(changes=[])

        with mock.patch.object(refusals, "parked_sizer", parked), mock.patch.object(refusals, "Sampler", quiet), \
                mock.patch.object(refusals, "generation", return_value=[7, 100]), \
                mock.patch.object(refusals, "desktop", return_value={"focus": "0xb2"}), \
                mock.patch.object(refusals, "_unsupported", return_value=[]), \
                mock.patch.object(refusals, "_supported", return_value=[]), \
                mock.patch.object(refusals, "_absent", return_value=[]), \
                mock.patch.object(refusals, "keeps_focus", return_value=[]), \
                mock.patch.object(refusals, "watched"), mock.patch.object(refusals, "located"), \
                mock.patch.object(refusals, "bystander"), mock.patch.object(refusals, "cursor"), \
                mock.patch.object(refusals, "restore_desktop"), mock.patch.object(refusals, "main_instance"):
            refusals.window_operation_refusals(ctx)
        self.assertEqual(evaluate(ctx), [])

    def test_a_refusal_the_app_saw_fails(self):
        log = Path(tempfile.mkdtemp()) / "sizer.events"
        log.write_text("")
        ctx = Recorder(failed("ACTION_NOT_SUPPORTED", "tiled_window"))

        def noisy(*_a, **_k):
            log.write_text(json.dumps({"ev": "configure", "w": 2, "h": 2}) + "\n")
            return failed("ACTION_NOT_SUPPORTED", "tiled_window")

        with mock.patch.object(app.raw, "poll", return_value=None):
            self.assertEqual(refusals._refused(ctx, log, "tiled move", ["move-window"], still({}), "tiled_window"), [])
            ctx.product = noisy
            self.assertNotEqual(refusals._refused(ctx, log, "tiled move", ["move-window"], still({}),
                                                  "tiled_window"), [])


if __name__ == "__main__":
    unittest.main()
