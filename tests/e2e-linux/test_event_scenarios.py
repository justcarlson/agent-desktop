import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from runner import legs, verdict
from runner.product import Envelope
from runner.verdict import LegContext
from scenarios import events


class EventScenarioTests(unittest.TestCase):
    def test_process_events_read_transitions_and_restore_private_registry_state(self):
        for kind in ("app_launched_transition", "app_terminated_transition", "app_launched_quiet",
                     "app_terminated_quiet", "target_killed_mid_wait"):
            with self.subTest(leg=kind):
                tier = SimpleNamespace(runtime_dir=Path("/private/test"))
                ctx = LegContext(tier, "headless", legs.leg_by_id("events." + kind),
                                 member_reader=lambda _: frozenset())
                owners, pending = set(), []
                def launch(tier, name, *a):
                    owner = "gnome-calculator" if name == "Calculator" else "gnome-text-editor"
                    owners.add(owner)
                    return SimpleNamespace(name=owner)
                def stop(app):
                    owners.discard(app.name)
                def delayed(seconds, action):
                    pending.append(action)
                    return SimpleNamespace(join=lambda: None)
                def product(tier, argv, **kw):
                    for action in pending:
                        action()
                    pending.clear()
                    if kind.endswith("quiet"):
                        return Envelope(False, None, {"code": "TIMEOUT"}, "{}", 1500, argv)
                    token = argv[argv.index("--event") + 1]
                    return Envelope(True, {"found": True, "event": {"kind": token}}, None, "{}", 1000, argv)
                with patch.object(events, "launch", side_effect=launch), \
                    patch.object(events, "stop", side_effect=stop), \
                    patch.object(events, "app_root", return_value=object()), \
                    patch.object(events, "registry", side_effect=lambda _: [SimpleNamespace(name=n) for n in owners]), \
                    patch.object(events, "delayed", side_effect=delayed), \
                    patch("runner.verdict.run_product", side_effect=product):
                    getattr(events, kind)(ctx)
                expected = []
                self.assertEqual(verdict.evaluate(ctx), expected)
                self.assertEqual(owners, set())

    def test_new_window_cycle_closes_only_added_frame_and_preserves_seed(self):
        tier = SimpleNamespace(runtime_dir=Path("/private/test"))
        ctx = LegContext(tier, "headless", legs.leg_by_id("events.open_and_close_inside_one_wait"),
                         member_reader=lambda _: frozenset())
        frames, pending, stopped = ["seed"], [], []
        app = SimpleNamespace(name="editor")
        root = SimpleNamespace(children=lambda: list(frames))
        def close(tier, existing):
            self.assertEqual(existing, ["seed"])
            frames[:] = existing
        def product(tier, argv, **kw):
            pending.pop()()
            return Envelope(True, {"event": {"kind": "window_closed"}}, None, "{}", 1000, argv)
        with patch.object(events, "_editor", return_value=(app, root)), \
            patch.object(events, "_window_ids", side_effect=lambda _: list(frames)), \
            patch.object(events, "_new_editor_window", side_effect=lambda _: frames.append("extra")), \
            patch.object(events, "_close_added_windows", side_effect=close), \
            patch.object(events, "root_windows", side_effect=lambda *a: len(frames)), \
            patch.object(events, "stop", side_effect=stopped.append), \
            patch.object(events, "delayed", side_effect=lambda delay, action: (pending.append(action) or SimpleNamespace(join=lambda: None))), \
            patch("runner.verdict.run_product", side_effect=product):
            events.open_and_close_inside_one_wait(ctx)
        self.assertEqual(frames, ["seed"])
        self.assertEqual(stopped, [app])
        self.assertEqual(verdict.evaluate(ctx), [])
