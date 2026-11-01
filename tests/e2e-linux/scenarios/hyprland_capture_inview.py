"""Capture of a window Hyprland shows, on the attached `hyprland` tier.

Hyprland composes a toplevel on a workspace a monitor shows and asks it for
every frame, so `screenshot --window-id` answers with the first frame. A slow
painter in view draws a tick about every 200 ms; each capture must show the
newest tick it committed before the request, drawn within one draw of it. A
painted terminal in view does not change; `screenshot` and `batch` must each
capture it with exactly one image-copy capture session, counted in the
product's Wayland protocol log. The redraw wait opens a second session for an
unchanged window, so a count of one shows it was skipped. Elapsed times are
kept as a record only.
"""

from __future__ import annotations

import json
import re

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture import (SHARE, _capture, _oracle_discriminates, _placement,
                                        _settled_capture, painted_problems, scale_of)
from scenarios.hyprland_capture_common import (COMMIT_MARGIN_NS, capture_path, colour, dominant, first_drawn,
                                               ids_by_pid, monotonic_ns, near, newest_drawn, painted, saved_image,
                                               settled_tick, slow_painted, tick_of, written)
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_windows import client, cursor, move, restore_desktop

ROUNDS = 3
TICKS_BETWEEN = 3
SLOW_DELAY_MS = 200
STILL_PAINT = "5070a0"
SESSION_REQUEST = re.compile(r" -> ext_image_copy_capture_manager_v1@\d+\.create_session\(")


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


def capture_sessions(envelope) -> int:
    """The image-copy capture sessions one product run asked for, from its WAYLAND_DEBUG log."""
    return len(SESSION_REQUEST.findall(envelope.stderr))


def _one_session(label: str, envelope) -> list[str]:
    sessions = capture_sessions(envelope)
    if sessions == 1:
        return []
    return [f"{label}: an unchanged window in view used {sessions} capture sessions, not 1, "
            f"so it {'waited for a redraw' if sessions > 1 else 'left no protocol log'}"]


def _still_in_view(ctx, window_id: str, found: dict) -> list[str]:
    """An unchanged window in view is answered from its first capture session, with no redraw wait."""
    size, scale = tuple(found["size"]), scale_of(ctx.tier, found)
    envelope, image = _settled_capture(ctx, window_id, "in-view-still", size, scale)
    if image is None:
        return [f"in view still: capture failed: {envelope.raw[:300]}"]
    logged = dict(ctx.tier.env, WAYLAND_DEBUG="1")
    path = capture_path(ctx.tier, "in-view-still-logged")
    envelope = ctx.product(["screenshot", "--window-id", window_id, str(path)], env=logged)
    image = saved_image(envelope, path) if envelope.ok else None
    if image is None:
        return [f"in view still: capture failed: {envelope.raw[:300]}"]
    problems = painted_problems("in view still", image, STILL_PAINT, size, scale)
    problems += _one_session("in view still", envelope)
    path = capture_path(ctx.tier, "in-view-still-batch")
    entries = json.dumps([{"command": "screenshot", "args": {"window_id": window_id, "output_path": str(path)}}])
    batched = ctx.product(["batch", entries], env=logged)
    ctx.finding(f"in view still, record only: screenshot {envelope.elapsed_ms:.0f} ms, "
                f"batch {batched.elapsed_ms:.0f} ms")
    results = (batched.data or {}).get("results") or []
    if not batched.ok or len(results) != 1 or results[0].get("ok") is not True:
        path.unlink(missing_ok=True)
        return problems + [f"in view still: the batch did not capture: {batched.raw[:300]}"]
    problems += _one_session("in view still batch", batched)
    colour_found, share = dominant(saved_image(batched, path))
    if not near(colour_found, colour(STILL_PAINT)) or share < SHARE:
        problems.append(f"in view still: the batch capture is {share:.0%} {colour_found}, "
                        f"the window is painted #{STILL_PAINT}")
    return problems
