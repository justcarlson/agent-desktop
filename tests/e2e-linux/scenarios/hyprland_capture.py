"""Window capture on the attached `hyprland` tier.

`screenshot --window-id` captures exactly the bound toplevel through the
compositor's own capture protocol, wherever it is: on a hidden special
workspace, on a regular workspace out of view, or in view. Each target is a
painted terminal, so the pixels say whose they are; the ticker says whether
they were drawn after the request, and the slow painter says whether a
window that takes 200 ms to draw was waited for. The desktop is polled while the product
works, and nothing is activated, moved or shown.
"""

from __future__ import annotations

import contextlib
import json

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_common import (capture_path, colour, dominant, drawn_after, first_drawn, fresh,
                                               ids_by_pid, monotonic_ns, near, painted, saved_image, settled_tick,
                                               slow_painted, tick_of, written)
from scenarios.hyprland_capture_refusals import batch_entry_problems
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_windows import (SPARE_WORKSPACE, client, cursor, foreign_id, listed, move, query,
                                        restore_desktop, terminal)

PAINTS = {"ad-e2e-cap-special": "c03020", "ad-e2e-cap-spare": "20c030", "ad-e2e-cap-shown": "3020c0"}
TICK_PERIOD = "0.05"
TICKS_BETWEEN = 12
ROUNDS = 3
SHARE = 0.9
STATIC_BUDGET_MS = 2000.0
SHORT_BATCH_MS = 250
LONG_BATCH_MS = 1000
RESIZE_SETTLE_S = 3.0
SLOW_DELAY_MS = 200
SLOW_IDLE_NS = 500_000_000


def scale_of(tier, found: dict) -> float:
    scales = {item["id"]: item["scale"] for item in query(tier, "monitors")}
    return scales[found["monitor"]]


def painted_problems(label: str, image, paint: str, size: tuple[int, int], scale: float) -> list[str]:
    """The capture is the painted window: its colour across the surface, at its size."""
    found, share = dominant(image)
    problems = []
    if not near(found, colour(paint)) or share < SHARE:
        problems.append(f"{label}: the capture is {share:.0%} {found}, the window is painted #{paint}")
    width, height = size
    if not sized(image, size, scale):
        problems.append(f"{label}: the capture is {image.width}x{image.height}, "
                        f"the window is {width}x{height} at scale {scale}")
    return problems


def _capture(ctx, window_id: str, name: str):
    path = capture_path(ctx.tier, name)
    envelope = ctx.product(["screenshot", "--window-id", window_id, str(path)])
    return envelope, (saved_image(envelope, path) if envelope.ok else None)


def sized(image, size: tuple[int, int], scale: float) -> bool:
    width, height = size
    return abs(image.width - width * scale) <= 1 and abs(image.height - height * scale) <= 1


def _settled_capture(ctx, window_id: str, name: str, size: tuple[int, int], scale: float):
    """A capture once the window's buffer has its laid-out size.

    Hyprland reports the size a move lays a window out at while its resize
    animation still grows the buffer, so a capture taken during the
    animation is a true, smaller frame. A wrong scale never settles.
    """
    shots = []

    def take() -> bool:
        shots.append(_capture(ctx, window_id, f"{name}-{len(shots)}"))
        envelope, image = shots[-1]
        return not envelope.ok or sized(image, size, scale)

    raw.poll(take, RESIZE_SETTLE_S)
    if len(shots) > 1:
        ctx.finding(f"{name}: {len(shots)} captures until the resize animation settled")
    return shots[-1]


def _placement(tier, windows: dict) -> dict:
    return {name: client(tier, found["address"])["workspace"]["name"] for name, found in windows.items()}


@implementation("capture.hyprland_bound_toplevel_pixels")
def bound_toplevel_pixels(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    shown = read()["workspace"]
    ctx.before("hyprland.desktop", read())
    try:
        with contextlib.ExitStack() as stack:
            windows = {name: stack.enter_context(painted(tier, name, "static", paint))[1]
                       for name, paint in PAINTS.items()}
            move(tier, windows["ad-e2e-cap-spare"]["address"], SPARE_WORKSPACE)
            move(tier, windows["ad-e2e-cap-shown"]["address"], shown)
            restore_desktop(tier, holder, point)
            placed = lambda: _placement(tier, windows)
            ctx.before("hyprland.placement", placed())
            ids = ids_by_pid(ctx.product(["list-windows"]), [found["pid"] for found in windows.values()])
            with Sampler({"desktop": read, "placement": placed}) as sampler:
                problems = []
                for name, paint in PAINTS.items():
                    found = client(tier, windows[name]["address"])
                    envelope, image = _settled_capture(ctx, ids[found["pid"]], name, tuple(found["size"]),
                                                       scale_of(tier, found))
                    if image is None:
                        problems.append(f"{name}: capture failed: {envelope.raw[:300]}")
                        continue
                    problems += painted_problems(name, image, paint, tuple(found["size"]), scale_of(tier, found))
                problems += _foreign_and_closed(ctx, ids[windows["ad-e2e-cap-special"]["pid"]])
            ctx.after("hyprland.placement", placed())
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            ctx.restored("hyprland.placement", placed())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _foreign_and_closed(ctx, window_id: str) -> list[str]:
    """Another compositor's handle and a closed toplevel are refused, and nothing is written."""
    path = capture_path(ctx.tier, "refused")
    problems = boundary_problems("other compositor instance",
                                 ctx.product(["screenshot", "--window-id", foreign_id(window_id), str(path)]),
                                 ("WINDOW_NOT_FOUND",))
    with terminal(ctx.tier, "ad-e2e-cap-closed") as (_, gone):
        old = [item["id"] for item in listed(ctx.product(["list-windows"]), pid=gone["pid"])]
    require(len(old) == 1, f"the closed terminal was listed {len(old)} times")
    problems += boundary_problems("closed toplevel",
                                  ctx.product(["screenshot", "--window-id", old[0], str(path)]),
                                  ("WINDOW_NOT_FOUND",))
    if path.exists():
        problems.append("a refused capture wrote an image")
    return problems


def _ticker_round(ctx, window_id: str, log, label: str) -> tuple[list[str], tuple[int | None, int]]:
    seen = len(written(log))
    require(raw.poll(lambda: len(written(log)) >= seen + TICKS_BETWEEN, 5.0), "the ticker stopped painting")
    mark = monotonic_ns()
    envelope, image = _capture(ctx, window_id, label)
    if image is None:
        return [f"{label}: capture failed: {envelope.raw[:300]}"], (None, mark)
    found, share = dominant(image)
    tick = tick_of(found)
    ticks = written(log)
    problems = []
    if share < SHARE or not fresh(tick, ticks, mark):
        problems.append(f"{label}: the capture shows tick {tick} ({share:.0%} {found}); "
                        f"tick {settled_tick(ticks, mark)} was drawn before the request")
    return problems, (tick, mark)


def _slow_round(ctx, window_id: str, log, label: str) -> tuple[list[str], tuple[int | None, int]]:
    """The slow painter, idle out of view, is captured showing a tick it first drew after the request."""
    idle = lambda: monotonic_ns() - written(log)[-1][1] >= SLOW_IDLE_NS
    require(raw.poll(idle, 5.0), "the slow painter kept drawing out of view")
    mark = monotonic_ns()
    envelope, image = _capture(ctx, window_id, label)
    if image is None:
        return [f"{label}: capture failed: {envelope.raw[:300]}"], (None, mark)
    found, share = dominant(image)
    tick = tick_of(found)
    ticks = written(log)
    if share < SHARE or not drawn_after(tick, ticks, mark):
        at = first_drawn(tick, ticks)
        age = "never drawn" if at is None else f"first drawn {(mark - at) / 1e6:.0f} ms before the request"
        return [f"{label}: the slow painter's capture shows tick {tick} ({share:.0%} {found}), {age}"], (tick, mark)
    return [], (tick, mark)


@implementation("capture.hyprland_hidden_counter_frame_after_request")
def hidden_counter_frame_after_request(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with (painted(tier, "ad-e2e-cap-ticker", "ticker", TICK_PERIOD) as (_, found, log),
              slow_painted(tier, "ad-e2e-cap-slow", SLOW_DELAY_MS) as (_, slow, slow_log)):
            ids = ids_by_pid(ctx.product(["list-windows"]), [found["pid"], slow["pid"]])
            placed = lambda: _placement(tier, {"ticker": found, "slow": slow})
            problems, shots, slow_shots = [], [], []
            for workspace in (None, SPARE_WORKSPACE):
                if workspace:
                    move(tier, found["address"], workspace)
                    move(tier, slow["address"], workspace)
                    restore_desktop(tier, holder, point)
                key = f"hyprland.placement_{workspace or 'special'}"
                ctx.before(key, placed())
                with Sampler({"desktop": read, "placement": placed}) as sampler:
                    for index in range(ROUNDS):
                        where = placed()["ticker"]
                        found_problems, shot = _ticker_round(ctx, ids[found["pid"]], log, f"{where} round {index}")
                        slow_problems, slow_shot = _slow_round(ctx, ids[slow["pid"]], slow_log,
                                                               f"{where} slow round {index}")
                        problems += found_problems + slow_problems
                        shots.append(shot)
                        slow_shots.append(slow_shot)
                ctx.after(key, placed())
                problems += sampler.changes
                ctx.restored(key, placed())
            ctx.after("hyprland.desktop", read())
            problems += keeps_focus(tier, read())
            problems += _oracle_discriminates(shots, written(log), fresh)
            problems += _oracle_discriminates(slow_shots, written(slow_log), drawn_after)
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _oracle_discriminates(shots: list[tuple[int | None, int]], ticks: list[tuple[int, int]], accepts) -> list[str]:
    """The first capture, offered as the answer to the last request, must fail the freshness oracle."""
    (first, _), (_, last_mark) = shots[0], shots[-1]
    if first is not None and accepts(first, ticks, last_mark):
        return [f"the {accepts.__name__} oracle accepts tick {first} for a request made after tick "
                f"{settled_tick(ticks, last_mark, 0)}, so it cannot see a cached frame"]
    return []


@implementation("capture.hyprland_static_frame_after_request")
def static_frame_after_request(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    paints = {"ad-e2e-cap-still": "a05000", "ad-e2e-cap-still-spare": "00a050"}
    ctx.before("hyprland.desktop", read())
    try:
        with contextlib.ExitStack() as stack:
            windows = {name: stack.enter_context(painted(tier, name, "static", paint))[1]
                       for name, paint in paints.items()}
            move(tier, windows["ad-e2e-cap-still-spare"]["address"], SPARE_WORKSPACE)
            restore_desktop(tier, holder, point)
            placed = lambda: _placement(tier, windows)
            ctx.before("hyprland.placement", placed())
            ids = ids_by_pid(ctx.product(["list-windows"]), [found["pid"] for found in windows.values()])
            with Sampler({"desktop": read, "placement": placed}) as sampler:
                problems = []
                for name, paint in paints.items():
                    found = client(tier, windows[name]["address"])
                    envelope, image = _settled_capture(ctx, ids[found["pid"]], name, tuple(found["size"]),
                                                       scale_of(tier, found))
                    if image is None:
                        problems.append(f"{name}: an unchanged window was not captured: {envelope.raw[:300]}")
                        continue
                    problems += painted_problems(name, image, paint, tuple(found["size"]), scale_of(tier, found))
                    if envelope.elapsed_ms > STATIC_BUDGET_MS:
                        problems.append(f"{name}: an unchanged window took {envelope.elapsed_ms:.0f} ms")
                    problems += _short_batch(ctx, ids[found["pid"]], name, paint)
            ctx.after("hyprland.placement", placed())
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            ctx.restored("hyprland.placement", placed())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _batch(ctx, window_id: str, path, budget_ms: int):
    entries = json.dumps([{"command": "screenshot", "args": {"window_id": window_id, "output_path": str(path)}}])
    return ctx.product(["batch", entries, "--timeout-ms", str(budget_ms)])


def _short_batch(ctx, window_id: str, name: str, paint: str) -> list[str]:
    """A batch budget shorter than the redraw wait times out unwritten; one that holds the wait captures."""
    path = capture_path(ctx.tier, f"{name}-batch")
    problems, outcome = batch_entry_problems(SHORT_BATCH_MS, _batch(ctx, window_id, path, SHORT_BATCH_MS))
    if outcome != "capture_frame":
        problems.append(f"{name}: a {SHORT_BATCH_MS} ms batch answered {outcome}, not a capture_frame timeout")
    if path.exists():
        problems.append(f"{name}: a {SHORT_BATCH_MS} ms batch wrote an image")
    path.unlink(missing_ok=True)
    envelope = _batch(ctx, window_id, path, LONG_BATCH_MS)
    found_problems, outcome = batch_entry_problems(LONG_BATCH_MS, envelope)
    problems += found_problems
    if outcome != "captured":
        path.unlink(missing_ok=True)
        return problems + [f"{name}: a {LONG_BATCH_MS} ms batch did not capture an unchanged window: "
                           f"{envelope.raw[:400]}"]
    found, share = dominant(saved_image(envelope, path))
    if not near(found, colour(paint)) or share < SHARE:
        problems.append(f"{name}: the batch capture is {share:.0%} {found}, the window is painted #{paint}")
    return problems
