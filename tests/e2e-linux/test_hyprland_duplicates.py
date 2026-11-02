from __future__ import annotations

import contextlib
import json
import sys
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
from scenarios import duplicates, hyprland_duplicates as route

LEG_ID = "duplicates.two_processes_distinct_window_ids"
FIRST = OwnedApp("gnome-calculator", 401, 11, [":1.40"], ["w-1.40-a"], "")
SECOND = OwnedApp("gnome-calculator", 402, 12, [":1.41"], ["w-1.41-a"], "")
DESKTOP = {"active_window": {"address": "0xb", "pid": 9}, "cursor": {"x": 1, "y": 2}}


def ok(data) -> Envelope:
    return Envelope(ok=True, data=data, error=None, raw=json.dumps({"data": data}), elapsed_ms=5.0,
                    argv=[], exit_code=0)


class FakeContext(LegContext):
    def __init__(self, listed: list[dict]):
        tier = SimpleNamespace(name="hyprland", display="")
        super().__init__(tier, "headless", leg=legs.leg_by_id(LEG_ID), member_reader=lambda _: frozenset())
        self.listed = listed
        self.sent: list[list[str]] = []

    def product(self, argv, *, env=None, **kw):
        self.sent.append(list(argv))
        if argv[0] == "list-windows":
            envelope = ok(self.listed)
        elif argv[0] == "snapshot":
            envelope = ok({"complete": True, "window": {"id": kw["window_id"]}, "tree": {}})
        else:
            raise AssertionError(f"unexpected product call {argv}")
        self._products.append(envelope)
        return envelope


@contextlib.contextmanager
def launched(_ctx, _name, _args=()):
    yield launched.queue.pop(0)


def run(listed: list[dict], toplevels: dict[int, list[str]]) -> FakeContext:
    ctx = FakeContext(listed)
    launched.queue = [FIRST, SECOND]
    patches = (
        mock.patch.object(route, "managed", launched),
        mock.patch.object(route, "main_instance", lambda tier: {}),
        mock.patch.object(route, "desktop", lambda tier, main: dict(DESKTOP)),
        mock.patch.object(route, "keeps_focus", lambda tier, state: []),
        mock.patch.object(route.apps, "owners_named", lambda tier, name: [FIRST, SECOND]),
        mock.patch.object(route, "clients_of",
                          lambda tier, pid: [{"address": address} for address in toplevels.get(pid, [])]),
        mock.patch.object(duplicates, "focus_exact", side_effect=AssertionError("X11 focus on hyprland")),
    )
    with contextlib.ExitStack() as stack:
        for patch in patches:
            stack.enter_context(patch)
        duplicates.two_processes_distinct_window_ids(ctx)
    return ctx


def listing(first_pid: int = 401) -> list[dict]:
    return [{"id": "w-1.40-a", "pid": first_pid}, {"id": "w-1.41-a", "pid": 402}]


class HyprlandDuplicatesTests(unittest.TestCase):
    def test_two_processes_with_their_own_toplevels_pass_without_focus(self):
        ctx = run(listing(), {401: ["0x1"], 402: ["0x2"]})
        self.assertEqual(evaluate(ctx), [])
        self.assertNotIn("focus-window", {argv[0] for argv in ctx.sent})
        self.assertEqual([argv[0] for argv in ctx.sent], ["list-windows", "snapshot", "snapshot"])

    def test_a_window_listed_under_the_other_pid_fails(self):
        with self.assertRaisesRegex(AssertionError, "list-windows names w-1.40-a with pid 402"):
            run(listing(first_pid=402), {401: ["0x1"], 402: ["0x2"]})

    def test_a_process_without_its_own_toplevel_fails(self):
        with self.assertRaisesRegex(AssertionError, "one toplevel per process"):
            run(listing(), {401: ["0x1"], 402: []})

    def test_the_x11_route_still_focuses_each_window(self):
        source = (ROOT / "scenarios" / "duplicates.py").read_text(encoding="utf-8")
        self.assertIn("focus_exact(ctx, app)", source)


if __name__ == "__main__":
    unittest.main()
