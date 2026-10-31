from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner.product import Envelope
from scenarios import hyprland_binding, hyprland_cancel, hyprland_common, hyprland_desktop
from test_hyprland_lifecycle import make_proc

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}


def refused(code: str, *, exit_code: int = 1, disposition=None, **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details,
             "disposition": NOT_DELIVERED if disposition is None else disposition}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=exit_code)


def listed(*names: tuple[str, bool]) -> Envelope:
    data = [{"id": name, "is_primary": primary, "bounds": {}, "scale": 1.0} for name, primary in names]
    return Envelope(ok=True, data=data, error=None, raw="[]", elapsed_ms=5.0, argv=[], exit_code=0)


class RefusalContractTests(unittest.TestCase):
    def test_a_complete_binding_refusal_passes(self):
        envelope = refused("PLATFORM_NOT_SUPPORTED", display_unavailable="hyprland_ambiguous",
                           physical_delivery_started=False)
        self.assertEqual(hyprland_common.refusal_problems(
            "x", envelope, "PLATFORM_NOT_SUPPORTED", "hyprland_ambiguous", physical=True), [])

    def test_each_broken_binding_refusal_field_is_reported(self):
        good = dict(display_unavailable="hyprland_mismatch", physical_delivery_started=False)
        cases = {
            "code": refused("TIMEOUT", **good),
            "cause": refused("PLATFORM_NOT_SUPPORTED", display_unavailable="hyprland_ambiguous",
                             physical_delivery_started=False),
            "exit": refused("PLATFORM_NOT_SUPPORTED", exit_code=2, **good),
            "delivery": refused("PLATFORM_NOT_SUPPORTED", disposition={"delivery": "delivered_unverified"},
                                **good),
            "physical": refused("PLATFORM_NOT_SUPPORTED", display_unavailable="hyprland_mismatch"),
            "success": listed(("Virtual-1", True)),
        }
        for name, envelope in cases.items():
            with self.subTest(name=name):
                self.assertTrue(hyprland_common.refusal_problems(
                    name, envelope, "PLATFORM_NOT_SUPPORTED", "hyprland_mismatch", physical=True))

    def test_a_delivery_boundary_needs_its_code_cause_safe_retry_and_no_physical_start(self):
        good = refused("ACTION_NOT_SUPPORTED", cause="wayland_session", physical_delivery_started=False)
        check = lambda envelope: hyprland_desktop.boundary_problems(
            "x", envelope, ("ACTION_NOT_SUPPORTED",), physical=True, cause="wayland_session")
        self.assertEqual(check(good), [])
        for name, envelope in {
            "code": refused("POLICY_DENIED", cause="wayland_session", physical_delivery_started=False),
            "cause": refused("ACTION_NOT_SUPPORTED", cause="x11", physical_delivery_started=False),
            "retry": refused("ACTION_NOT_SUPPORTED", disposition={"delivery": "not_delivered", "retry": "unsafe"},
                             cause="wayland_session", physical_delivery_started=False),
            "physical": refused("ACTION_NOT_SUPPORTED", cause="wayland_session", physical_delivery_started=True),
            "exit": refused("ACTION_NOT_SUPPORTED", exit_code=0, cause="wayland_session",
                            physical_delivery_started=False),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(check(envelope))


class SilentDiscoveryTests(unittest.TestCase):
    def test_discovery_meets_the_silent_fixture_alone(self):
        created, seen = [], []

        def instance(work, behavior):
            created.append(behavior)
            return behavior, SimpleNamespace(path=Path(f"/run/hypr/{behavior}/.socket.sock"), close=lambda: None)

        def product(argv, env=None, headed=True):
            seen.append(tuple(created))
            return refused("TIMEOUT", display_unavailable="silent", retryable=True)

        ctx = SimpleNamespace(tier=SimpleNamespace(runtime_dir=Path("/work")), mode="headless", product=product)
        main = {"signature": "main", "wayland": "wayland-1"}
        with mock.patch.object(hyprland_binding, "main_instance", return_value=main), \
             mock.patch.object(hyprland_binding.socket_fixture, "instance", side_effect=instance), \
             mock.patch.object(hyprland_binding.socket_fixture, "Listener",
                               side_effect=lambda path, behavior: SimpleNamespace(path=path, close=lambda: None)), \
             mock.patch.object(hyprland_binding, "_observe"), \
             mock.patch.object(hyprland_binding, "_displays", return_value=[]), \
             mock.patch.object(hyprland_binding, "monitors", return_value=[]), \
             mock.patch.object(hyprland_binding, "pair_env", return_value={}):
            with self.assertRaises(AssertionError):
                hyprland_binding.silent_socket_deadline(ctx)
        self.assertEqual(seen[0], ("silent",))


class NoLiveInstanceTests(unittest.TestCase):
    def test_the_no_live_case_keeps_the_session_bus_and_drops_only_the_compositor(self):
        envs = []

        def refused_with(ctx, env, code, cause, label):
            envs.append((label, env))
            return []

        tier = SimpleNamespace(runtime_dir=Path("/work"), state_root=Path("/work/ad-home"),
                               bus_address="unix:path=/run/user/1000/bus", env={"PATH": "/usr/bin"})
        ctx = SimpleNamespace(tier=tier, mode="headed", product=lambda *a, **k: refused("PLATFORM_NOT_SUPPORTED"))
        main = {"signature": "main", "wayland": "wayland-1"}
        nested = {"signature": "nested", "wayland": "wayland-2"}
        with mock.patch.object(hyprland_binding, "main_instance", return_value=main), \
             mock.patch.object(hyprland_binding.second_instance, "ensure", return_value=nested), \
             mock.patch.object(hyprland_binding.socket_fixture, "private_runtime", return_value=Path("/work/nl.x")), \
             mock.patch.object(hyprland_common, "shared", return_value=Path("/run/user/1000")), \
             mock.patch.object(hyprland_binding, "_observe"), \
             mock.patch.object(hyprland_binding, "_refused", side_effect=refused_with), \
             mock.patch.object(hyprland_binding, "_displays", return_value=[]), \
             mock.patch.object(hyprland_binding, "monitors", return_value=[]):
            with self.assertRaises(AssertionError):
                hyprland_binding.ambiguous_or_mismatched_binding_refused(ctx)
        env = dict(envs)["no live instance"]
        self.assertEqual(env["XDG_RUNTIME_DIR"], "/work/nl.x")
        self.assertEqual(env["DBUS_SESSION_BUS_ADDRESS"], tier.bus_address)
        for key in ("WAYLAND_DISPLAY", "DISPLAY", "HYPRLAND_INSTANCE_SIGNATURE"):
            self.assertNotIn(key, env)


class AbsentRefTests(unittest.TestCase):
    def check(self, mode: str, envelope: Envelope) -> list[str]:
        ctx = SimpleNamespace(mode=mode, product=lambda argv: envelope)
        return hyprland_desktop.absent_ref_problems(ctx, "@s1:e999999")

    def test_headless_identity_refusal_is_not_found_or_stale(self):
        self.assertEqual(self.check("headless", refused("STALE_REF")), [])
        self.assertTrue(self.check("headless", refused("ACTION_NOT_SUPPORTED", cause="wayland_session",
                                                       physical_delivery_started=False)))

    def test_the_headed_pointer_gate_refuses_before_the_ref_is_read(self):
        gate = refused("ACTION_NOT_SUPPORTED", cause="wayland_session", physical_delivery_started=False)
        self.assertEqual(self.check("headed", gate), [])
        self.assertTrue(self.check("headed", refused("STALE_REF")))
        self.assertTrue(self.check("headed", refused("ACTION_NOT_SUPPORTED", cause="wayland_session",
                                                     physical_delivery_started=True)))


class DisplayTests(unittest.TestCase):
    def test_the_lowest_monitor_id_is_primary_whatever_the_reply_order(self):
        monitors = [{"id": 3, "name": "HDMI-A-1"}, {"id": 0, "name": "eDP-1"}, {"id": 1, "name": "DP-2"}]
        self.assertEqual(hyprland_common.expected_displays(monitors),
                         [("eDP-1", True), ("DP-2", False), ("HDMI-A-1", False)])

    def test_a_failed_listing_is_never_read_as_an_empty_desktop(self):
        with self.assertRaises(AssertionError):
            hyprland_common.reported_displays(refused("TIMEOUT"))

    def test_a_listing_that_disagrees_with_the_compositor_is_a_problem(self):
        tier = SimpleNamespace()
        app = SimpleNamespace(pid=10, windows=["w1"])
        replies = {
            "list-apps": Envelope(True, {"apps": [{"pid": 10}]}, None, "", 1.0, []),
            "list-windows": Envelope(True, [{"id": "w1"}], None, "", 1.0, []),
            "snapshot": Envelope(True, {"complete": True}, None, "", 1.0, []),
            "status": Envelope(True, {}, None, "", 1.0, []),
        }
        for reply, expected in ((listed(("Virtual-1", True)), []),
                                (listed(("Virtual-1", False)), 1),
                                (listed(("Virtual-1", True), ("AGENT-DESKTOP-1", False)), 1)):
            ctx = SimpleNamespace(tier=tier, product=lambda argv: replies.get(argv[0], reply))
            with self.subTest(reply=reply.data), mock.patch.object(
                    hyprland_desktop, "monitors", return_value=[("Virtual-1", True)]):
                problems = hyprland_desktop._read_only_commands(ctx, app, {})
                self.assertEqual(problems if expected == [] else len(problems), expected)


class DesktopObservationTests(unittest.TestCase):
    def test_a_transient_desktop_change_is_caught_even_when_it_reverts(self):
        states = iter(["calm", "calm", "moved"] + ["calm"] * 1000)
        with hyprland_common.Sampler({"desktop": lambda: next(states)}, interval_s=0.01) as sampler:
            import time
            time.sleep(0.1)
        self.assertTrue(any("moved" in change for change in sampler.changes))

    def test_an_unchanged_desktop_records_no_change(self):
        with hyprland_common.Sampler({"desktop": lambda: "calm"}, interval_s=0.01) as sampler:
            pass
        self.assertEqual(sampler.changes, [])

    def test_focus_must_stay_on_the_bystander_generation(self):
        holder = {"pid": 60, "start": 600, "address": "0xa"}
        with mock.patch.object(hyprland_desktop, "bystander", return_value=holder):
            self.assertEqual(hyprland_desktop.keeps_focus(None, {"active_window": {"address": "0xa", "pid": 60}}), [])
            self.assertTrue(hyprland_desktop.keeps_focus(None, {"active_window": {"address": "0xb", "pid": 60}}))
            self.assertTrue(hyprland_desktop.keeps_focus(None, {"active_window": {"address": "0xa", "pid": 61}}))


class CancelTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.proc = self.root / "proc"
        self.shared = self.root / "run"
        (self.shared / "hypr" / "main_sig").mkdir(parents=True)
        make_proc(self.proc, 50, 500, comm="Hyprland")
        make_proc(self.proc, 60, 600, comm="foot")
        self.parent = {"shared_runtime": str(self.shared), "compositor": [50, 500],
                       "bystander": {"pid": 60, "start": 600}}
        self.child = {"slice": "ad_e2e_7_8.slice", "shared_runtime": str(self.shared), "instances": [],
                      "fixtures": [], "generations": [],
                      "survivors": [[50, 500, "Hyprland", "compositor"], [60, 600, "foot", "bystander"]]}
        self.ready = {"app": [80, 800], "instance": [90, 900], "signature": "nested",
                      "state_root": str(self.root / "gone" / "ad-home"), "members": [[80, 800]]}

    def problems(self):
        return hyprland_cancel.aftermath(self.ready, self.child, self.parent, self.proc)

    def test_a_clean_cancel_has_no_problems(self):
        self.assertEqual(self.problems(), [])

    def test_a_surviving_child_generation_is_reported(self):
        make_proc(self.proc, 80, 800)
        self.assertTrue(any("Calculator" in item for item in self.problems()))

    def test_a_reused_pid_is_not_a_survivor(self):
        make_proc(self.proc, 80, 801)
        self.assertEqual(self.problems(), [])

    def test_a_surviving_second_instance_or_its_directory_is_reported(self):
        make_proc(self.proc, 90, 900)
        self.assertTrue(any("second instance" in item for item in self.problems()))
        make_proc(self.proc, 90, 901)
        (self.shared / "hypr" / "nested").mkdir()
        self.assertTrue(any("instance directory" in item for item in self.problems()))

    def test_a_lost_session_is_reported(self):
        (self.proc / "60" / "stat").unlink()
        self.assertTrue(any("bystander" in item for item in self.problems()))
        (self.proc / "50" / "stat").unlink()
        self.assertTrue(any("compositor" in item for item in self.problems()))

    def test_a_removed_shared_runtime_is_reported(self):
        self.parent["shared_runtime"] = str(self.root / "missing")
        self.assertTrue(any("shared runtime" in item for item in self.problems()))

    def test_the_child_inherits_the_lease_and_its_own_suite(self):
        source = {"AGENT_DESKTOP_INTERACTION_LEASE_FD": "7", "AGENT_DESKTOP_E2E_LOCK_ROOT": "/tmp/x",
                  "AGENT_DESKTOP_E2E_BIN": "/suite/bin/agent-desktop", "AGENT_DESKTOP_E2E_SUITE": "/suite",
                  "HYPRLAND_INSTANCE_SIGNATURE": "main_sig"}
        env, fd = hyprland_cancel.child_env(source, Path("/suite/child"), Path("/suite/child/ready.json"))
        self.assertEqual(fd, 7)
        self.assertNotIn("AGENT_DESKTOP_E2E_LOCK_ROOT", env)
        self.assertNotIn("AGENT_DESKTOP_E2E_BIN", env)
        self.assertEqual(env["AGENT_DESKTOP_E2E_BIN_SRC"], "/suite/bin/agent-desktop")
        self.assertEqual(env["AGENT_DESKTOP_E2E_SUITE"], "/suite/child")
        self.assertEqual(env[hyprland_cancel.CHILD_ENV], "/suite/child/ready.json")
        self.assertEqual(env["HYPRLAND_INSTANCE_SIGNATURE"], "main_sig")

    def test_the_child_needs_an_inherited_lease(self):
        with self.assertRaises(AssertionError):
            hyprland_cancel.child_env({"AGENT_DESKTOP_E2E_BIN": "/b"}, Path("/s"), Path("/s/r"))

    def test_the_child_runs_the_runner_that_cancel_verifies(self):
        from runner import cancel
        self.assertEqual(hyprland_cancel.RUNNER, Path(cancel.__file__).with_name("main.py").resolve())


if __name__ == "__main__":
    unittest.main()
