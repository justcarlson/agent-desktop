"""Display capture on the attached `hyprland` tier.

`screenshot --screen N` captures the output `list-displays` names at index N
through the compositor's screencopy. A leg adds a headless output left of and
above the session's monitors at a fractional scale, so a negative origin and
a scale that is not whole are both on the desktop, and removes it after.
Each capture is held against `grim`, an independent screencopy client, and
its size and the reported bounds against `j/monitors`.
"""

from __future__ import annotations

import subprocess

from oracle import atspi_raw as raw, hyprland, pixels
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_common import capture_path, saved_image
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_windows import control, cursor, query, restore_desktop

EXTRA_OUTPUT = "AD-E2E-1"
EXTRA_MODE = "1280x800@60"
EXTRA_POSITION = (-1024, -640)
EXTRA_SCALE = 1.25
GRIM_TOLERANCE = 0.02


def output_rule(lua: bool) -> list[str]:
    x, y = EXTRA_POSITION
    if lua:
        return ["eval", f'hl.monitor({{ output = "{EXTRA_OUTPUT}", mode = "{EXTRA_MODE}", '
                        f'position = "{x}x{y}", scale = {EXTRA_SCALE} }})']
    return ["keyword", "monitor", f"{EXTRA_OUTPUT},{EXTRA_MODE},{x}x{y},{EXTRA_SCALE}"]


def expected_display(monitor: dict) -> dict:
    """What `list-displays` reports for one `j/monitors` entry: logical bounds and the scale."""
    width, height = monitor["width"], monitor["height"]
    if monitor.get("transform", 0) % 2 == 1:
        width, height = height, width
    scale = monitor["scale"]
    return {"id": monitor["name"], "scale": scale,
            "bounds": {"x": float(monitor["x"]), "y": float(monitor["y"]),
                       "width": width / scale, "height": height / scale}}


def display_problems(reported: list[dict], monitors: list[dict]) -> list[str]:
    expected = {item["name"]: expected_display(item) for item in monitors}
    problems = []
    if sorted(item.get("id") for item in reported) != sorted(expected):
        problems.append(f"list-displays names {[item.get('id') for item in reported]}, "
                        f"the compositor has {sorted(expected)}")
    for item in reported:
        want = expected.get(item.get("id"))
        if want is None:
            continue
        bounds = item.get("bounds") or {}
        if any(abs(float(bounds.get(key, -1e9)) - value) > 0.01 for key, value in want["bounds"].items()):
            problems.append(f"{item['id']}: bounds {bounds}, the compositor says {want['bounds']}")
        if abs(float(item.get("scale", 0)) - want["scale"]) > 0.001:
            problems.append(f"{item['id']}: scale {item.get('scale')}, the compositor says {want['scale']}")
    return problems


def pixel_size(monitor: dict) -> tuple[int, int]:
    width, height = monitor["width"], monitor["height"]
    return (height, width) if monitor.get("transform", 0) % 2 == 1 else (width, height)


def grim(tier, name: str) -> pixels.Image:
    done = subprocess.run(["grim", "-o", name, "-"], env=dict(tier.env), capture_output=True,
                          timeout=10, check=False)
    require(done.returncode == 0 and done.stdout, f"grim could not capture {name}: {done.stderr[:200]!r}")
    return pixels.decode_png(done.stdout)


def _user_view(state: dict) -> dict:
    """The desktop a person sees, without the output the leg adds."""
    return {**state, "monitors": [item for item in state["monitors"] if item[0] != EXTRA_OUTPUT]}


def _extra_placed(tier) -> bool:
    found = [item for item in query(tier, "monitors") if item["name"] == EXTRA_OUTPUT]
    return bool(found) and (found[0]["x"], found[0]["y"]) == EXTRA_POSITION \
        and abs(found[0]["scale"] - EXTRA_SCALE) < 0.001


def _add_output(tier) -> None:
    control(tier, lambda lua: ["output", "create", "headless", EXTRA_OUTPUT])
    control(tier, output_rule)
    require(raw.poll(lambda: _extra_placed(tier), 5.0),
            f"{EXTRA_OUTPUT} never took its place: {query(tier, 'monitors')}")


def _remove_output(tier) -> None:
    if any(item["name"] == EXTRA_OUTPUT for item in query(tier, "monitors")):
        control(tier, lambda lua: ["output", "remove", EXTRA_OUTPUT])
    require(raw.poll(lambda: all(item["name"] != EXTRA_OUTPUT for item in query(tier, "monitors")), 5.0),
            f"{EXTRA_OUTPUT} was not removed")


def _screen_problems(ctx, index: int, monitor: dict) -> list[str]:
    name = monitor["name"]
    before = grim(ctx.tier, name)
    path = capture_path(ctx.tier, f"screen-{index}")
    envelope = ctx.product(["screenshot", "--screen", str(index), str(path)])
    if not envelope.ok:
        path.unlink(missing_ok=True)
        return [f"--screen {index} ({name}) failed: {envelope.raw[:300]}"]
    image = saved_image(envelope, path)
    after = grim(ctx.tier, name)
    problems = []
    if (image.width, image.height) != pixel_size(monitor):
        problems.append(f"--screen {index} ({name}) is {image.width}x{image.height}, "
                        f"the output is {pixel_size(monitor)} pixels")
    elif not any(pixels.similar(image, oracle, tolerance=GRIM_TOLERANCE) for oracle in (before, after)):
        problems.append(f"--screen {index} ({name}) does not match grim's capture of that output")
    if abs(float((envelope.data or {}).get("scale_factor", 0)) - monitor["scale"]) > 0.001:
        problems.append(f"--screen {index} ({name}) reports scale {(envelope.data or {}).get('scale_factor')}, "
                        f"the output has {monitor['scale']}")
    return problems


@implementation("capture.hyprland_screen_exact_output")
def screen_exact_output(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: _user_view(desktop(tier, main))
    holder, point = bystander(tier), cursor(tier)
    outputs = lambda: sorted(item["name"] for item in query(tier, "monitors"))
    ctx.before("hyprland.desktop", read())
    ctx.before("hyprland.outputs", outputs())
    try:
        _add_output(tier)
        restore_desktop(tier, holder, point)
        with Sampler({"desktop": read}) as sampler:
            monitors = query(tier, "monitors")
            listed = ctx.product(["list-displays"])
            require(listed.ok and isinstance(listed.data, list), f"list-displays failed: {listed.raw[:400]}")
            problems = display_problems(listed.data, monitors)
            by_name = {item["name"]: item for item in monitors}
            for index, item in enumerate(listed.data):
                if item.get("id") in by_name:
                    problems += _screen_problems(ctx, index, by_name[item["id"]])
        ctx.after("hyprland.outputs", outputs())
        ctx.after("hyprland.desktop", read())
        problems += sampler.changes + keeps_focus(tier, desktop(tier, main))
        if EXTRA_OUTPUT not in [item.get("id") for item in listed.data]:
            problems.append(f"list-displays does not list the added {EXTRA_OUTPUT}")
        require(not problems, "; ".join(problems))
    finally:
        _remove_output(tier)
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.outputs", outputs())
    ctx.restored("hyprland.desktop", read())
