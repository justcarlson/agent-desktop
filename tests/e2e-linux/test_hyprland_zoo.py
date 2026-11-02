from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import apps
from runner.session import Tier
from scenarios import ref_contract

INSTANCE = {"pid": 4242}


def tier(name: str, runtime: Path) -> Tier:
    return Tier(name, runtime, None, None, "unix:path=/bus", "unix:path=/a11y", runtime / "state",
                runtime / "key", {"PATH": "/usr/bin", "QT_QPA_PLATFORM": "wayland"})


def run(tier_name: str, leg_id: str) -> tuple[list[Tier], list[str], Tier]:
    """The tiers `apps.launch` saw and the apps `apps.stop` stopped for one zoo leg."""
    with tempfile.TemporaryDirectory() as raw_dir:
        owned = tier(tier_name, Path(raw_dir))
        ctx = SimpleNamespace(tier=owned, leg=SimpleNamespace(id=leg_id))
        launched: list[Tier] = []
        stopped: list[str] = []

        def launch(given, name, args=()):
            launched.append(given)
            return SimpleNamespace(name=name)

        with mock.patch.object(apps, "launch", launch), \
                mock.patch.object(apps, "stop", lambda app: stopped.append(app.name)), \
                mock.patch.object(ref_contract, "reset", lambda path: None), \
                mock.patch("scenarios.hyprland_zoo.main_instance", lambda given: INSTANCE), \
                mock.patch("scenarios.hyprland_zoo.x_display",
                           lambda instance: ":7" if instance is INSTANCE else ""):
            with ref_contract.zoo(ctx) as (app, _):
                pass
        return launched, stopped, owned


class HyprlandZooTests(unittest.TestCase):
    def test_the_qt_zoo_runs_on_the_instance_xwayland_display(self):
        """The tier passes no DISPLAY and `apps.launch` runs Qt on xcb, so without it the zoo never maps."""
        launched, stopped, owned = run("hyprland", "ref_contract.zoo_qt")
        self.assertEqual(len(launched), 1)
        self.assertEqual(launched[0].env["DISPLAY"], ":7")
        self.assertEqual(launched[0].runtime_dir, owned.runtime_dir)
        self.assertNotIn("DISPLAY", owned.env)
        self.assertEqual(stopped, ["Zoo Qt"])

    def test_the_qt_windows_leg_takes_the_same_route(self):
        launched, _, _ = run("hyprland", "ref_contract.zoo_qt_windows")
        self.assertEqual(launched[0].env["DISPLAY"], ":7")

    def test_gtk_zoos_on_hyprland_keep_the_tier_launch(self):
        for leg_id in ("ref_contract.zoo_gtk3", "ref_contract.zoo_gtk4_windows"):
            with self.subTest(leg=leg_id):
                launched, stopped, owned = run("hyprland", leg_id)
                self.assertIs(launched[0], owned)
                self.assertEqual(len(stopped), 1)

    def test_other_tiers_keep_the_tier_launch_for_qt(self):
        for tier_name in ("x11-openbox", "x11-mutter", "wayland"):
            with self.subTest(tier=tier_name):
                launched, _, owned = run(tier_name, "ref_contract.zoo_qt")
                self.assertIs(launched[0], owned)

    def test_no_xwayland_display_refuses_before_any_launch(self):
        from scenarios import hyprland_zoo

        with mock.patch.object(hyprland_zoo, "LAUNCH_S", 0.0), \
                mock.patch.object(hyprland_zoo, "x_display", lambda instance: ""), \
                mock.patch.object(hyprland_zoo, "main_instance", lambda given: INSTANCE), \
                mock.patch.object(apps, "launch", side_effect=AssertionError("launched")), \
                tempfile.TemporaryDirectory() as raw_dir:
            ctx = SimpleNamespace(tier=tier("hyprland", Path(raw_dir)),
                                  leg=SimpleNamespace(id="ref_contract.zoo_qt"))
            with self.assertRaisesRegex(AssertionError, "no XWayland display names the compositor 4242"):
                with ref_contract.zoo(ctx):
                    pass


if __name__ == "__main__":
    unittest.main()
