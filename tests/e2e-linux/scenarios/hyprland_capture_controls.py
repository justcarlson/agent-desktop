"""Capture controls on the attached `hyprland` tier: a withheld frame, and terminals other than foot.

A slow painter that takes 3000 ms to draw holds back the frame a capture
asks for, so a 1000 ms batch budget ends while the frame is awaited: the
answer is TIMEOUT capture_frame, nothing is written, and the painter's own
log shows that the draw the request started ended only after the answer.
A longer budget then captures it, so the timeout was the draw, not a broken
window. Ghostty and Alacritty run the same painter as foot does, each with
its own configuration files ignored, and are captured the same way.
"""

from __future__ import annotations

import contextlib
import json
import shutil
import subprocess

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture import _placement, _settled_capture, painted_problems, scale_of
from scenarios.hyprland_capture_common import (capture_path, dominant, drawn_after, first_drawn, ids_by_pid,
                                               monotonic_ns, painter_command, saved_image, slow_painted, tick_of,
                                               written)
from scenarios.hyprland_capture_refusals import batch_entry_problems
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus
from scenarios.hyprland_toplevel import toplevel
from scenarios.hyprland_windows import SPARE_WORKSPACE, client, cursor, move, restore_desktop

WITHHELD_DRAW_MS = 3000
WITHHELD_BUDGET_MS = 1000
HEALTHY_BUDGET_MS = 10_000
IDLE_NS = (WITHHELD_DRAW_MS + 1000) * 1_000_000
IDLE_WAIT_S = 15.0
LATE_DRAW_WAIT_S = (WITHHELD_DRAW_MS + 2000) / 1000
SHARE = 0.9
PAINTS = ("c03020", "20c030", "3020c0")
PLACES = ("special", "spare", "shown")
VERSION_TIMEOUT_S = 5.0


def ghostty_argv(app_id: str, command: list[str]) -> list[str]:
    """A Ghostty toplevel of class and title `app_id`, with no user configuration and no decorations."""
    return ["ghostty", "--config-default-files=false", f"--class={app_id}", f"--title={app_id}",
            "--gtk-single-instance=false", "--window-decoration=none", "-e", *command]


def alacritty_argv(app_id: str, command: list[str]) -> list[str]:
    """An Alacritty toplevel of class and title `app_id`, with an empty configuration."""
    return ["alacritty", "--config-file", "/dev/null", "--class", app_id, "--title", app_id, "-e", *command]


TERMINALS = {"ghostty": ghostty_argv, "alacritty": alacritty_argv}


def terminal_paints(kind: str) -> dict[str, str]:
    """One dotted application id per placement, each with its paint; Ghostty takes only a valid GTK id."""
    return {f"ad.e2e.{kind}-{place}": paint for place, paint in zip(PLACES, PAINTS)}


def withheld_problems(envelope, wrote: bool, ticks: list[tuple[int, int]], mark_ns: int,
                      answered_ns: int) -> list[str]:
    """A batch that timed out while the frame it asked for was still being drawn.

    The painter logs a tick when its draw ends, so the first tick logged after
    the request must come after the answer: the compositor did ask for a frame,
    and the client had not delivered it when the budget ended.
    """
    label = f"withheld {WITHHELD_BUDGET_MS} ms"
    problems, outcome = batch_entry_problems(WITHHELD_BUDGET_MS, envelope)
    if outcome != "capture_frame":
        problems.append(f"{label}: answered {outcome}, not a capture_frame timeout")
    if wrote:
        problems.append(f"{label}: a timed-out capture wrote an image")
    late = [at for _, at in ticks if at > mark_ns]
    if not late:
        problems.append(f"{label}: the painter drew nothing after the request, so no frame was asked for")
    elif min(late) < answered_ns:
        problems.append(f"{label}: the painter finished a draw {(answered_ns - min(late)) / 1e6:.0f} ms "
                        "before the answer, so the frame was not withheld")
    return problems


def _idle(log) -> bool:
    """Whether the painter has drawn nothing for a full draw and a second.

    A draw blocks the painter's main loop, so its accessibility tree cannot
    answer then, and a window listed during a draw gets a compositor-only id
    that a later listing replaces. Every listing and request waits for this.
    """
    ticks = written(log)
    return bool(ticks) and monotonic_ns() - ticks[-1][1] >= IDLE_NS


def _batch(ctx, window_id: str, path, budget_ms: int):
    entries = json.dumps([{"command": "screenshot", "args": {"window_id": window_id, "output_path": str(path)}}])
    return ctx.product(["batch", entries, "--timeout-ms", str(budget_ms)])


def _withheld(ctx, window_id: str, log) -> list[str]:
    require(raw.poll(lambda: _idle(log), IDLE_WAIT_S), "the slow painter never went idle")
    path = capture_path(ctx.tier, "withheld")
    mark = monotonic_ns()
    envelope = _batch(ctx, window_id, path, WITHHELD_BUDGET_MS)
    answered = monotonic_ns()
    wrote = path.exists()
    path.unlink(missing_ok=True)
    raw.poll(lambda: any(at > mark for _, at in written(log)), LATE_DRAW_WAIT_S)
    ticks = written(log)
    late = [at for _, at in ticks if at > mark]
    if late:
        ctx.finding(f"the withheld frame was drawn {(min(late) - mark) / 1e6:.0f} ms after the request, "
                    f"{(min(late) - answered) / 1e6:.0f} ms after the {envelope.elapsed_ms:.0f} ms answer")
    return withheld_problems(envelope, wrote, ticks, mark, answered)


def _healthy(ctx, window_id: str, log) -> list[str]:
    """A budget that holds the draw captures the same window, showing a tick drawn after this request."""
    require(raw.poll(lambda: _idle(log), IDLE_WAIT_S), "the slow painter never went idle after the timeout")
    path = capture_path(ctx.tier, "withheld-healthy")
    mark = monotonic_ns()
    envelope = _batch(ctx, window_id, path, HEALTHY_BUDGET_MS)
    problems, outcome = batch_entry_problems(HEALTHY_BUDGET_MS, envelope)
    if outcome != "captured":
        path.unlink(missing_ok=True)
        return problems + [f"a {HEALTHY_BUDGET_MS} ms batch did not capture the slow window: {envelope.raw[:400]}"]
    found, share = dominant(saved_image(envelope, path))
    tick, ticks = tick_of(found), written(log)
    if share < SHARE or not drawn_after(tick, ticks, mark):
        at = first_drawn(tick, ticks)
        age = "never drawn" if at is None else f"first drawn {(mark - at) / 1e6:.0f} ms before the request"
        problems.append(f"the healthy capture shows tick {tick} ({share:.0%} {found}), {age}")
    ctx.finding(f"a {HEALTHY_BUDGET_MS} ms batch captured tick {tick} in {envelope.elapsed_ms:.0f} ms")
    return problems


@implementation("capture.hyprland_withheld_frame_timeout")
def withheld_frame_timeout(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with slow_painted(tier, "ad-e2e-cap-withheld", WITHHELD_DRAW_MS) as (owned, found, log):
            require(raw.poll(lambda: _idle(log), IDLE_WAIT_S), "the slow painter never went idle after it mapped")
            window_id = ids_by_pid(ctx.product(["list-windows"]), [found["pid"]])[found["pid"]]
            running = lambda: client(tier, found["address"]).get("pid") == owned.pid
            ctx.before("applog.painter_running", running())
            with Sampler({"desktop": read}) as sampler:
                problems = _withheld(ctx, window_id, log)
                problems += _healthy(ctx, window_id, log)
            ctx.after("applog.painter_running", running())
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            ctx.restored("applog.painter_running", running())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _version(kind: str) -> str | None:
    """The first line `kind` prints for its version, or None when it is not installed."""
    if not shutil.which(kind):
        return None
    argv = [kind, "+version"] if kind == "ghostty" else [kind, "--version"]
    try:
        done = subprocess.run(argv, capture_output=True, text=True, timeout=VERSION_TIMEOUT_S, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"unreadable: {error}"
    lines = (done.stdout or done.stderr).strip().splitlines()
    return lines[0] if lines else f"exit {done.returncode}"


@contextlib.contextmanager
def painted_in(tier, kind: str, app_id: str, paint: str):
    """A `kind` terminal on the launch special workspace running the static painter, and its log."""
    log = tier.runtime_dir / f"painter-{app_id}.log"
    log.unlink(missing_ok=True)
    with toplevel(tier, app_id, TERMINALS[kind](app_id, painter_command(log, "static", paint))) as (owned, found):
        require(raw.poll(lambda: written(log), 10.0), f"{app_id} never painted")
        yield owned, found, log


def _terminal_pixels(ctx: LegContext, kind: str) -> None:
    version = _version(kind)
    require(version, f"{kind} is not installed in this image")
    ctx.finding(f"{kind}: {version}")
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    shown = read()["workspace"]
    paints = terminal_paints(kind)
    names = dict(zip(PLACES, paints))
    ctx.before("hyprland.desktop", read())
    try:
        with contextlib.ExitStack() as stack:
            windows = {name: stack.enter_context(painted_in(tier, kind, name, paint))[1]
                       for name, paint in paints.items()}
            move(tier, windows[names["spare"]]["address"], SPARE_WORKSPACE)
            move(tier, windows[names["shown"]]["address"], shown)
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
                        problems.append(f"{name}: capture failed: {envelope.raw[:300]}")
                        continue
                    problems += painted_problems(name, image, paint, tuple(found["size"]), scale_of(tier, found))
            ctx.after("hyprland.placement", placed())
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            ctx.restored("hyprland.placement", placed())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


@implementation("capture.hyprland_ghostty_toplevel_pixels")
def ghostty_toplevel_pixels(ctx: LegContext) -> None:
    _terminal_pixels(ctx, "ghostty")


@implementation("capture.hyprland_alacritty_toplevel_pixels")
def alacritty_toplevel_pixels(ctx: LegContext) -> None:
    _terminal_pixels(ctx, "alacritty")
