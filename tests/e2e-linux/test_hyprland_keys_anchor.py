from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import tier_record
from scenarios import hyprland_keys_common as common

OWNER = ":1.42"


class FakeConnection:
    def __init__(self, events: list[str]):
        self.events = events

    def close_sync(self, _cancellable):
        self.events.append("anchor closed")


def fake_tier(runtime_dir: Path):
    return SimpleNamespace(env={"PATH": "/usr/bin"}, runtime_dir=runtime_dir, a11y_bus_address="unix:path=/bus")


class ListenerAnchorTests(unittest.TestCase):
    def test_the_anchor_listens_to_the_app_on_its_own_connection_until_it_ends(self):
        events: list[str] = []
        connection = FakeConnection(events)
        calls = []
        with mock.patch.object(common.raw, "connect", return_value=connection) as connect, \
                mock.patch.object(common.raw, "call", side_effect=lambda *args: calls.append(args)):
            with common.listener_anchor(fake_tier(Path("/run")), OWNER) as held:
                self.assertIs(held, connection)
                self.assertEqual(events, [])
        connect.assert_called_once_with("unix:path=/bus")
        self.assertEqual(calls, [(connection, common.REGISTRY, common.REGISTRY_PATH, common.REGISTRY,
                                  "RegisterEvent", "(sass)", (common.LISTENED_EVENT, [], OWNER))])
        self.assertEqual(events, ["anchor closed"])

    @unittest.skipUnless(tier_record.installed(), "the attached tier is fork-only")
    def test_the_key_log_holds_the_anchor_from_registration_until_after_its_release(self):
        events: list[str] = []
        owned = SimpleNamespace(owner_names=[OWNER], pid=7, start_time=1, windows=["0x1"], name="Key Log")

        class Anchor:
            def __init__(self, _tier, owner):
                events.append(f"anchor {owner}")

            def __enter__(self):
                return None

            def __exit__(self, *_):
                events.append("anchor closed")

        with tempfile.TemporaryDirectory() as runtime, \
                mock.patch.object(common.apps, "_owners", return_value=[]), \
                mock.patch.object(common.shutil, "which", return_value="/usr/bin/python3"), \
                mock.patch.object(common, "private_interpreter", return_value=Path("/py")), \
                mock.patch("runner.attached_session.spawn"), \
                mock.patch.object(common.raw, "poll", return_value=owned), \
                mock.patch.object(common, "_register", side_effect=lambda _t, o: events.append("registered") or o), \
                mock.patch.object(common, "_release", side_effect=lambda *_: events.append("released")), \
                mock.patch.object(common, "listener_anchor", Anchor):
            with common.key_log(fake_tier(Path(runtime)), "anchor") as (app, _log):
                events.append("leg")
                self.assertIs(app, owned)
        self.assertEqual(events, ["registered", f"anchor {OWNER}", "leg", "released", "anchor closed"])

    @unittest.skipUnless(tier_record.installed(), "the attached tier is fork-only")
    def test_an_anchor_that_cannot_register_still_stops_the_app(self):
        events: list[str] = []
        owned = SimpleNamespace(owner_names=[OWNER], pid=7, start_time=1, windows=["0x1"], name="Key Log")

        def refuse(*_args):
            raise RuntimeError("registry gone")

        with tempfile.TemporaryDirectory() as runtime, \
                mock.patch.object(common.apps, "_owners", return_value=[]), \
                mock.patch.object(common.shutil, "which", return_value="/usr/bin/python3"), \
                mock.patch.object(common, "private_interpreter", return_value=Path("/py")), \
                mock.patch("runner.attached_session.spawn"), \
                mock.patch.object(common.raw, "poll", return_value=owned), \
                mock.patch.object(common.raw, "connect", return_value=FakeConnection(events)), \
                mock.patch.object(common.raw, "call", side_effect=refuse), \
                mock.patch.object(common, "_register", side_effect=lambda _t, o: o), \
                mock.patch.object(common, "_release", side_effect=lambda *_: events.append("released")):
            with self.assertRaisesRegex(RuntimeError, "registry gone"):
                with common.key_log(fake_tier(Path(runtime)), "anchor"):
                    events.append("leg")
        self.assertEqual(events, ["anchor closed", "released"])


if __name__ == "__main__":
    unittest.main()
