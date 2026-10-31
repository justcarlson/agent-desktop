from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs, verdict
from runner.product import Envelope
from scenarios import hyprland_binding

LEG_IDS = ("session.hyprland_live_instance_binds", "session.hyprland_dead_named_instance_refused",
           "session.hyprland_absent_keeps_generic_route")
NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}


def listed(*names: str) -> Envelope:
    data = [{"id": name, "is_primary": index == 0} for index, name in enumerate(names)]
    return Envelope(ok=True, data=data, error=None, raw="[]", elapsed_ms=5.0, argv=[], exit_code=0)


def refused(code: str = "PLATFORM_NOT_SUPPORTED", *, exit_code: int = 1, elapsed_ms: float = 5.0,
            disposition=None, **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details,
             "disposition": NOT_DELIVERED if disposition is None else disposition}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=elapsed_ms, argv=[],
                    exit_code=exit_code)


def tier() -> SimpleNamespace:
    return SimpleNamespace(env={"PATH": "/usr/bin", "HOME": "/home/user", "WAYLAND_DISPLAY": "wayland-1",
                                "HYPRLAND_INSTANCE_SIGNATURE": "sig", "DBUS_SESSION_BUS_ADDRESS": "unix:x"},
                           state_root=Path("/tmp/state"), runtime_dir=Path("/tmp/work"))


class RegistryTests(unittest.TestCase):
    def test_each_binding_leg_is_a_headless_hyprland_leg_with_an_implementation(self):
        for leg_id in LEG_IDS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "pass"))
                self.assertIn("hyprland", leg.oracles)
                self.assertIn(leg_id, legs.implementations())


class EnvironmentTests(unittest.TestCase):
    def test_the_bare_environment_drops_display_bus_and_compositor_variables(self):
        env = hyprland_binding.bare_env(tier(), XDG_RUNTIME_DIR="/run/user/1000", WAYLAND_DISPLAY=None)
        self.assertEqual(env, {"PATH": "/usr/bin", "HOME": "/home/user", "AGENT_DESKTOP_HOME": "/tmp/state",
                               "XDG_RUNTIME_DIR": "/run/user/1000"})

    def test_a_pair_names_the_runtime_signature_and_display_it_is_given(self):
        env = hyprland_binding.pair_env(tier(), "sig_2", None, Path("/run/user/1000"))
        self.assertEqual(env["HYPRLAND_INSTANCE_SIGNATURE"], "sig_2")
        self.assertNotIn("WAYLAND_DISPLAY", env)


class DisplayTests(unittest.TestCase):
    def test_disabled_outputs_are_not_expected(self):
        names = hyprland_binding.monitor_names([{"name": "B-1"}, {"name": "A-1"}, {"name": "C-1", "disabled": True}])
        self.assertEqual(names, ["A-1", "B-1"])

    def test_the_listed_ids_must_equal_the_compositor_outputs_in_any_order(self):
        check = hyprland_binding.display_problems
        self.assertEqual(check("x", listed("B-1", "A-1"), ["A-1", "B-1"]), [])
        self.assertTrue(check("x", listed("A-1"), ["A-1", "B-1"]))
        self.assertTrue(check("x", listed("A-1", "OTHER-1"), ["A-1"]))
        self.assertTrue(check("x", refused(display_unavailable="hyprland_mismatch"), ["A-1"]))
        self.assertTrue(check("x", listed(), []))


class RefusalTests(unittest.TestCase):
    def test_a_mismatch_needs_its_code_cause_exit_disposition_and_budget(self):
        check = lambda envelope: hyprland_binding.refusal_problems("x", envelope, "hyprland_mismatch")
        self.assertEqual(check(refused(display_unavailable="hyprland_mismatch")), [])
        for name, envelope in {
            "code": refused("TIMEOUT", display_unavailable="hyprland_mismatch"),
            "cause": refused(display_unavailable="hyprland_ambiguous"),
            "exit": refused(exit_code=2, display_unavailable="hyprland_mismatch"),
            "disposition": refused(disposition={"delivery": "unknown", "retry": "unknown"},
                                   display_unavailable="hyprland_mismatch"),
            "budget": refused(elapsed_ms=hyprland_binding.BUDGET_MS + 1, display_unavailable="hyprland_mismatch"),
            "bound": listed("A-1"),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(check(envelope))


class DeadNamedLegTests(unittest.TestCase):
    def run_leg(self, answer) -> None:
        ctx = SimpleNamespace(tier=tier(), mode="headless", before=mock.Mock(), after=mock.Mock(),
                              restored=mock.Mock(), product=lambda argv, env, headed: answer(env))
        main = {"signature": "live", "pid": 10, "wayland": "wayland-1"}
        with mock.patch.object(hyprland_binding, "shared", return_value=Path("/run/user/1000")), \
                mock.patch.object(hyprland_binding, "main_instance", return_value=main), \
                mock.patch.object(hyprland_binding.second_instance, "stop"), \
                mock.patch.object(hyprland_binding, "compositor_names", return_value=["A-1"]), \
                mock.patch.object(hyprland_binding, "desktop", return_value={"focus": "x"}), \
                mock.patch.object(hyprland_binding.socket_fixture, "stale_instances",
                                  return_value=["ad_e2e_stale_1", "ad_e2e_stale_2"]):
            hyprland_binding.hyprland_dead_named_instance_refused(ctx)

    @staticmethod
    def contract(env) -> Envelope:
        signature = env.get("HYPRLAND_INSTANCE_SIGNATURE")
        if signature in (None, "live"):
            return listed("A-1")
        return refused(display_unavailable="hyprland_mismatch",
                       detail=f"HYPRLAND_INSTANCE_SIGNATURE {signature} names no live Hyprland instance")

    def test_the_leg_passes_when_every_dead_name_refuses(self):
        self.run_leg(self.contract)

    def test_the_leg_fails_when_a_dead_name_falls_back_to_the_live_instance(self):
        fallback = lambda env: listed("A-1") if env.get("WAYLAND_DISPLAY") else self.contract(env)
        with self.assertRaises(AssertionError):
            self.run_leg(fallback)

    def test_the_leg_fails_when_the_refusal_does_not_name_the_signature(self):
        unnamed = lambda env: (listed("A-1") if env.get("HYPRLAND_INSTANCE_SIGNATURE") in (None, "live")
                               else refused(display_unavailable="hyprland_mismatch", detail="no instance"))
        with self.assertRaises(AssertionError):
            self.run_leg(unnamed)


class LiveLegDumpTests(unittest.TestCase):
    MAIN = {"signature": "live", "pid": 10, "wayland": "wayland-1"}
    NESTED = {"signature": "nested", "pid": 20, "wayland": "wayland-2"}

    def run_leg(self, answer, sized=None, ctx=None) -> None:
        self.after = mock.Mock()
        self.stop = mock.Mock()
        ctx = ctx or SimpleNamespace(tier=tier(), mode="headless", before=mock.Mock(), after=self.after,
                                     restored=mock.Mock())
        ctx.product = lambda argv, env, headed: answer(env)
        names = lambda runtime, signature: ["A-1"] if signature == "live" else ["B-1"]
        with mock.patch.object(hyprland_binding, "shared", return_value=Path("/run/user/1000")), \
                mock.patch.object(hyprland_binding, "main_instance", return_value=self.MAIN), \
                mock.patch.object(hyprland_binding.second_instance, "stop", self.stop), \
                mock.patch.object(hyprland_binding.second_instance, "ensure_sized",
                                  side_effect=sized, return_value=self.NESTED), \
                mock.patch.object(hyprland_binding, "compositor_names", side_effect=names), \
                mock.patch.object(hyprland_binding, "desktop", return_value={"focus": "x"}), \
                mock.patch.object(hyprland_binding.socket_fixture, "stale_instances"), \
                mock.patch.object(hyprland_binding.socket_fixture, "instance", return_value=(None, mock.Mock())), \
                mock.patch.object(hyprland_binding, "monitors_all_record", return_value=[{"name": "B-1"}]), \
                mock.patch.object(hyprland_binding, "rerun_with_wayland_debug",
                                  side_effect=lambda label, env: {"label": label, "stderr_empty": True}):
            hyprland_binding.hyprland_live_instance_binds(ctx)

    def dumps(self) -> list[tuple[str, str]]:
        return [(call.args[0], call.args[1]["label"]) for call in self.after.call_args_list
                if call.args[0] in ("hyprland.second_monitors_all", "hyprland.second_wayland_debug")]

    def test_a_refused_second_listing_records_its_monitors_and_protocol_log_and_still_fails(self):
        incomplete = refused(detail="an output came back without a name, position or size")
        answer = lambda env: incomplete if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "nested" else listed("A-1")
        with self.assertRaises(AssertionError):
            self.run_leg(answer)
        self.assertEqual(self.dumps(), [
            ("hyprland.second_monitors_all", "second pair"), ("hyprland.second_wayland_debug", "second pair"),
            ("hyprland.second_monitors_all", "second signature only"),
            ("hyprland.second_wayland_debug", "second signature only")])

    def test_a_listing_that_succeeds_records_no_dump(self):
        answer = lambda env: listed("B-1") if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "nested" else listed("A-1")
        self.run_leg(answer)
        self.assertEqual(self.dumps(), [])

    def test_outputs_that_never_get_a_size_record_the_diagnostics_and_skip_the_second_listing(self):
        failure = hyprland_binding.second_instance.OutputsNotReady("0x0")
        failure.diagnostics = {"outer_client": {"pid": 20}}
        seen = []
        answer = lambda env: seen.append(env.get("HYPRLAND_INSTANCE_SIGNATURE")) or listed("A-1")
        with self.assertRaises(hyprland_binding.second_instance.OutputsNotReady):
            self.run_leg(answer, sized=failure)
        self.after.assert_any_call("hyprland.second_not_ready", {"outer_client": {"pid": 20}})
        self.assertNotIn("nested", seen)
        self.assertEqual(self.stop.call_count, 2)

    def test_a_passing_leg_reads_each_instance_before_after_and_restored(self):
        answer = lambda env: listed("B-1") if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "nested" else listed("A-1")
        ctx = verdict.LegContext(tier(), "headless", legs.leg_by_id("session.hyprland_live_instance_binds"),
                                 member_reader=lambda _tier: frozenset())
        self.run_leg(answer, ctx=ctx)
        self.assertEqual(verdict.evaluate(ctx), [])
        self.assertEqual(list(ctx.reads["hyprland.second_desktop"]), ["before", "after", "restored"])

    def test_the_leg_stops_its_sized_instance_so_later_legs_start_their_own(self):
        answer = lambda env: listed("B-1") if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "nested" else listed("A-1")
        self.run_leg(answer)
        self.assertEqual(self.stop.call_count, 2)


class WaylandDebugRecordTests(unittest.TestCase):
    def test_an_empty_stderr_is_recorded_as_empty(self):
        record = hyprland_binding.debug_record("second pair", {"WAYLAND_DEBUG": "1"}, 1, "  \n")
        self.assertTrue(record["stderr_empty"])
        self.assertNotIn("stderr", record)

    def test_the_protocol_log_is_kept_and_bounded(self):
        log = "x" * (hyprland_binding.DEBUG_LIMIT + 5)
        record = hyprland_binding.debug_record("second pair", {"WAYLAND_DEBUG": "1"}, 1, log)
        self.assertEqual(len(record["stderr"]), hyprland_binding.DEBUG_LIMIT)
        self.assertTrue(record["stderr_truncated"])
        self.assertEqual(record["env"]["WAYLAND_DEBUG"], "1")

    def test_the_rerun_adds_wayland_debug_to_the_failing_environment(self):
        env = hyprland_binding.wayland_debug_env({"HYPRLAND_INSTANCE_SIGNATURE": "nested"})
        self.assertEqual((env["WAYLAND_DEBUG"], env["HYPRLAND_INSTANCE_SIGNATURE"]), ("1", "nested"))


if __name__ == "__main__":
    unittest.main()
