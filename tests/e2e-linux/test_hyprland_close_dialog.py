from __future__ import annotations

import contextlib
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.apps import OwnedApp
from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import hyprland_close_dialog as hold

HOLD_LEGS = ("lifecycle.hyprland_close_app_holds_for_a_parented_unrelated_dialog",
             "lifecycle.hyprland_close_app_holds_for_an_unparented_unrelated_dialog")
PENDING = {"code": "ACTION_FAILED", "details": {"cause": "dialog_pending", "dialogs": 1},
           "disposition": {"delivery": "not_delivered", "retry": "safe"}}


def envelope(ok: bool, error: dict | None = None, data: dict | None = None) -> Envelope:
    return Envelope(ok=ok, data=data, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0 if ok else 1)


def toplevel(address: str, pid: int, app_class: str) -> dict:
    return {"address": address, "pid": pid, "class": app_class, "workspace": {"name": "special:ad_e2e"},
            "mapped": True}


HOST = [toplevel("0x70", 70, "org.agentdesktop.DialogHost"), toplevel("0x71", 70, "ad-dialog-host")]


class HoldRegistryTests(unittest.TestCase):
    def test_every_hold_leg_is_headless_hyprland_reads_compositor_and_proc_and_is_implemented(self):
        import scenarios  # noqa: F401

        implemented = legs.implementations()
        for leg_id in HOLD_LEGS:
            with self.subTest(leg=leg_id):
                leg = legs.leg_by_id(leg_id)
                self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headless",), "pass"))
                self.assertTrue({"hyprland", "proc"} <= set(leg.oracles))
                self.assertIn(leg_id, implemented)

    def test_the_dialog_takes_the_program_name_the_leg_places_by_class(self):
        script = hold.HOST_SCRIPT.read_text()
        self.assertIn(f'DIALOG_CLASS = "{hold.DIALOG_CLASS}"', script)
        self.assertIn("GLib.set_prgname(DIALOG_CLASS)", script)
        self.assertNotIn("set_application(", script)

    def test_the_host_is_found_by_the_program_name_its_accessibility_root_carries(self):
        self.assertIn(hold.DIALOG_CLASS, hold.HOST_NAMES)


class HoldProblemTests(unittest.TestCase):
    def test_both_toplevels_kept_a_running_process_and_no_request_pass(self):
        self.assertEqual(hold.hold_problems(HOST, list(HOST), False, "", ([], [])), [])

    def test_a_closed_dialog_an_exit_a_request_or_a_changed_bystander_fails(self):
        problems = hold.hold_problems(HOST, HOST[:1], True, "close-request dialog\n", ([1], [2]))
        self.assertEqual(len(problems), 4)
        self.assertIn("close-request", problems[2])

    def test_only_a_dialog_pending_refusal_that_sent_nothing_passes(self):
        self.assertEqual(hold.pending_report_problems(envelope(False, PENDING)), [])
        sent = {**PENDING, "disposition": {"delivery": "delivered_unverified"}}
        timeout = {"code": "TIMEOUT", "details": {"dialogs": 0}, "disposition": {"delivery": "delivered_unverified"}}
        for report in (envelope(False, sent), envelope(False, timeout),
                       envelope(True, data={"disposition": {"delivery": "delivered_verified"}})):
            with self.subTest(report=report.error or report.data):
                self.assertEqual(len(hold.pending_report_problems(report)), 1)


class HoldLegTests(unittest.TestCase):
    HOLDER = {"pid": 10, "address": "0x10"}

    def run_leg(self, leg_id: str, *, report: dict, requests: str = "") -> list:
        host = OwnedApp("Dialog Host", 70, 700, [], [], "")
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / "requests"
            log.write_text(requests)

            @contextlib.contextmanager
            def host_with(tier, mode):
                yield host, list(HOST), log

            def product(argv, **kw):
                if argv == ["list-apps"]:
                    return envelope(True, data={"apps": [{"name": "ad-dialog-host", "pid": 70}]})
                return envelope(False, report)

            ctx = LegContext(SimpleNamespace(), "headless", leg=legs.leg_by_id(leg_id),
                             member_reader=lambda tier: frozenset())
            ctx.product = product
            sampler = SimpleNamespace(changes=[])
            patches = {
                "main_instance": mock.Mock(return_value={}),
                "bystander": mock.Mock(return_value=self.HOLDER),
                "desktop": mock.Mock(return_value={"active": "0x10"}),
                "keeps_focus": mock.Mock(return_value=[]),
                "dialog_host": mock.Mock(side_effect=host_with),
                "clients": mock.Mock(return_value=[toplevel("0x10", 10, "foot"), *HOST]),
                "clients_of": mock.Mock(side_effect=lambda tier, pid: list(HOST)),
                "exited": mock.Mock(return_value=False),
                "Sampler": mock.Mock(return_value=mock.MagicMock(__enter__=mock.Mock(return_value=sampler))),
            }
            with contextlib.ExitStack() as stack:
                for name, value in patches.items():
                    stack.enter_context(mock.patch.object(hold, name, value))
                stack.enter_context(mock.patch.object(hold.raw, "poll", lambda check, timeout: check()))
                try:
                    legs.implementations()[leg_id](ctx)
                except Exception as error:
                    return [str(error)]
            return evaluate(ctx)

    def test_a_held_close_passes_with_one_before_after_and_restored_read(self):
        import scenarios  # noqa: F401

        for leg_id in HOLD_LEGS:
            with self.subTest(leg=leg_id):
                self.assertEqual(self.run_leg(leg_id, report=PENDING), [])

    def test_a_close_request_the_app_logged_fails_even_with_a_dialog_pending_report(self):
        import scenarios  # noqa: F401

        failures = self.run_leg(HOLD_LEGS[1], report=PENDING, requests="close-request host\n")
        self.assertTrue(failures and "close requests" in failures[0])


if __name__ == "__main__":
    unittest.main()
