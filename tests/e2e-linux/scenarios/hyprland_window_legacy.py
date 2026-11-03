"""A nested Hyprland started from a legacy `hyprland.conf`, for the window legs.

The session compositor and the usual nested instance take a Lua config, so the legacy dispatch
of a window operation needs an instance of its own. It takes the second instance's place: the
running second instance is stopped by its recorded pid and start time, this one is recorded the
same way, and it is stopped the same way after, so teardown and a cancelled run remove it too.
A later leg that needs the second instance starts a fresh one.

A release with no legacy config manager reads the same file and answers in Lua. `lua` says
which dialect the instance answered in, from `eval return 1`, never from the file name.
"""

from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw, hyprland
from runner import second_instance
from runner.helper import require
from scenarios.hyprland_common import shared
from scenarios.wayland_outputs import settle_outputs

START_S = 20.0
CONFIG = "window-legacy-instance.conf"


@contextlib.contextmanager
def legacy_instance(tier):
    """Yields the instance and whether it answers in Lua."""
    from runner import attached, attached_session
    second_instance.stop(tier)
    runtime = shared(tier)
    config = tier.runtime_dir / CONFIG
    config.write_text(second_instance.config_text(False))
    before = set(hyprland.instances(runtime))
    try:
        attached_session.spawn(tier, ["Hyprland", "-c", str(config)], second_instance.launch_env(tier),
                               tier.runtime_dir / "window-legacy-instance.log", second_instance.OUTER_WORKSPACE)
        instance = raw.poll(lambda: second_instance.find_new(attached.load(tier.runtime_dir), before), START_S)
        require(instance, "the legacy config instance did not answer on its IPC socket")
        attached.record_instance(tier.runtime_dir, instance)
        instance["wayland"] = raw.poll(lambda: second_instance.wayland_of(runtime, instance["pid"]), START_S)
        require(instance["wayland"], "the legacy config instance serves no Wayland socket")
        attached.record_instance(tier.runtime_dir, instance)
        lua = hyprland.uses_lua(runtime, instance["signature"])
        hyprland.control(runtime, instance["signature"], ["output", "create", "headless", second_instance.OUTPUT])
        hyprland.control(runtime, instance["signature"], second_instance.output_command(lua))
        settle_outputs(runtime, instance["signature"], lua)
        yield instance, lua
    finally:
        second_instance.stop(tier)
