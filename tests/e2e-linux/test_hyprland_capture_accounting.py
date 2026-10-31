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
from runner.product import Envelope
from runner.verdict import LegContext, evaluate
from scenarios import (hyprland_capture, hyprland_capture_inview as inview, hyprland_capture_refusals as refusals,
                       hyprland_capture_screen as screen)

CAPTURE_LEGS = {
    "capture.hyprland_bound_toplevel_pixels": hyprland_capture,
    "capture.hyprland_hidden_counter_frame_after_request": hyprland_capture,
    "capture.hyprland_static_frame_after_request": hyprland_capture,
    "capture.hyprland_in_view_first_frame": inview,
    "capture.hyprland_screen_exact_output": screen,
    "capture.hyprland_permission_denied_no_dialog": refusals,
    "capture.hyprland_deadline_or_cancel_released": refusals,
}
SPECIAL = "special:ad_e2e"
HOLDER = {"address": "0xby", "pid": 1}
DESKTOP = {"active_window": HOLDER, "workspace": "1", "monitors": [("M-1", True, "1", "")]}


def ok(data=None) -> Envelope:
    return Envelope(ok=True, data=data, error=None, raw="{}", elapsed_ms=5.0, argv=[], exit_code=0)


@contextlib.contextmanager
def yielding(value):
    yield value


class FakeCompositor:
    """Toplevels that start on the launch special workspace and move when told, an output list and one option."""

    def __init__(self):
        self.places: dict[str, str] = {}
        self.pids: dict[str, int] = {}
        self.outputs = ["M-1"]
        self.enforced = False

    def window(self, name: str):
        address, pid = f"0x{name}", 100 + len(self.places)
        self.places[address] = SPECIAL
        self.pids[address] = pid
        return SimpleNamespace(pid=pid), {"address": address, "pid": pid}

    def painted(self, tier, name, *args):
        owned, found = self.window(name)
        return yielding((owned, found, Path("/nonexistent/painter.log")))

    def client(self, tier, address):
        return {"address": address, "workspace": {"name": self.places[address]}, "size": [320, 240],
                "pid": self.pids[address], "monitor": 0}

    def move(self, tier, address, workspace):
        self.places[address] = workspace

    def monitors(self, tier, name):
        return [{"name": output} for output in self.outputs]

    def control(self, tier, build):
        argv = build(False)
        if argv[:2] == ["keyword", refusals.ENFORCE]:
            self.enforced = argv[2] == "true"

    def add_output(self, tier):
        self.outputs.append(screen.EXTRA_OUTPUT)

    def remove_output(self, tier):
        self.outputs = [output for output in self.outputs if output != screen.EXTRA_OUTPUT]


class CaptureLegAccountingTests(unittest.TestCase):
    """Each capture leg, run against a fake compositor, satisfies the runner's oracle accounting."""

    def run_leg(self, leg_id: str) -> list[str]:
        fake = FakeCompositor()
        module = CAPTURE_LEGS[leg_id]
        with tempfile.TemporaryDirectory() as runtime:
            tier = SimpleNamespace(runtime_dir=Path(runtime), env={})
            ctx = LegContext(tier, "headless", leg=legs.leg_by_id(leg_id), member_reader=lambda tier: frozenset())
            ctx.product = lambda argv: ok([{"id": "M-1"}, {"id": screen.EXTRA_OUTPUT}])
            sampler = mock.MagicMock(__enter__=mock.Mock(return_value=SimpleNamespace(changes=[])))
            common = {
                "main_instance": mock.Mock(return_value={}),
                "bystander": mock.Mock(return_value=HOLDER),
                "cursor": mock.Mock(return_value={"x": 0, "y": 0}),
                "desktop": mock.Mock(side_effect=lambda tier, main: dict(DESKTOP)),
                "keeps_focus": mock.Mock(return_value=[]),
                "restore_desktop": mock.Mock(),
                "Sampler": mock.Mock(return_value=sampler),
                "painted": mock.Mock(side_effect=fake.painted),
                "ids_by_pid": mock.Mock(side_effect=lambda envelope, pids: {pid: f"w-{pid}" for pid in pids}),
            }
            per_module = {
                hyprland_capture: {
                    "slow_painted": mock.Mock(side_effect=fake.painted),
                    "move": mock.Mock(side_effect=fake.move),
                    "client": mock.Mock(side_effect=fake.client),
                    "scale_of": mock.Mock(return_value=1.0),
                    "_settled_capture": mock.Mock(return_value=(ok(), object())),
                    "painted_problems": mock.Mock(return_value=[]),
                    "boundary_problems": mock.Mock(return_value=[]),
                    "terminal": mock.Mock(side_effect=lambda tier, name: yielding(fake.window(name))),
                    "listed": mock.Mock(return_value=[{"id": "w-old"}]),
                    "_ticker_round": mock.Mock(return_value=([], (None, 0))),
                    "_slow_round": mock.Mock(return_value=([], (None, 0))),
                    "_short_batch": mock.Mock(return_value=[]),
                    "written": mock.Mock(return_value=[]),
                },
                inview: {
                    "slow_painted": mock.Mock(side_effect=fake.painted),
                    "move": mock.Mock(side_effect=fake.move),
                    "client": mock.Mock(side_effect=fake.client),
                    "_view_round": mock.Mock(return_value=([], (None, 0))),
                    "_still_in_view": mock.Mock(return_value=[]),
                    "written": mock.Mock(return_value=[]),
                },
                screen: {
                    "query": mock.Mock(side_effect=fake.monitors),
                    "_add_output": mock.Mock(side_effect=fake.add_output),
                    "_remove_output": mock.Mock(side_effect=fake.remove_output),
                    "display_problems": mock.Mock(return_value=[]),
                    "_screen_problems": mock.Mock(return_value=[]),
                },
                refusals: {
                    "enforced": mock.Mock(side_effect=lambda tier: fake.enforced),
                    "control": mock.Mock(side_effect=fake.control),
                    "askers": mock.Mock(return_value=set()),
                    "layers": mock.Mock(return_value=[]),
                    "permission_problems": mock.Mock(return_value=[]),
                    "_deadlines": mock.Mock(return_value=[]),
                    "_cancelled": mock.Mock(return_value=[]),
                },
            }
            with contextlib.ExitStack() as stack:
                for name, value in {**common, **per_module[module]}.items():
                    if hasattr(module, name):
                        stack.enter_context(mock.patch.object(module, name, value))
                stack.enter_context(mock.patch.object(hyprland_capture, "client", fake.client))
                legs.implementations()[leg_id](ctx)
            return evaluate(ctx)

    def test_every_capture_leg_is_covered(self):
        import scenarios  # noqa: F401

        registered = {leg_id for leg_id in legs.implementations() if leg_id.startswith("capture.hyprland_")}
        self.assertEqual(registered, set(CAPTURE_LEGS))

    def test_every_recorded_oracle_has_one_before_after_and_restored_read(self):
        import scenarios  # noqa: F401

        for leg_id in CAPTURE_LEGS:
            with self.subTest(leg=leg_id):
                self.assertEqual(self.run_leg(leg_id), [])


if __name__ == "__main__":
    unittest.main()
