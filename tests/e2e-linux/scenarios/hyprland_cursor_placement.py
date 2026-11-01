"""Where the agent cursor is drawn across outputs on the attached `hyprland` tier.

The leg adds the screen capture leg's headless output left of and above the
session's monitor, at scale 1.25, while a renderer already runs, and opens
Calculator's special workspace on each output in turn. A headless click
places the cursor on the button it clicks. `j/layers` says which output holds
the cursor surface and where, `grim` of that output says the arrow is drawn at
the button at the output's scale, `j/clients` and `j/monitors` say where the
button is and whether it is shown, and `j/cursorpos` says the OS pointer never
moves. A click on Calculator while its workspace is hidden must draw no
cursor on the output that is shown. The command's own `ok` is never the
evidence.
"""

from __future__ import annotations

from oracle import atspi_raw as raw, hyprland, pixels
from runner.helper import display, managed, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_screen import EXTRA_OUTPUT, _add_output, _remove_output, grim
from scenarios.hyprland_common import bystander, main_instance, shared
from scenarios.hyprland_cursor import (SETTLE_S, _frame, alive, app_shown, ended, focused_monitor, live_button,
                                       monitor_at, outcome_problems, renderer_of, started)
from scenarios.hyprland_cursor_agents import surfaces_of, tip_at
from scenarios.hyprland_cursor_hide import undo_all
from scenarios.hyprland_focus_controls import cursor, restore_desktop, shown

DIGITS_ENTERED = 5
SEARCH = 48
TIP_SLACK_PX = 3
TIP_FADE_PX = 7
RATIO_SLACK = 0.12
CHANNEL_DELTA = 64


def monitors_of(tier) -> list[dict]:
    main = main_instance(tier)
    return hyprland.query(shared(tier), main["signature"], "monitors", pid=main["pid"])


def outputs(tier) -> list[tuple]:
    return sorted((item["name"], item["x"], item["y"], round(float(item["scale"]), 3)) for item in monitors_of(tier))


def named(monitors: list[dict], name: str) -> dict:
    found = [item for item in monitors if item["name"] == name]
    require(len(found) == 1, f"{name} is listed {len(found)} times")
    return found[0]


def search_region(monitor: dict, point: tuple[float, float], size: tuple[int, int]) -> tuple[int, int, int, int]:
    """The physical region of an output image where an arrow with its tip at `point` can be drawn."""
    scale = float(monitor["scale"])
    x = max(0, int((point[0] - monitor["x"]) * scale) - 2 * TIP_SLACK_PX)
    y = max(0, int((point[1] - monitor["y"]) * scale) - 2 * TIP_SLACK_PX)
    side = int(SEARCH * scale)
    return (x, y, max(1, min(side, size[0] - x)), max(1, min(side, size[1] - y)))


def drawn_box(shown_image: pixels.Image, bare: pixels.Image,
              region: tuple[int, int, int, int]) -> tuple[int, int, int, int] | None:
    """The bounding box of the pixels in `region` that differ between the two images, or None."""
    x, y, width, height = region
    found = [(column, row) for row in range(y, y + height) for column in range(x, x + width)
             if max(abs(a - b) for a, b in zip(shown_image.pixel(column, row), bare.pixel(column, row)))
             > CHANNEL_DELTA]
    if not found:
        return None
    columns, rows = [item[0] for item in found], [item[1] for item in found]
    return (min(columns), min(rows), max(columns) - min(columns) + 1, max(rows) - min(rows) + 1)


def tip_problems(label: str, box: tuple[int, int, int, int] | None, monitor: dict,
                 point: tuple[float, float]) -> list[str]:
    """The arrow's top left must sit on the point, in the output's physical pixels.

    The arrow's left edge is upright, so its column is exact. Its tip is a
    point, and the first rows under it cover too little of a pixel to pass
    the channel threshold, so the drawn top may sit a few rows below it.
    """
    if box is None:
        return [f"{label}: grim of {monitor['name']} shows no cursor at {point}"]
    scale = float(monitor["scale"])
    want = ((point[0] - monitor["x"]) * scale, (point[1] - monitor["y"]) * scale)
    if abs(box[0] - want[0]) > TIP_SLACK_PX or not -TIP_SLACK_PX <= box[1] - want[1] <= TIP_FADE_PX:
        return [f"{label}: the arrow on {monitor['name']} starts at {box[:2]}, the point is at {want} physical"]
    return []


def scale_problems(drawn: dict[str, tuple[tuple[int, int, int, int], float]]) -> list[str]:
    """The arrow's physical size on each output must follow that output's scale."""
    (first, (box_a, scale_a)), (second, (box_b, scale_b)) = sorted(drawn.items())
    require(abs(scale_a - scale_b) > 0.1, f"{first} and {second} have one scale, so the leg proves nothing")
    want = scale_b / scale_a
    problems = []
    for axis, label in ((2, "width"), (3, "height")):
        ratio = box_b[axis] / box_a[axis]
        if abs(ratio - want) > RATIO_SLACK:
            problems.append(f"the arrow's {label} is {box_a[axis]} px on {first} at {scale_a:g} and {box_b[axis]} px "
                            f"on {second} at {scale_b:g}: ratio {ratio:.2f}, not {want:.2f}")
    return problems


def layer_problems(label: str, surfaces: list[dict], owner_pid: int, monitor: str,
                   point: tuple[float, float]) -> list[str]:
    """One surface of the renderer on `monitor` with its tip on the point, and none on another output."""
    own = [surface for surface in surfaces if surface["pid"] == owner_pid]
    at_tip = [surface for surface in own if tip_at(surface, point)]
    if [surface["monitor"] for surface in at_tip] == [monitor]:
        return []
    return [f"{label}: expected one cursor surface at {point}, on {monitor} alone, found {own}"]


def pointer_problems(label: str, before: dict, after: dict) -> list[str]:
    return [] if before == after else [f"{label}: the OS pointer moved from {before} to {after}"]


def hidden_problems(label: str, drawn: pixels.Image, bare: pixels.Image, monitor: dict,
                    points: list[tuple[float, float]]) -> list[str]:
    """No arrow anywhere a parked cursor could be drawn on the output that is shown."""
    problems = []
    for point in points:
        box = drawn_box(drawn, bare, search_region(monitor, point, (drawn.width, drawn.height)))
        if box is not None:
            problems.append(f"{label}: grim of {monitor['name']} shows a cursor near {point} at {box}")
    return problems


def _click(ctx: LegContext, session: str, ref: str, label: str) -> list[str]:
    before = cursor(ctx.tier)
    clicked = ctx.product(["--session", session, "click", ref])
    require(clicked.ok, f"{label}: the click failed: {clicked.raw[:400]}")
    return pointer_problems(label, before, cursor(ctx.tier))


def on_monitor(monitor: dict, point: tuple[float, float]) -> bool:
    width, height = monitor["width"] / monitor["scale"], monitor["height"] / monitor["scale"]
    return monitor["x"] <= point[0] < monitor["x"] + width and monitor["y"] <= point[1] < monitor["y"] + height


def leftover_boxes(first: pixels.Image, second: pixels.Image, monitor: dict,
                   points: list[tuple[float, float]]) -> list:
    """What differs between two images of one output wherever the leg drew a cursor on it."""
    return [drawn_box(first, second, search_region(monitor, point, (first.width, first.height))) for point in points]


def placed_on(ctx: LegContext, app, session: str, owner: dict, name: str):
    """Clicks 7 on `name`, reads where the cursor is drawn, then clicks 3 to read the bare output."""
    tier, runtime, main = ctx.tier, shared(ctx.tier), main_instance(ctx.tier)
    seven, point = live_button(ctx, app, session, "7")
    monitor = named(monitors_of(tier), name)
    require(monitor_at(monitors_of(tier), point)["name"] == name, f"button 7 at {point} is not on {name}")
    problems = _click(ctx, session, seven, f"7 on {name}")
    raw.poll(lambda: any(tip_at(item, point) and item["pid"] == owner["pid"]
                         for item in surfaces_of(runtime, main)), SETTLE_S)
    layers = surfaces_of(runtime, main)
    problems += layer_problems(f"7 on {name}", layers, owner["pid"], name, point)
    drawn = grim(tier, name)
    three, away = live_button(ctx, app, session, "3")
    problems += _click(ctx, session, three, f"3 on {name}")
    raw.poll(lambda: not any(tip_at(item, point) for item in surfaces_of(runtime, main)), SETTLE_S)
    bare = grim(tier, name)
    box = drawn_box(drawn, bare, search_region(monitor, point, (drawn.width, drawn.height)))
    problems += tip_problems(f"7 on {name}", box, monitor, point)
    return problems, {"layers": layers, "arrow": box, "scale": float(monitor["scale"]), "points": [point, away]}


def _hidden_target(ctx: LegContext, app, session: str, owner: dict, earlier: tuple[float, float]):
    """A click on Calculator while its workspace is hidden draws no cursor on the output that is shown."""
    tier, runtime, main = ctx.tier, shared(ctx.tier), main_instance(ctx.tier)
    workspace = (_frame(runtime, main, app.pid).get("workspace") or {}).get("name", "")
    require(not any(special == workspace for _, special in shown(tier).values()),
            f"{workspace} is still shown: {shown(tier)}")
    seven, point = live_button(ctx, app, session, "7")
    monitor = monitor_at(monitors_of(tier), point)
    problems = _click(ctx, session, seven, "7 while hidden")
    layers = surfaces_of(runtime, main)
    drawn = grim(tier, monitor["name"])
    disabled = ctx.product(["--session", session, "cursor-overlay", "disable"])
    problems += outcome_problems("disable", disabled, "stopped")
    require(raw.poll(lambda: not alive(owner), SETTLE_S), "the renderer survived disable")
    bare = grim(tier, monitor["name"])
    problems += hidden_problems("7 while hidden", drawn, bare, monitor, [point, earlier])
    return problems, {"layers": layers, "arrow": leftover_boxes(drawn, bare, monitor, [point, earlier]),
                      "points": [point]}


def _record(ctx: LegContext, placed: dict, bare: tuple, final: tuple, problems: list[str]) -> None:
    """The cursor's layers and arrows per output, read before the session, while it drew and after it ended.

    `bare` is the home monitor with two images of it taken before the
    session, `final` an image of it and the cursor layers after the session
    ended. The pixel reads cover every point the leg drew a cursor at on it.
    """
    (monitor, first, second), (restored, layers_after) = bare, final
    points = [point for item in placed.values() for point in item["points"]
              if on_monitor(monitor, point)]
    ctx.before("hyprland.layers", [])
    ctx.after("hyprland.layers", {name: item["layers"] for name, item in placed.items()})
    ctx.restored("hyprland.layers", layers_after)
    still, left = leftover_boxes(first, second, monitor, points), leftover_boxes(first, restored, monitor, points)
    ctx.before("pixels.arrow", still)
    ctx.after("pixels.arrow", {name: item["arrow"] for name, item in placed.items()})
    ctx.restored("pixels.arrow", left)
    if any(still) or any(left):
        problems.append(f"the home output changed where the cursor was drawn: before the session {still}, "
                        f"after it ended {left}")


@implementation("cursor.layout_placement_and_hidden_targets")
def layout_placement_and_hidden_targets(ctx: LegContext) -> None:
    tier, runtime, main = ctx.tier, shared(ctx.tier), main_instance(ctx.tier)
    holder, pointer = bystander(tier), cursor(tier)
    home = focused_monitor(tier)
    ctx.before("hyprland.outputs", outputs(tier))
    ctx.before("hyprland.cursor", pointer)
    require(not surfaces_of(runtime, main), f"a cursor surface is mapped before the session: "
                                            f"{surfaces_of(runtime, main)}")
    bare = (named(monitors_of(tier), home), grim(tier, home), grim(tier, home))
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started(ctx)
        placed = {}
        try:
            owner = renderer_of(runtime, main, session)
            _add_output(tier)
            restore_desktop(tier, holder, pointer)
            ctx.after("hyprland.outputs", outputs(tier))
            with app_shown(tier, app.pid, EXTRA_OUTPUT):
                problems, placed[EXTRA_OUTPUT] = placed_on(ctx, app, session, owner, EXTRA_OUTPUT)
            _remove_output(tier)
            restore_desktop(tier, holder, pointer)
            if not alive(owner):
                problems.append(f"renderer {owner['pid']} did not survive the removal of {EXTRA_OUTPUT}")
            with app_shown(tier, app.pid, home):
                more, placed[home] = placed_on(ctx, app, session, owner, home)
            problems += more + scale_problems({name: (item["arrow"], item["scale"]) for name, item in placed.items()})
            more, placed["hidden"] = _hidden_target(ctx, app, session, owner, placed[home]["points"][1])
            problems += more
            ctx.after("hyprland.cursor", cursor(tier))
            ctx.after("atspi.display", display(tree))
        finally:
            ended(ctx, session)
            _remove_output(tier)
            restore_desktop(tier, holder, pointer)
        ctx.restored("atspi.display", undo_all(tree, before, DIGITS_ENTERED))
    _record(ctx, placed, bare, (grim(tier, home), surfaces_of(runtime, main)), problems)
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.outputs", outputs(tier))
    ctx.restored("hyprland.cursor", cursor(tier))
