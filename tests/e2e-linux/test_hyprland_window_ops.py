from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import hyprland_identity as identity, hyprland_window_ops as ops

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}
WINDOW_LEGS = (
    "lifecycle.hyprland_floating_move_resize",
    "lifecycle.hyprland_minimize_restore",
    "lifecycle.hyprland_maximize_restore",
)


def report(delivery: str) -> Envelope:
    data = {"action": "move-window", "disposition": {"delivery": delivery, "retry": "unsafe"}}
    return Envelope(ok=True, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)


def refused(cause: str, *, started=False, disposition=None) -> Envelope:
    error = {"code": "ACTION_NOT_SUPPORTED", "message": "refused",
             "details": {"cause": cause, "physical_delivery_started": started},
             "disposition": NOT_DELIVERED if disposition is None else disposition}
    return Envelope(ok=False, data=None, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=1)


def toplevel(at=(40, 60), size=(500, 400), workspace="31", floating=True, fullscreen=0) -> dict:
    return {"address": "0xa1", "at": list(at), "size": list(size), "floating": floating,
            "fullscreen": fullscreen, "workspace": {"id": 31, "name": workspace}}


class FakeContext:
    def __init__(self, envelope: Envelope):
        self.tier = SimpleNamespace()
        self.envelope = envelope
        self.argv: list[list[str]] = []

    def product(self, argv):
        self.argv.append(argv)
        return self.envelope


class WindowRegistryTests(unittest.TestCase):
    def test_every_window_leg_is_headless_hyprland_reads_the_compositor_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in WINDOW_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "pass"))
                self.assertIn("hyprland", leg.oracles)
                self.assertIn(leg_id, implemented)


class ControlTests(unittest.TestCase):
    def test_float_has_a_lua_and_a_classic_form(self):
        self.assertEqual(ops.float_argv("0xa1", True),
                         ["dispatch", 'hl.dsp.window.float({action="enable",window="address:0xa1"})'])
        self.assertEqual(ops.float_argv("0xa1", False), ["dispatch", "setfloating", "address:0xa1"])

    def test_float_refuses_text_it_cannot_embed(self):
        for lua in (True, False):
            with self.subTest(lua=lua), self.assertRaises(ValueError):
                ops.float_argv('0xa1") os.exit(', lua)


class PlacementTests(unittest.TestCase):
    def test_placement_keeps_only_what_an_operation_changes(self):
        item = toplevel() | {"title": "foot", "pid": 7}
        self.assertEqual(ops.placement(item), {"at": [40, 60], "size": [500, 400], "floating": True,
                                               "fullscreen": 0, "workspace": "31"})
        self.assertEqual(ops.placement(None)["workspace"], None)

    def test_holds_allows_the_tolerance_and_nothing_more(self):
        now = ops.placement(toplevel(at=(42, 58)))
        self.assertTrue(ops.holds(now, {"at": [40, 60], "workspace": "31"}))
        self.assertFalse(ops.holds(now, {"at": [39, 60]}))
        self.assertFalse(ops.holds(now, {"workspace": "32"}))
        self.assertFalse(ops.holds(ops.placement(None), {"size": [500, 400]}))


class OperationVerdictTests(unittest.TestCase):
    def run_operated(self, envelope, reads, expected):
        ctx = FakeContext(envelope)
        with mock.patch.object(ops, "client", side_effect=lambda *_: reads[-1]), \
                mock.patch.object(ops.raw, "poll", return_value=None):
            return ops.operated(ctx, "move", ["move-window"], "0xa1", expected)

    def test_a_verified_report_with_the_read_back_passes(self):
        self.assertEqual(self.run_operated(report("delivered_verified"), [toplevel()], {"at": [40, 60]}), [])

    def test_a_verified_report_the_compositor_does_not_show_fails(self):
        problems = self.run_operated(report("delivered_verified"), [toplevel(at=(0, 0))], {"at": [40, 60]})
        self.assertEqual(len(problems), 1)
        self.assertIn("j/clients", problems[0])

    def test_an_unverified_report_fails_even_when_the_compositor_shows_it(self):
        for envelope in (report("delivered_unverified"), refused("tiled_window")):
            with self.subTest(envelope=envelope.data or envelope.error):
                self.assertEqual(len(self.run_operated(envelope, [toplevel()], {"at": [40, 60]})), 1)

    def run_refusal(self, envelope, reads, cause="tiled_window"):
        ctx = FakeContext(envelope)
        states = iter(reads)
        with mock.patch.object(ops, "client", side_effect=lambda *_: next(states)):
            return ops.refusal(ctx, "tiled move", ["move-window"], "0xa1", cause)

    def test_a_refusal_that_sent_nothing_passes(self):
        self.assertEqual(self.run_refusal(refused("tiled_window"), [toplevel(), toplevel()]), [])

    def test_a_refusal_with_a_changed_target_or_the_wrong_evidence_fails(self):
        cases = {
            "moved": (refused("tiled_window"), [toplevel(), toplevel(at=(0, 0))]),
            "cause": (refused("window_maximized"), [toplevel(), toplevel()]),
            "started": (refused("tiled_window", started=True), [toplevel(), toplevel()]),
            "delivered": (refused("tiled_window", disposition={"delivery": "delivered_unverified",
                                                                "retry": "unsafe"}), [toplevel(), toplevel()]),
            "ok": (report("delivered_verified"), [toplevel(), toplevel()]),
        }
        for name, (envelope, reads) in cases.items():
            with self.subTest(case=name):
                self.assertNotEqual(self.run_refusal(envelope, reads), [])

    def run_satisfied(self, envelope, reads):
        ctx = FakeContext(envelope)
        states = iter(reads)
        with mock.patch.object(ops, "client", side_effect=lambda *_: next(states)):
            return ops.satisfied(ctx, "move again", ["move-window"], "0xa1")

    def test_an_already_satisfied_operation_is_not_delivered_and_changes_nothing(self):
        self.assertEqual(self.run_satisfied(report("not_delivered"), [toplevel(), toplevel()]), [])
        self.assertNotEqual(self.run_satisfied(report("delivered_verified"), [toplevel(), toplevel()]), [])
        self.assertNotEqual(self.run_satisfied(report("not_delivered"), [toplevel(), toplevel(at=(1, 1))]), [])


class TwinBoundaryTests(unittest.TestCase):
    def test_a_bound_twin_on_a_special_workspace_is_refused_for_its_workspace_name(self):
        twins = SimpleNamespace(windows=["w-a", "w-b"])
        ctx = FakeContext(refused("workspace_name_unsupported"))
        self.assertEqual(identity._bound(ctx, twins), [])
        self.assertEqual([argv[0] for argv in ctx.argv], ["minimize", "minimize"])

    def test_a_bound_twin_that_reaches_the_old_wayland_boundary_fails(self):
        twins = SimpleNamespace(windows=["w-a"])
        self.assertNotEqual(identity._bound(FakeContext(refused("wayland_session")), twins), [])


if __name__ == "__main__":
    unittest.main()
