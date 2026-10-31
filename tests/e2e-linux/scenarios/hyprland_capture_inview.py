"""Capture of a window Hyprland shows, on the attached `hyprland` tier.

Hyprland composes a toplevel on a workspace a monitor shows and asks it for
every frame, so `screenshot --window-id` answers with the first frame. A slow
painter in view draws a tick about every 200 ms; each capture must show the
newest tick it committed before the request, drawn within one draw of it. A
painted terminal in view does not change; it must be captured faster than the
redraw wait can answer an unchanged window, and within a 250 ms batch budget.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture import (SHARE, _batch, _capture, _oracle_discriminates, _placement,
                                        _settled_capture, painted_problems, scale_of)
from scenarios.hyprland_capture_common import (COMMIT_MARGIN_NS, capture_path, colour, dominant, first_drawn,
                                               ids_by_pid, monotonic_ns, near, newest_drawn, painted, saved_image,
                                               settled_tick, slow_painted, tick_of, written)
from scenarios.hyprland_capture_refusals import batch_entry_problems
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_windows import client, cursor, move, restore_desktop

ROUNDS = 3
TICKS_BETWEEN = 3
SLOW_DELAY_MS = 200
STILL_PAINT = "5070a0"
IN_VIEW_BUDGET_MS = 250


@implementation("capture.hyprland_in_view_first_frame")
def in_view_first_frame(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    shown = read()["workspace"]
    ctx.before("hyprland.desktop", read())
    try:
        with (slow_painted(tier, "ad-e2e-cap-view-slow", SLOW_DELAY_MS) as (_, slow, log),
              painted(tier, "ad-e2e-cap-view-still", "static", STILL_PAINT) as (_, still, _)):
            for found in (slow, still):
                move(tier, found["address"], shown)
            restore_desktop(tier, holder, point)
            placed = lambda: _placement(tier, {"slow": slow, "still": still})
            ctx.before("hyprland.placement", placed())
            ids = ids_by_pid(ctx.product(["list-windows"]), [slow["pid"], still["pid"]])
            with Sampler({"desktop": read, "placement": placed}) as sampler:
                problems, shots = [], []
                for index in range(ROUNDS):
                    found_problems, shot = _view_round(ctx, ids[slow["pid"]], log, f"in view slow round {index}")
                    problems += found_problems
                    shots.append(shot)
                problems += _still_in_view(ctx, ids[still["pid"]], client(tier, still["address"]))
            ctx.after("hyprland.placement", placed())
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            problems += _oracle_discriminates(shots, written(log), newest_drawn)
            ctx.restored("hyprland.placement", placed())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _view_round(ctx, window_id: str, log, label: str) -> tuple[list[str], tuple[int | None, int]]:
    """The slow painter, drawing in view, is captured showing its newest committed tick."""
    seen = len(written(log))
    require(raw.poll(lambda: len(written(log)) >= seen + TICKS_BETWEEN, 5.0),
            "the slow painter stopped drawing in view")
    mark = monotonic_ns()
    envelope, image = _capture(ctx, window_id, label)
    if image is None:
        return [f"{label}: capture failed: {envelope.raw[:300]}"], (None, mark)
    found, share = dominant(image)
    tick = tick_of(found)
    ticks = written(log)
    if share < SHARE or not newest_drawn(tick, ticks, mark):
        at = first_drawn(tick, ticks)
        age = "never drawn" if at is None else f"first drawn {(mark - at) / 1e6:.0f} ms before the request"
        return [f"{label}: the capture shows tick {tick} ({share:.0%} {found}), {age}; tick "
                f"{settled_tick(ticks, mark, COMMIT_MARGIN_NS)} was committed before the request"], (tick, mark)
    return [], (tick, mark)


def _still_in_view(ctx, window_id: str, found: dict) -> list[str]:
    """An unchanged window in view is answered sooner than the redraw wait could answer it."""
    size, scale = tuple(found["size"]), scale_of(ctx.tier, found)
    envelope, image = _settled_capture(ctx, window_id, "in-view-still", size, scale)
    if image is None:
        return [f"in view still: capture failed: {envelope.raw[:300]}"]
    envelope, image = _capture(ctx, window_id, "in-view-still-timed")
    if image is None:
        return [f"in view still: capture failed: {envelope.raw[:300]}"]
    problems = painted_problems("in view still", image, STILL_PAINT, size, scale)
    if envelope.elapsed_ms > IN_VIEW_BUDGET_MS:
        problems.append(f"in view still: an unchanged window in view took {envelope.elapsed_ms:.0f} ms, "
                        f"more than {IN_VIEW_BUDGET_MS} ms, so it waited for a redraw")
    path = capture_path(ctx.tier, "in-view-still-batch")
    batched = _batch(ctx, window_id, path, IN_VIEW_BUDGET_MS)
    found_problems, outcome = batch_entry_problems(IN_VIEW_BUDGET_MS, batched)
    problems += found_problems
    if outcome != "captured":
        path.unlink(missing_ok=True)
        return problems + [f"in view still: a {IN_VIEW_BUDGET_MS} ms batch answered {outcome}, not a capture"]
    colour_found, share = dominant(saved_image(batched, path))
    if not near(colour_found, colour(STILL_PAINT)) or share < SHARE:
        problems.append(f"in view still: the batch capture is {share:.0%} {colour_found}, "
                        f"the window is painted #{STILL_PAINT}")
    return problems
