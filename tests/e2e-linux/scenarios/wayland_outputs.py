"""`list-displays` on Wayland, read against the compositor's own answer.

On Hyprland the oracle is `hyprctl -j monitors` against one instance; on the
generic `wayland` tier it is Mutter's `DisplayConfig.GetCurrentState` over the
session bus, read with `busctl`. Each expectation is computed here from the
oracle's reply, never from the product's output.
"""

from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

from oracle import hyprland
from runner import second_instance
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance, shared

SCALE_TOLERANCE = 0.01
SIZE_TOLERANCE = 1.0
FRACTIONAL = 1.25
GONE_INSTANCE = "ad_e2e_no_such_instance"
MUTTER = ("org.gnome.Mutter.DisplayConfig", "/org/gnome/Mutter/DisplayConfig",
          "org.gnome.Mutter.DisplayConfig", "GetCurrentState")
LOGICAL_LAYOUT = 1


def ordered(displays: list[dict]) -> list[dict]:
    """The product's order: primary first, then by id."""
    return sorted(displays, key=lambda item: (not item["is_primary"], item["id"]))


def expected_from_monitors(monitors: list[dict]) -> list[dict]:
    """What `list-displays` must report for a `hyprctl -j monitors` reply.

    Bounds are logical: the mode over the scale, turned with a quarter
    transform. Wayland names no primary, so the output at the layout origin is
    primary, else the top-left one, and the lowest name breaks a tie.
    """
    displays = []
    for monitor in monitors:
        if monitor.get("disabled"):
            continue
        scale = float(monitor["scale"])
        width, height = monitor["width"] / scale, monitor["height"] / scale
        if int(monitor.get("transform", 0)) % 2:
            width, height = height, width
        displays.append({"id": monitor["name"], "x": float(monitor["x"]), "y": float(monitor["y"]),
                         "width": width, "height": height, "scale": scale, "is_primary": False})
    if displays:
        primary = min(displays, key=lambda item: ((item["x"], item["y"]) != (0.0, 0.0),
                                                  item["y"], item["x"], item["id"]))
        primary["is_primary"] = True
    return ordered(displays)


def _variant(value):
    return value["data"] if isinstance(value, dict) and "data" in value and "type" in value else value


def expected_from_mutter(reply: dict) -> list[dict]:
    """What `list-displays` must report for a busctl JSON `GetCurrentState` reply."""
    _, physical, logical, properties = reply["data"]
    logical_layout = _variant(properties.get("layout-mode")) == LOGICAL_LAYOUT
    current = {}
    for spec, modes, _ in physical:
        for mode in modes:
            if _variant(mode[6].get("is-current")) is True:
                current[spec[0]] = (mode[1], mode[2])
    displays = []
    for x, y, scale, _, primary, specs, _ in logical:
        size = next((current[spec[0]] for spec in specs if spec[0] in current), (0, 0))
        width, height = (round(size[0] / scale), round(size[1] / scale)) if logical_layout else size
        displays.append({"id": "+".join(spec[0] for spec in specs), "x": float(x), "y": float(y),
                         "width": float(width), "height": float(height), "scale": float(scale),
                         "is_primary": bool(primary)})
    return ordered(displays)


def display_problems(label: str, envelope, expected: list[dict]) -> list[str]:
    """Each way the product's `list-displays` answer differs from `expected`."""
    if not envelope.ok or not isinstance(envelope.data, list):
        return [f"{label}: list-displays failed: {envelope.raw[:400]}"]
    if not expected:
        return [f"{label}: the oracle reports no display"]
    got = [(item.get("id"), item.get("is_primary") is True) for item in envelope.data]
    want = [(item["id"], item["is_primary"]) for item in expected]
    if got != want:
        return [f"{label}: list-displays names {got}, the compositor {want}"]
    problems = []
    for item, oracle in zip(envelope.data, expected):
        bounds = item.get("bounds") or {}
        for key, tolerance in (("x", 0.0), ("y", 0.0), ("width", SIZE_TOLERANCE), ("height", SIZE_TOLERANCE)):
            if not isinstance(bounds.get(key), (int, float)) or abs(bounds[key] - oracle[key]) > tolerance:
                problems.append(f"{label}: {oracle['id']} {key} is {bounds.get(key)!r}, "
                                f"the compositor {oracle[key]:g}")
        scale = item.get("scale")
        if not isinstance(scale, (int, float)) or abs(scale - oracle["scale"]) > SCALE_TOLERANCE:
            problems.append(f"{label}: {oracle['id']} scale is {scale!r}, the compositor {oracle['scale']:g}")
    return problems


def monitors(runtime: Path, signature: str) -> list[dict]:
    return json.loads(hyprland.control(runtime, signature, ["-j", "monitors"]))


class OutputsNotSettled(RuntimeError):
    """The second instance's outputs never took the config mode: a fixture error, never a product result."""


def monitor_command(lua: bool, output: str, position: str, scale: float) -> list[str]:
    mode = second_instance.MODE
    if lua:
        return ["eval", f'hl.monitor({{ output = "{output}", mode = "{mode}", position = "{position}", scale = {scale:g} }})']
    return ["keyword", "monitor", f"{output},{mode},{position},{scale:g}"]


def scale_command(lua: bool, scale: float) -> list[str]:
    return monitor_command(lua, second_instance.OUTPUT, "1280x0", scale)


def mode_size() -> tuple[int, int]:
    width, height = second_instance.MODE.split("@")[0].split("x")
    return int(width), int(height)


def unsettled(monitors: list[dict]) -> list[dict]:
    """The enabled outputs that do not yet report the config mode's size."""
    return [monitor for monitor in monitors if not monitor.get("disabled")
            and (monitor.get("width"), monitor.get("height")) != mode_size()]


def settle_outputs(runtime: Path, signature: str, lua: bool, seconds: float = 10.0) -> list[dict]:
    """Wait until every enabled output of one instance reports the config mode, applying the rule to any
    that does not. A nested output can start at its outer window's size and take the rule later."""
    for monitor in unsettled(monitors(runtime, signature)):
        hyprland.control(runtime, signature, monitor_command(
            lua, monitor["name"], f"{monitor['x']}x{monitor['y']}", float(monitor["scale"])))
    deadline = time.monotonic() + seconds
    while True:
        current = monitors(runtime, signature)
        if current and not unsettled(current):
            return current
        if time.monotonic() >= deadline:
            raise OutputsNotSettled(f"the second instance's outputs did not take {second_instance.MODE} "
                                    f"within {seconds:g} s: {json.dumps(current)}")
        time.sleep(0.1)


def _scale_of(runtime: Path, signature: str) -> float | None:
    for monitor in monitors(runtime, signature):
        if monitor["name"] == second_instance.OUTPUT:
            return float(monitor["scale"])
    return None


def _set_scale(runtime: Path, signature: str, lua: bool, scale: float, seconds: float = 5.0) -> None:
    hyprland.control(runtime, signature, scale_command(lua, scale))
    deadline = time.monotonic() + seconds
    while _scale_of(runtime, signature) != scale:
        require(time.monotonic() < deadline,
                f"the second instance never took scale {scale:g}: {monitors(runtime, signature)}")
        time.sleep(0.1)


@implementation("observation.hyprland_displays_match_monitors")
def hyprland_displays_match_monitors(ctx: LegContext) -> None:
    runtime = shared(ctx.tier)
    main = main_instance(ctx.tier)
    nested = second_instance.ensure(ctx.tier)
    lua = hyprland.uses_lua(runtime, main["signature"])
    read_main = lambda: monitors(runtime, main["signature"])
    read_nested = lambda: monitors(runtime, nested["signature"])
    nested_env = dict(ctx.tier.env, WAYLAND_DISPLAY=nested["wayland"])
    settle_outputs(runtime, nested["signature"], lua)
    ctx.before("hyprland.monitors", read_main())
    ctx.before("hyprland.second_monitors", read_nested())
    problems = display_problems("session", ctx.product(["list-displays"]), expected_from_monitors(read_main()))
    two = expected_from_monitors(read_nested())
    require(len(two) == 2, f"the second instance has no two-output layout: {read_nested()}")
    problems += display_problems("second instance", ctx.product(["list-displays"], env=nested_env), two)
    try:
        _set_scale(runtime, nested["signature"], lua, FRACTIONAL)
        scaled = read_nested()
        problems += display_problems("fractional scale", ctx.product(["list-displays"], env=nested_env),
                                     expected_from_monitors(scaled))
        ctx.after("hyprland.monitors", read_main())
        ctx.after("hyprland.second_monitors", scaled)
    finally:
        _set_scale(runtime, nested["signature"], lua, 1.0)
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.monitors", read_main())
    ctx.restored("hyprland.second_monitors", read_nested())


def mutter_state(env: dict[str, str]) -> dict:
    result = subprocess.run(["busctl", f"--address={env['DBUS_SESSION_BUS_ADDRESS']}", "--json=short",
                             "call", *MUTTER], env=env, capture_output=True, text=True, timeout=5, check=False)
    if result.returncode != 0:
        raise AssertionError(f"Mutter DisplayConfig did not answer: {result.stderr.strip()}")
    return json.loads(result.stdout)


def hyprland_like_envs(env: dict[str, str]) -> dict[str, dict[str, str]]:
    return {
        "a named Hyprland instance that is gone": dict(env, HYPRLAND_INSTANCE_SIGNATURE=GONE_INSTANCE),
        "a Hyprland desktop name": dict(env, XDG_CURRENT_DESKTOP="Hyprland"),
        "both": dict(env, HYPRLAND_INSTANCE_SIGNATURE=GONE_INSTANCE, XDG_CURRENT_DESKTOP="Hyprland"),
    }


@implementation("session.other_wayland_route_unchanged")
def other_wayland_route_unchanged(ctx: LegContext) -> None:
    read = lambda: expected_from_mutter(mutter_state(ctx.tier.env))
    expected = read()
    ctx.before("dbus.display_config", expected)
    baseline = ctx.product(["list-displays"], headed=False)
    problems = display_problems("GNOME", baseline, expected)
    for label, env in hyprland_like_envs(ctx.tier.env).items():
        listed = ctx.product(["list-displays"], env=env, headed=False)
        if not listed.ok or listed.data != baseline.data:
            problems.append(f"{label}: list-displays became {listed.raw[:400]}")
    ctx.after("dbus.display_config", read())
    require(not problems, "; ".join(problems))
    ctx.restored("dbus.display_config", read())
