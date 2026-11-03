"""The zoo launch for the ref-contract legs on the attached `hyprland` tier.

The tier passes apps no DISPLAY and `apps.launch` runs Qt on xcb on every
tier but `wayland`, so a Qt zoo started there never maps a window and the leg
times out before its first read. This route starts the Qt zoo on the
instance's XWayland display, as `parallel_zoo` does for its Qt agent, and
leaves every other zoo and tier to `managed`.
"""

from __future__ import annotations

import contextlib
import dataclasses

from oracle import atspi_raw as raw
from runner import apps
from runner.helper import managed, require
from scenarios.hyprland_common import main_instance
from scenarios.hyprland_common import x_display

QT_ZOO = "Zoo Qt"
LAUNCH_S = 20.0


def xwayland_tier(tier):
    """`tier` with DISPLAY set to the X display its compositor's XWayland serves."""
    instance = main_instance(tier)
    display = raw.poll(lambda: x_display(instance), LAUNCH_S)
    require(display, f"no XWayland display names the compositor {instance['pid']}")
    return dataclasses.replace(tier, env={**tier.env, "DISPLAY": display})


@contextlib.contextmanager
def zoo_app(ctx, name: str, args: tuple[str, ...]):
    """The zoo `name`, stopped on the way out."""
    if getattr(ctx.tier, "name", None) != "hyprland" or name != QT_ZOO:
        with managed(ctx, name, args) as app:
            yield app
        return
    app = apps.launch(xwayland_tier(ctx.tier), name, args)
    try:
        yield app
    finally:
        apps.stop(app)
