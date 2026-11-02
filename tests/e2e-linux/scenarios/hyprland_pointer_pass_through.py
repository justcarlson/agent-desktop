"""A headed coordinate click through a drawn agent cursor on the attached `hyprland` tier.

The cursor surface takes no input: its input region is empty. The product hides every agent cursor
before headed pointer input, coordinate clicks included, so in a normal run no press lands under a
drawn cursor. The renderer's `keep-drawn-for-test` flag keeps the cursor drawn through that hide,
which lets this leg click an XWayland pad's button at the cursor's tip while the cursor is drawn.

The pad's own event log counts the presses. `j/layers` and `grim` pixels at the arrow, read back to
back from before the click until after the press, show the cursor stayed drawn. A live holder
without the flag must clear the arrow first, so the pixel read can tell drawn from cleared. The
command's own `ok` is never the evidence.
"""

from __future__ import annotations

import threading
import time
from typing import NamedTuple

from oracle import atspi_raw as raw, hyprland
from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import NAMESPACE, app_shown, ended, layers_of, renderer_of, started
from scenarios.hyprland_cursor_hide import (SHOW_SETTLE_S, Screen, _grab_until, harness_holder, holders_in,
                                            keep_drawn, registry_of)
from scenarios.hyprland_pointer_pad import (XWAYLAND, apart_from_last_click, covering_layers, events,
                                            instance_env, pad_ref, pads_left, pointer_pad, presses, settled,
                                            since, xy)

QUIET_S = 1.0
READ_INTERVAL_S = 0.01
ORACLES = ("layers", "pixels")


class Sample(NamedTuple):
    oracle: str
    start: float
    end: float
    drawn: bool
    detail: str


def one_press_problems(found: list[dict]) -> list[str]:
    """The pad logged exactly one physical press, on its button, and no press anywhere else."""
    problems = []
    pressed = presses(found, "button")
    if pressed != [1]:
        problems.append(f"the pad button logged {len(pressed)} presses {pressed}, expected one")
    others = sorted({item.get("widget", "") for item in found
                     if item.get("ev") == "press" and item.get("widget") != "button"})
    if others:
        problems.append(f"the click also pressed {', '.join(others)}")
    return problems


def drawn_problems(samples: list[Sample], window: tuple[float, float]) -> list[str]:
    """Every sample shows the cursor, and each oracle has a sample whose read overlaps the click."""
    start, end = window
    problems = [f"{sample.oracle} {sample.start - start:+.3f}s from the click: the cursor was not drawn"
                f"{' (' + sample.detail + ')' if sample.detail else ''}" for sample in samples if not sample.drawn]
    for oracle in ORACLES:
        if not any(sample.oracle == oracle and sample.start <= end and sample.end >= start for sample in samples):
            problems.append(f"no {oracle} read overlaps the click from {start:.3f} to {end:.3f}")
    return problems


class DrawnWatch:
    """Reads each oracle in its own thread, back to back, until the block exits."""

    def __init__(self, reads: dict, interval_s: float = READ_INTERVAL_S):
        self.samples: list[Sample] = []
        self._stop = threading.Event()
        self._lock = threading.Lock()
        self._threads = [threading.Thread(target=self._run, args=(name, read, interval_s), daemon=True)
                         for name, read in reads.items()]

    def _read(self, name: str, read) -> None:
        began = time.monotonic()
        try:
            shown, detail = bool(read()), ""
        except Exception as error:
            shown, detail = False, f"{type(error).__name__}: {error}"[:200]
        with self._lock:
            self.samples.append(Sample(name, began, time.monotonic(), shown, detail))

    def _run(self, name: str, read, interval_s: float) -> None:
        self._read(name, read)
        while not self._stop.wait(interval_s):
            self._read(name, read)

    def __enter__(self):
        for thread in self._threads:
            thread.start()
        return self

    def __exit__(self, *exc):
        self._stop.set()
        for thread in self._threads:
            thread.join(10.0)
        return False


def _button_point(ctx: LegContext, env: dict, button: str) -> tuple[float, float]:
    read = ctx.product(["get", button, "--property", "bounds"], env=env, headed=False)
    bounds = (read.data or {}).get("value") if read.ok and isinstance(read.data, dict) else None
    require(bounds and read.data.get("live") is True, f"the pad button has no live bounds: {read.raw[:400]}")
    return int(bounds["x"] + bounds["width"] / 2), int(bounds["y"] + bounds["height"] / 2)


def _click_through(ctx: LegContext, env: dict, owned, log, session: str) -> list[str]:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    registry = registry_of(runtime, session)
    button = pad_ref(ctx, env, owned)
    point = _button_point(ctx, env, button)
    screen = Screen(ctx, main, point)
    placed = ctx.product(["click", button], env=env, headed=False)
    require(placed.ok, f"the click that places the cursor failed: {placed.raw[:400]}")
    covered = lambda: NAMESPACE in covering_layers(
        hyprland.query(runtime, main["signature"], "layers", pid=main["pid"]), point)
    require(raw.poll(covered, SHOW_SETTLE_S), f"no cursor layer covers {point}: {layers_of(runtime, main)}")
    require(_grab_until(screen, True) is not None, f"the screen shows no cursor at {point}")
    with harness_holder(registry):
        cleared = _grab_until(screen, False)
    require(cleared is not None, "a live holder left the arrow on screen, so pixels cannot tell drawn from cleared")
    require(_grab_until(screen, True) is not None, "the cursor did not come back after the holder left")
    apart_from_last_click(log)
    start = len(events(log))
    pixels_drawn = lambda: screen.shows(screen.grab())
    with keep_drawn(registry), DrawnWatch({"layers": covered, "pixels": pixels_drawn}) as watch:
        began = time.monotonic()
        clicked = ctx.product(["mouse-click", f"--xy={xy(point)}"], env=env, headed=True)
        finished = time.monotonic()
        settled(log, lambda found: bool(presses(since(found, start), "button")), SHOW_SETTLE_S)
        time.sleep(QUIET_S)
    problems = one_press_problems(since(events(log), start))
    if not clicked.ok:
        problems.append(f"the headed mouse-click failed: {clicked.raw[:400]}")
    problems += drawn_problems(watch.samples, (began, finished))
    if _grab_until(screen, True) is None or not covered():
        problems.append("the cursor was not drawn over the button after the click")
    ctx.after("proc.holders", len(holders_in(registry)))
    ctx.restored("proc.holders", len(holders_in(registry)))
    return problems


def coordinate_click_through_cursor(ctx: LegContext) -> None:
    """A headed coordinate click at the drawn cursor's tip lands one press, and the cursor stays drawn."""
    tier, runtime, main = ctx.tier, shared(ctx.tier), main_instance(ctx.tier)
    env = instance_env(tier, main)
    ctx.before("applog.pad", pads_left(tier))
    ctx.before("hyprland.layers", layers_of(runtime, main))
    with pointer_pad(tier, main, "pass-through", XWAYLAND) as (owned, _, log), app_shown(tier, owned.pid):
        session = started(ctx, env)
        try:
            renderer_of(runtime, main, session)
            ctx.before("proc.holders", len(holders_in(registry_of(runtime, session))))
            problems = _click_through(ctx, dict(env, AGENT_DESKTOP_SESSION=session), owned, log, session)
            ctx.after("applog.pad", len(events(log)))
            ctx.after("hyprland.layers", layers_of(runtime, main))
        finally:
            ended(ctx, session, env)
        raw.poll(lambda: not layers_of(runtime, main), SHOW_SETTLE_S)
        require(not problems, "; ".join(problems))
    ctx.restored("applog.pad", pads_left(tier))
    ctx.restored("hyprland.layers", layers_of(runtime, main))
