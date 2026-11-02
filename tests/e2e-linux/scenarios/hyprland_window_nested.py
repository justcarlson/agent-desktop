"""Window operations on a nested Hyprland instance the run owns.

The nested instance holds no part of the user's desktop, so its output can move to a negative
origin and a fractional scale, and the sizer there can take focus. Each case reads the sizer
back from that instance's `j/clients` and from the sizer's own log: the log proves the size and
toplevel state the app was given, `j/clients` the position. The same cases run on a Lua and on
a legacy config instance; `lua` names the dialect the instance answers in.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner.helper import require
from scenarios.hyprland_focus_controls import focus_argv, listed_ids
from scenarios.hyprland_pointer import FRACTIONAL, NEGATIVE_ORIGIN, monitor_of, moved_output
from scenarios.hyprland_pointer_pad import instance_control, instance_env, instance_query
from scenarios.hyprland_window_app import (client_of, quiet_problems, reader, settled_count, size_problems, sizer,
                                           state_problems)
from scenarios.hyprland_window_checks import (SETTLE_S, float_argv, holds, operated, refusal, satisfied,
                                              window_argv)

PLAIN_SIZE = (500, 400)
SCALED_SIZES = ((480, 360), (333, 221))
LEGACY_MAXIMIZE = ["dispatch", "fullscreen", "1", "set"]
LEGACY_UNSET = ["dispatch", "fullscreen", "1", "unset"]


class Sized:
    """The sizer on one instance: its id, how to read it back, and the product's environment."""

    def __init__(self, ctx, instance: dict, lua: bool, found: dict, log):
        self.ctx, self.instance, self.lua, self.log = ctx, instance, lua, log
        self.address = found["address"]
        self.env = instance_env(ctx.tier, instance)
        self.read = reader(ctx.tier, instance, self.address)
        ids = listed_ids(ctx.product(["list-windows"], env=self.env, headed=False), found["pid"])
        require(len(ids) == 1, f"list-windows gives the sizer {ids}")
        self.window_id = ids[0]

    def argv(self, verb: str, *extra: str) -> list[str]:
        return window_argv(verb, self.window_id, *extra)

    def operated(self, label: str, argv: list[str], expected: dict) -> list[str]:
        return operated(self.ctx, label, argv, self.read, expected, env=self.env)

    def refused(self, label: str, argv: list[str], cause: str) -> list[str]:
        count = settled_count(self.log)
        problems = refusal(self.ctx, label, argv, self.read, cause, env=self.env)
        return problems + quiet_problems(label, self.log, count)

    def satisfied(self, label: str, argv: list[str]) -> list[str]:
        return satisfied(self.ctx, label, argv, self.read, env=self.env)

    def control(self, argv: list[str]) -> None:
        instance_control(self.ctx.tier, self.instance, argv)

    def monitor(self) -> dict:
        found = client_of(self.ctx.tier, self.instance, self.address)
        require(found, f"the sizer {self.address} has no toplevel")
        return monitor_of(self.ctx.tier, self.instance, found)

    def floated(self) -> dict:
        self.control(float_argv(self.address, self.lua))
        require(raw.poll(lambda: self.read()["floating"] is True, SETTLE_S), f"{self.address} did not float")
        return self.read()

    def focused(self) -> None:
        self.control(focus_argv(self.address, self.lua))
        require(raw.poll(lambda: instance_query(self.ctx.tier, self.instance, "activewindow").get("address")
                         == self.address, SETTLE_S), f"{self.address} did not take focus")


def placed(sized: Sized, label: str, at: list[int], size: list[int]) -> list[str]:
    """Move, then resize, which keeps the corner; read both back from `j/clients` and the size
    from the app's log. A move to where the window already is must be satisfied."""
    move = sized.argv("move-window", "--x", str(at[0]), "--y", str(at[1]))
    if holds(sized.read(), {"at": list(at)}):
        problems = sized.satisfied(f"{label} move", move)
    else:
        problems = sized.operated(f"{label} move", move, {"at": list(at)})
    problems += sized.operated(f"{label} resize", sized.argv("resize-window", "--width", str(size[0]),
                                                             "--height", str(size[1])),
                               {"at": list(at), "size": list(size)})
    return problems + size_problems(label, sized.log, list(size))


def inside(client: dict, monitor: dict) -> bool:
    (x, y), (width, height) = client["at"], client["size"]
    right = monitor["x"] + monitor["width"] / monitor["scale"]
    bottom = monitor["y"] + monitor["height"] / monitor["scale"]
    return monitor["x"] <= x and x + width <= right + 1 and monitor["y"] <= y and y + height <= bottom + 1


def geometry_cases(ctx, instance: dict, lua: bool, label: str) -> list[str]:
    """Move and resize a floating sizer, at a negative origin and a fractional scale too, then give
    it its first geometry back."""
    tier = ctx.tier
    with sizer(tier, instance, f"geometry-{label}") as (_, found, log):
        sized = Sized(ctx, instance, lua, found, log)
        problems = sized.refused(f"{label} tiled move", sized.argv("move-window", "--x", "40", "--y", "60"),
                                 "tiled_window")
        first = sized.floated()
        problems += size_problems(f"{label} floated", log, first["size"])
        problems += placed(sized, f"{label} plain", [40, 60], list(PLAIN_SIZE))
        problems += sized.satisfied(f"{label} move again", sized.argv("move-window", "--x", "40", "--y", "60"))
        output = sized.monitor()["name"]
        with moved_output(tier, instance, lua, output, NEGATIVE_ORIGIN, FRACTIONAL):
            monitor = sized.monitor()
            corner = [int(monitor["x"]) + 40, int(monitor["y"]) + 60]
            require(corner[0] < 0 and corner[1] < 0, f"{output} did not move to a negative origin: {monitor}")
            for size in SCALED_SIZES:
                problems += placed(sized, f"{label} at {corner} scale {FRACTIONAL:g}", corner, list(size))
        problems += placed(sized, f"{label} restored", first["at"], first["size"])
    return problems


def _maximized_cases(sized: Sized, label: str, first: dict) -> list[str]:
    problems = sized.operated(f"{label} maximize", sized.argv("maximize"),
                              {"fullscreen": 1, "workspace": first["workspace"]})
    problems += size_problems(f"{label} maximized", sized.log, sized.read()["size"])
    problems += state_problems(f"{label} maximized", sized.log, True)
    problems += sized.satisfied(f"{label} maximize again", sized.argv("maximize"))
    problems += sized.refused(f"{label} move while maximized", sized.argv("move-window", "--x", "40", "--y", "60"),
                              "window_maximized")
    problems += sized.operated(f"{label} restore", sized.argv("restore"),
                               {"fullscreen": 0, "at": first["at"], "size": first["size"],
                                "workspace": first["workspace"]})
    problems += size_problems(f"{label} restored", sized.log, first["size"])
    return problems + state_problems(f"{label} restored", sized.log, False)


def _changed_output(sized: Sized, label: str, first: dict) -> list[str]:
    """Maximize, move the output to a negative origin at a fractional scale, then restore: the
    window keeps its workspace and size and lies inside the moved output."""
    problems = sized.operated(f"{label} maximize", sized.argv("maximize"), {"fullscreen": 1})
    output = sized.monitor()["name"]
    with moved_output(sized.ctx.tier, sized.instance, sized.lua, output, NEGATIVE_ORIGIN, FRACTIONAL):
        monitor = sized.monitor()
        problems += sized.operated(f"{label} restore on the moved output", sized.argv("restore"),
                                   {"fullscreen": 0, "size": first["size"], "workspace": first["workspace"]})
        now = client_of(sized.ctx.tier, sized.instance, sized.address) or {}
        if not now or not inside(now, monitor):
            problems.append(f"{label}: the restored window at {now.get('at')} {now.get('size')} "
                            f"is outside {output} {monitor}")
        problems += size_problems(f"{label} restored on the moved output", sized.log, first["size"])
        problems += state_problems(f"{label} restored on the moved output", sized.log, False)
    return problems


def _legacy_maximize_cases(sized: Sized, label: str) -> list[str]:
    problems = sized.refused(f"{label} legacy maximize", sized.argv("maximize"), "legacy_config_focused_only")
    sized.focused()
    sized.control(LEGACY_MAXIMIZE)
    require(raw.poll(lambda: sized.read()["fullscreen"] == 1, SETTLE_S), f"{sized.address} did not maximize")
    try:
        problems += sized.satisfied(f"{label} legacy maximize of a maximized window", sized.argv("maximize"))
        problems += sized.refused(f"{label} legacy restore", sized.argv("restore"), "legacy_config_focused_only")
    finally:
        sized.control(LEGACY_UNSET)
    require(raw.poll(lambda: sized.read()["fullscreen"] == 0, SETTLE_S), f"{sized.address} stayed maximized")
    return problems


def maximize_cases(ctx, instance: dict, lua: bool, label: str) -> list[str]:
    """Maximize and restore a focused floating sizer, also across an output change. A legacy
    config instance refuses both and changes nothing."""
    with sizer(ctx.tier, instance, f"maximize-{label}") as (_, found, log):
        sized = Sized(ctx, instance, lua, found, log)
        first = sized.floated()
        if not lua:
            return _legacy_maximize_cases(sized, label)
        sized.focused()
        problems = _maximized_cases(sized, label, first)
        return problems + _changed_output(sized, label, sized.read())


def minimize_cases(ctx, instance: dict, lua: bool, label: str) -> list[str]:
    """Minimize and restore a floating sizer on its numbered workspace."""
    with sizer(ctx.tier, instance, f"minimize-{label}") as (_, found, log):
        sized = Sized(ctx, instance, lua, found, log)
        first = sized.floated()
        origin = first["workspace"]
        problems = sized.operated(f"{label} minimize", sized.argv("minimize"),
                                  {"workspace": f"special:agent-desktop-minimized-{origin}"})
        problems += sized.satisfied(f"{label} minimize again", sized.argv("minimize"))
        problems += sized.operated(f"{label} restore", sized.argv("restore"),
                                   {"workspace": origin, "at": first["at"], "size": first["size"], "fullscreen": 0})
        return problems + size_problems(f"{label} restored", log, first["size"])
