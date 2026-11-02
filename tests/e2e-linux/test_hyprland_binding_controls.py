from __future__ import annotations

import importlib
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope

REFUSAL_LEGS = ("session.hyprland_silent_ipc_refused", "session.hyprland_several_instances_ambiguous",
                "session.hyprland_swapped_pair_mismatch")
PASS_LEGS = ("session.hyprland_effects_stay_on_bound_instance", "identity.hyprland_two_processes_one_title")
NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
RUNTIME = Path("/run/user/1000")
MAIN = {"signature": "live", "pid": 10, "wayland": "wayland-1"}
NESTED = {"signature": "nested", "pid": 20, "wayland": "wayland-2"}


def module(name: str):
    return importlib.import_module(f"scenarios.{name}")


def listed(*names: str) -> Envelope:
    data = [{"id": name, "is_primary": index == 0} for index, name in enumerate(names)]
    return Envelope(ok=True, data=data, error=None, raw="[]", elapsed_ms=5.0, argv=[], exit_code=0)


def windows(*entries: dict) -> Envelope:
    return Envelope(ok=True, data=list(entries), error=None, raw="[]", elapsed_ms=5.0, argv=[], exit_code=0)


def refused(code: str = "PLATFORM_NOT_SUPPORTED", *, exit_code: int = 1, elapsed_ms: float = 5.0,
            disposition=None, **details) -> Envelope:
    error = {"code": code, "message": "refused", "details": details,
             "disposition": NOT_DELIVERED if disposition is None else disposition}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=elapsed_ms, argv=[],
                    exit_code=exit_code)


def silent(socket: Path, **overrides) -> Envelope:
    details = {"display_unavailable": "silent", "retryable": True, "detail": str(socket)}
    details.update(overrides.pop("details", {}))
    return refused("TIMEOUT", **overrides, **details)


def tier() -> SimpleNamespace:
    return SimpleNamespace(env={"PATH": "/usr/bin", "HOME": "/home/user", "WAYLAND_DISPLAY": "wayland-1",
                                "HYPRLAND_INSTANCE_SIGNATURE": "live", "DBUS_SESSION_BUS_ADDRESS": "unix:x"},
                           state_root=Path("/tmp/state"), runtime_dir=Path("/tmp/work"))


def context(answer) -> SimpleNamespace:
    return SimpleNamespace(tier=tier(), mode="headless", before=mock.Mock(), after=mock.Mock(),
                           restored=mock.Mock(), product=lambda argv, env, headed: answer(env))


class RegistryTests(unittest.TestCase):
    def test_each_refusal_is_a_headless_hyprland_asserted_refusal_with_a_reason(self):
        for leg_id in REFUSAL_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition),
                                 (("hyprland",), ("headless",), "asserted-refusal"))
                self.assertTrue(leg.reason)
                self.assertIn("hyprland", leg.oracles)
                self.assertIn(leg_id, legs.implementations())

    def test_each_effect_leg_is_a_headless_hyprland_pass_leg(self):
        for leg_id in PASS_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "pass"))
                self.assertIn("hyprland", leg.oracles)
                self.assertIn(leg_id, legs.implementations())


class SilentTests(unittest.TestCase):
    SOCKET = RUNTIME / "hypr" / "quiet" / ".socket.sock"

    def check(self, envelope) -> list[str]:
        return module("hyprland_binding_refusals").silent_problems("x", envelope, self.SOCKET)

    def test_the_documented_silent_refusal_passes(self):
        self.assertEqual(self.check(silent(self.SOCKET)), [])

    def test_each_broken_field_fails(self):
        for name, envelope in {
            "listed": listed("A-1"),
            "code": refused(display_unavailable="silent", retryable=True, detail=str(self.SOCKET)),
            "cause": silent(self.SOCKET, details={"display_unavailable": "hyprland_mismatch"}),
            "retryable": silent(self.SOCKET, details={"retryable": False}),
            "socket": silent(self.SOCKET, details={"detail": "/run/user/1000/hypr/other/.socket.sock"}),
            "exit": silent(self.SOCKET, exit_code=2),
            "disposition": silent(self.SOCKET, disposition={"delivery": "delivered_unverified"}),
            "budget": silent(self.SOCKET, elapsed_ms=2500.0),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(self.check(envelope))


class AmbiguousTests(unittest.TestCase):
    def check(self, envelope) -> list[str]:
        return module("hyprland_binding_refusals").ambiguous_problems("x", envelope, ["live", "nested"])

    def test_both_live_signatures_in_any_order_pass(self):
        self.assertEqual(self.check(refused(display_unavailable="hyprland_ambiguous", signatures=["nested", "live"])),
                         [])

    def test_a_missing_extra_or_absent_signature_list_fails(self):
        for name, envelope in {
            "one": refused(display_unavailable="hyprland_ambiguous", signatures=["live"]),
            "extra": refused(display_unavailable="hyprland_ambiguous", signatures=["live", "nested", "stale"]),
            "absent": refused(display_unavailable="hyprland_ambiguous"),
            "string": refused(display_unavailable="hyprland_ambiguous", signatures="live nested"),
            "cause": refused(display_unavailable="hyprland_mismatch", signatures=["live", "nested"]),
            "listed": listed("A-1"),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(self.check(envelope))


class SwappedTests(unittest.TestCase):
    SOCKET = RUNTIME / "wayland-2"

    def check(self, envelope) -> list[str]:
        return module("hyprland_binding_refusals").swapped_problems("x", envelope, "live", self.SOCKET)

    def test_a_mismatch_naming_the_signature_and_the_socket_passes(self):
        detail = f"Hyprland instance live (pid 10) is bound, but the Wayland socket {self.SOCKET} is served by pid 20"
        self.assertEqual(self.check(refused(display_unavailable="hyprland_mismatch", detail=detail)), [])

    def test_a_mismatch_that_names_only_one_side_fails(self):
        for detail in ("Hyprland instance live is bound", f"the Wayland socket {self.SOCKET} is served by pid 20"):
            with self.subTest(detail=detail):
                self.assertTrue(self.check(refused(display_unavailable="hyprland_mismatch", detail=detail)))

    def test_a_listing_fails(self):
        self.assertTrue(self.check(listed("B-1")))


class SilentLegTests(unittest.TestCase):
    def run_leg(self, answer) -> mock.Mock:
        refusals = module("hyprland_binding_refusals")
        listener = mock.Mock()
        with mock.patch.object(refusals, "shared", return_value=RUNTIME), \
                mock.patch.object(refusals, "main_instance", return_value=MAIN), \
                mock.patch.object(refusals.second_instance, "stop"), \
                mock.patch.object(refusals, "compositor_names", return_value=["A-1"]), \
                mock.patch.object(refusals, "desktop", return_value={"focus": "x"}), \
                mock.patch.object(refusals.socket_fixture, "instance", return_value=("quiet", listener)):
            refusals.hyprland_silent_ipc_refused(context(answer))
        return listener

    @staticmethod
    def contract(env) -> Envelope:
        if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "live":
            return listed("A-1")
        return silent(RUNTIME / "hypr" / "quiet" / ".socket.sock")

    def test_the_leg_passes_and_closes_each_fixture_when_every_quiet_endpoint_refuses(self):
        listener = self.run_leg(self.contract)
        self.assertEqual(listener.close.call_count, 2)

    def test_the_leg_fails_when_discovery_skips_the_quiet_endpoint(self):
        skipping = lambda env: listed("A-1") if "HYPRLAND_INSTANCE_SIGNATURE" not in env else self.contract(env)
        with self.assertRaises(AssertionError):
            self.run_leg(skipping)

    def test_the_leg_fails_when_the_named_endpoint_is_reported_as_a_mismatch(self):
        mismatch = lambda env: (refused(display_unavailable="hyprland_mismatch")
                                if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "quiet" else self.contract(env))
        with self.assertRaises(AssertionError):
            self.run_leg(mismatch)


class SecondInstanceLegTests(unittest.TestCase):
    def run_leg(self, name: str, answer) -> mock.Mock:
        refusals = module("hyprland_binding_refusals")
        stop = mock.Mock()
        names = lambda runtime, signature: ["A-1"] if signature == "live" else ["B-1"]
        with mock.patch.object(refusals, "shared", return_value=RUNTIME), \
                mock.patch.object(refusals, "main_instance", return_value=MAIN), \
                mock.patch.object(refusals.second_instance, "stop", stop), \
                mock.patch.object(refusals.second_instance, "ensure_sized", return_value=NESTED), \
                mock.patch.object(refusals, "compositor_names", side_effect=names), \
                mock.patch.object(refusals, "desktop", return_value={"focus": "x"}):
            getattr(refusals, name)(context(answer))
        return stop

    @staticmethod
    def ambiguous(env) -> Envelope:
        signature = env.get("HYPRLAND_INSTANCE_SIGNATURE")
        if signature is None:
            return refused(display_unavailable="hyprland_ambiguous", signatures=["live", "nested"])
        return listed("A-1") if signature == "live" else listed("B-1")

    @staticmethod
    def swapped(env) -> Envelope:
        signature, wayland = env.get("HYPRLAND_INSTANCE_SIGNATURE"), env.get("WAYLAND_DISPLAY")
        socket = RUNTIME / wayland
        return refused(display_unavailable="hyprland_mismatch",
                       detail=f"Hyprland instance {signature} is bound, but the Wayland socket {socket} is foreign")

    def test_the_ambiguous_leg_passes_and_stops_the_second_instance(self):
        stop = self.run_leg("hyprland_several_instances_ambiguous", self.ambiguous)
        stop.assert_called_once()

    def test_the_ambiguous_leg_fails_when_the_session_display_picks_one_instance(self):
        picking = lambda env: listed("A-1") if env.get("WAYLAND_DISPLAY") == "wayland-1" else self.ambiguous(env)
        with self.assertRaises(AssertionError):
            self.run_leg("hyprland_several_instances_ambiguous", picking)

    def test_the_swapped_leg_passes_both_ways(self):
        stop = self.run_leg("hyprland_swapped_pair_mismatch", self.swapped)
        stop.assert_called_once()

    def test_the_swapped_leg_fails_when_one_way_binds(self):
        binding = lambda env: (listed("B-1") if env.get("HYPRLAND_INSTANCE_SIGNATURE") == "nested"
                               else self.swapped(env))
        with self.assertRaises(AssertionError):
            self.run_leg("hyprland_swapped_pair_mismatch", binding)


class OwnListingTests(unittest.TestCase):
    PREFIX = "w-hypr-10-500-"

    def check(self, envelope, required=frozenset({7}), forbidden=frozenset({30})) -> list[str]:
        return module("hyprland_binding_effects").own_listing_problems("x", envelope, self.PREFIX, {7, 8},
                                                                      set(required), set(forbidden))

    def test_own_toplevels_and_accessible_frames_of_any_pid_pass(self):
        envelope = windows({"id": f"{self.PREFIX}1", "pid": 7, "accessible": False},
                           {"id": "w-4242", "pid": 99, "accessible": True})
        self.assertEqual(self.check(envelope), [])

    def test_a_foreign_id_pid_missing_or_forbidden_entry_fails(self):
        for name, envelope in {
            "foreign id": windows({"id": "w-hypr-20-600-1", "pid": 7, "accessible": False}),
            "foreign pid": windows({"id": f"{self.PREFIX}1", "pid": 7, "accessible": False},
                                   {"id": f"{self.PREFIX}2", "pid": 31, "accessible": False}),
            "missing": windows({"id": f"{self.PREFIX}1", "pid": 8, "accessible": False}),
            "forbidden": windows({"id": f"{self.PREFIX}1", "pid": 7, "accessible": False},
                                 {"id": "w-4242", "pid": 30, "accessible": True}),
            "failed": refused(),
        }.items():
            with self.subTest(name=name):
                self.assertTrue(self.check(envelope))


class ClientsStateTests(unittest.TestCase):
    def client(self, address: str, workspace: str) -> dict:
        return {"address": address, "pid": 7, "mapped": True, "hidden": False, "at": [0, 0], "size": [600, 900],
                "workspace": {"name": workspace}, "floating": False, "fullscreen": 0, "title": "t"}

    def test_the_state_ignores_order_and_sees_a_moved_window(self):
        state = module("hyprland_binding_effects").clients_state
        first, second = self.client("0x1", "1"), self.client("0x2", "2")
        self.assertEqual(state([first, second]), state([second, first]))
        self.assertNotEqual(state([first, second]), state([first, self.client("0x2", "special:x")]))


class OneTitleOneSizeTests(unittest.TestCase):
    def toplevel(self, pid: int, title: str = "Zoo GTK3", size=(600, 900)) -> dict:
        return {"pid": pid, "title": title, "size": list(size)}

    def test_two_processes_with_one_title_and_size_within_the_tolerance_match(self):
        match = module("hyprland_identity_title").one_title_one_size
        self.assertTrue(match([self.toplevel(1), self.toplevel(2, size=(601, 899))]))

    def test_one_process_another_title_another_size_or_a_gone_toplevel_do_not_match(self):
        match = module("hyprland_identity_title").one_title_one_size
        for name, toplevels in {
            "one process": [self.toplevel(1), self.toplevel(1)],
            "title": [self.toplevel(1), self.toplevel(2, title="Other")],
            "size": [self.toplevel(1), self.toplevel(2, size=(600, 700))],
            "gone": [self.toplevel(1), None],
            "one": [self.toplevel(1)],
        }.items():
            with self.subTest(name=name):
                self.assertFalse(match(toplevels))


if __name__ == "__main__":
    unittest.main()
