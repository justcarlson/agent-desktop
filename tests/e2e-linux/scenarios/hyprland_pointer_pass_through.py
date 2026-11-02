"""A headed coordinate click through a drawn agent cursor on the attached `hyprland` tier.

The cursor surface takes no input: its input region is empty. The product hides every agent cursor
before headed pointer input, coordinate clicks included, so in a normal run no press ever lands
under a drawn cursor. The renderer's `keep-drawn-for-test` flag keeps the cursor drawn through that
hide, which lets this leg click Calculator's 7 at the cursor's tip while the cursor is on screen.

Calculator's AT-SPI display counts the presses. `j/layers` and `grim` pixels at the arrow, sampled
without a gap from before the click until after the press shows, prove the cursor stayed drawn.
A live holder without the flag must clear the arrow first, so the pixel read can tell drawn from
cleared. The command's own `ok` is never the evidence.
"""

from __future__ import annotations

import re
import threading
import time
from typing import NamedTuple

from oracle import atspi_raw as raw, hyprland
from runner.helper import display, managed, require, root
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import NAMESPACE, _seven, app_shown, ended, layers_of, renderer_of, started
from scenarios.hyprland_cursor_hide import (SHOW_SETTLE_S, Screen, _grab_until, harness_holder, holders_in,
                                            keep_drawn, registry_of, undo_all)
from scenarios.hyprland_pointer_pad import covering_layers, xy

DIGIT = "7"
QUIET_S = 1.0
LAYER_INTERVAL_S = 0.01
ORACLES = ("layers", "pixels")


class Sample(NamedTuple):
    oracle: str
    start: float
    end: float
    drawn: bool
    detail: str


def presses_entered(before: str, after: str, digit: str) -> int | None:
    """How many `digit` presses turn the display `before` into `after`; None if something else changed it.
    Group separators do not count, and a display of 0 is empty."""
    def digits(text: str) -> str:
        found = re.sub(r"[^0-9]", "", text)
        return "" if found == "0" else found
    start, end = digits(before), digits(after)
    if not end.startswith(start):
        return None
    tail = end[len(start):]
    return len(tail) if tail == digit * len(tail) else None


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

    def __init__(self, reads: dict, interval_s: float = LAYER_INTERVAL_S):
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


def coordinate_click_through_cursor(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        ctx.before("hyprland.layers", layers_of(runtime, main))
        session = started(ctx)
        problems: list[str] = []
        try:
            renderer_of(runtime, main, session)
            registry = registry_of(runtime, session)
            ctx.before("proc.holders", len(holders_in(registry)))
            seven, point = _seven(ctx, app, session)
            screen = Screen(ctx, main, point)
            placed = ctx.product(["--session", session, "click", seven], headed=False)
            require(placed.ok, f"the click that places the cursor failed: {placed.raw[:400]}")
            covered = lambda: NAMESPACE in covering_layers(
                hyprland.query(runtime, main["signature"], "layers", pid=main["pid"]), point)
            require(raw.poll(covered, SHOW_SETTLE_S), f"no cursor layer covers {point}: {layers_of(runtime, main)}")
            require(_grab_until(screen, True) is not None, f"the screen shows no cursor at {point}")
            with harness_holder(registry):
                cleared = _grab_until(screen, False)
            require(cleared is not None, "a live holder left the arrow on screen, so pixels cannot tell drawn from cleared")
            require(_grab_until(screen, True) is not None, "the cursor did not come back after the holder left")
            start = raw.poll(lambda: (lambda now: now if presses_entered(before, now, DIGIT) == 1 else None)(
                display(tree)), SHOW_SETTLE_S)
            require(start, f"the placement click did not enter one {DIGIT}: {before!r} -> {display(tree)!r}")
            pixels_drawn = lambda: screen.shows(screen.grab())
            with keep_drawn(registry), DrawnWatch({"layers": covered, "pixels": pixels_drawn}) as watch:
                began = time.monotonic()
                clicked = ctx.product(["--session", session, "mouse-click", f"--xy={xy(point)}"], headed=True)
                finished = time.monotonic()
                raw.poll(lambda: presses_entered(start, display(tree), DIGIT) != 0, SHOW_SETTLE_S)
                raw.poll(lambda: presses_entered(start, display(tree), DIGIT) not in (0, 1), QUIET_S, floor_s=QUIET_S)
            entered = presses_entered(start, display(tree), DIGIT)
            if entered != 1:
                problems.append(f"the click under the cursor entered {entered} presses: {start!r} -> {display(tree)!r}")
            if not clicked.ok:
                problems.append(f"the headed mouse-click failed: {clicked.raw[:400]}")
            problems += drawn_problems(watch.samples, (began, finished))
            if _grab_until(screen, True) is None or not covered():
                problems.append("the cursor was not drawn over the button after the click")
            ctx.after("atspi.display", display(tree))
            ctx.after("hyprland.layers", layers_of(runtime, main))
            ctx.after("proc.holders", len(holders_in(registry)))
            ctx.restored("proc.holders", len(holders_in(registry)))
        finally:
            ended(ctx, session)
        ctx.restored("atspi.display", undo_all(tree, before))
        ctx.restored("hyprland.layers", layers_of(runtime, main))
        require(not problems, "; ".join(problems))
