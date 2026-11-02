"""Delayed and withheld arrival acknowledgements on the attached `hyprland` tier.

A headless click first sends its cursor's travel to the session renderer and
waits up to the 900 ms arrival timeout for the renderer to acknowledge the
arrival. The renderer's `arrival-ack-for-test` flag file, read only from its
own registry directory, holds that acknowledgement: a number delays it to
that many milliseconds after the renderer received the travel, `withhold`
never sends it. The renderer traces each held answer with CLOCK_MONOTONIC
times, the same clock as `time.monotonic_ns()` here.

The oracle is the Calculator display, read through AT-SPI in a loop while the
click runs. A read that starts after an instant and still shows the old
display proves the click had not reached the app at that instant. A delayed
acknowledgement inside the deadline must come before the first app event; a
withheld one must hold the event until the arrival timeout has passed, after
which the click still lands once, because a travel that fails is presentation
only and never refuses the action.
"""

from __future__ import annotations

import contextlib
import json
import os
import threading
import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner.helper import display, managed, require, root
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import SETTLE_S, app_shown, ended, record_of
from scenarios.hyprland_cursor_agents import (agent_click, button, gone_problems, started_multi, surfaces_of,
                                              undone)
from scenarios.hyprland_cursor_hide import registry_of

ACK_FLAG = "arrival-ack-for-test"
ACK_TRACE = "arrival-ack-trace-for-test"
ARRIVAL_TIMEOUT_MS = 900
DELAY_MS = 600
SEND_SLACK_MS = 50
QUIET_S = 0.5
POLL_S = 0.002
AGENT = "agent-a"
NS_PER_MS = 1_000_000


def read_trace(path: Path) -> list[dict]:
    """Every line the renderer traced for a held acknowledgement."""
    try:
        text = path.read_text()
    except OSError:
        return []
    return [json.loads(line) for line in text.splitlines() if line.strip()]


def transitions(reads: list[tuple[int, str]], before: str) -> list[str]:
    """Each new display value, in the order the reads saw them."""
    seen, current = [], before
    for _, value in reads:
        if value != current:
            seen.append(value)
            current = value
    return seen


def last_unchanged_start(reads: list[tuple[int, str]], before: str) -> int | None:
    """When the last read that still showed `before`, ahead of the first change, started."""
    last = None
    for started, value in reads:
        if value != before:
            return last
        last = started
    return None


def event_problems(reads: list[tuple[int, str]], before: str, after_ns: int, label: str) -> list[str]:
    """The click reached the app once, and a read that started at or after `after_ns` still saw
    the old display."""
    seen = transitions(reads, before)
    if len(seen) != 1:
        return [f"{label}: the display changed {len(seen)} times, not once: {seen}"]
    unchanged = last_unchanged_start(reads, before)
    if unchanged is None or unchanged < after_ns:
        gap = None if unchanged is None else (unchanged - after_ns) / NS_PER_MS
        return [f"{label}: no read at or after the bound still showed {before!r}; last unchanged read "
                f"{gap} ms from it, so the click may have reached the app before"]
    return []


def delayed_problems(trace: list[dict], reads: list[tuple[int, str]], before: str,
                     delay_ms: int = DELAY_MS) -> list[str]:
    """Each acknowledgement was held to `delay_ms` after the renderer received its travel and
    sent inside the arrival timeout, and the click reached the app after the last of them."""
    held = [line for line in trace if line.get("hold") == "delay"]
    if not held or len(held) != len(trace):
        return [f"expected only delayed acknowledgements in the trace, got {trace}"]
    problems = []
    for line in held:
        waited = (line["acked_ns"] - line["received_ns"]) / NS_PER_MS
        if line.get("delay_ms") != delay_ms or not delay_ms <= waited < ARRIVAL_TIMEOUT_MS:
            problems.append(f"acknowledgement sent {waited:.1f} ms after its travel, not at "
                            f"{delay_ms} ms inside {ARRIVAL_TIMEOUT_MS} ms: {line}")
    acked = max(line["acked_ns"] for line in held)
    return problems + event_problems(reads, before, acked, "delayed acknowledgement")


def withheld_problems(trace: list[dict], reads: list[tuple[int, str]], before: str) -> list[str]:
    """No acknowledgement was sent, and the click reached the app only once the arrival timeout
    of the first travel had passed."""
    held = [line for line in trace if line.get("hold") == "withhold"]
    if not held or len(held) != len(trace):
        return [f"expected only withheld acknowledgements in the trace, got {trace}"]
    if any(line.get("acked_ns") is not None for line in held):
        return [f"a withheld acknowledgement was sent: {held}"]
    first = min(line["received_ns"] for line in held)
    bound = first + (ARRIVAL_TIMEOUT_MS - SEND_SLACK_MS) * NS_PER_MS
    return event_problems(reads, before, bound, "withheld acknowledgement")


class DisplayTimeline:
    """Reads the display in a loop while a command runs, keeping each read's monotonic start,
    until the display has changed and then stayed put for `quiet_s`."""

    def __init__(self, read_display, before: str, quiet_s: float = QUIET_S):
        self.read_display, self.before, self.quiet_s = read_display, before, quiet_s
        self.reads: list[tuple[int, str]] = []
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self) -> None:
        changed_at = None
        while not self._stop.is_set():
            started = time.monotonic_ns()
            value = self.read_display()
            self.reads.append((started, value))
            if value != self.before and changed_at is None:
                changed_at = time.monotonic()
            if changed_at is not None and time.monotonic() - changed_at >= self.quiet_s:
                return
            time.sleep(POLL_S)

    def __enter__(self):
        self._thread.start()
        return self

    def __exit__(self, *exc):
        self._thread.join(SETTLE_S)
        self._stop.set()
        self._thread.join()


@contextlib.contextmanager
def held_ack(registry: Path, hold: str):
    """The renderer's arrival-ack test flag naming `hold`, and its trace, removed on exit."""
    flag, trace = registry / ACK_FLAG, registry / ACK_TRACE
    trace.unlink(missing_ok=True)
    descriptor = os.open(flag, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w") as handle:
        handle.write(hold)
    try:
        yield trace
    finally:
        flag.unlink(missing_ok=True)
        trace.unlink(missing_ok=True)


def _held_click(ctx: LegContext, hold: str, judge) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    ctx.before("hyprland.layers", surfaces_of(runtime, main))
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started_multi(ctx)
        owner, problems = None, []
        try:
            seven, _ = button(ctx, app, session, "7")
            placed = agent_click(ctx, session, seven, AGENT)
            require(placed.ok, f"the click that starts the renderer failed: {placed.raw[:400]}")
            owner = record_of(runtime, session)
            require(raw.poll(lambda: display(tree) != before, SETTLE_S), "the first click had no effect")
            registry, shown = registry_of(runtime, session), display(tree)
            ctx.before("proc.arrival_ack_trace", read_trace(registry / ACK_TRACE))
            three, _ = button(ctx, app, session, "3")
            with held_ack(registry, hold) as trace_path:
                with DisplayTimeline(lambda: display(tree), shown) as timeline:
                    clicked = agent_click(ctx, session, three, AGENT)
                trace = read_trace(trace_path)
            ctx.after("proc.arrival_ack_trace", trace)
            ctx.after("hyprland.layers", surfaces_of(runtime, main))
            require(clicked.ok, f"the held click failed: {clicked.raw[:400]}")
            held_problems = judge(trace, timeline.reads, shown)
            problems += held_problems
            proven = "proved" if not held_problems else "not proved"
            label = f"{hold}: {proven}, click ok in {clicked.elapsed_ms:.0f} ms"
            ctx.finding(_held_finding(label, trace, timeline.reads, shown))
            problems += _recovery_problems(ctx, app, session, tree, registry)
            ctx.restored("proc.arrival_ack_trace", read_trace(registry / ACK_TRACE))
            ctx.after("atspi.display", display(tree))
        finally:
            ended(ctx, session)
        if owner:
            problems += gone_problems("session end", runtime, main, session, owner)
        ctx.restored("atspi.display", undone(tree, before))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.layers", surfaces_of(runtime, main))


def _held_finding(label: str, trace: list[dict], reads: list[tuple[int, str]], before: str) -> str:
    """What the held click showed, for the run log."""
    unchanged = last_unchanged_start(reads, before)
    first = min((line["received_ns"] for line in trace), default=None)
    held_ms = None if unchanged is None or first is None else round((unchanged - first) / NS_PER_MS, 1)
    return (f"{label}; {len(trace)} held acknowledgement(s); the app still showed {before!r} in a read "
            f"started {held_ms} ms after the first travel was received, then {transitions(reads, before)}")


def _recovery_problems(ctx: LegContext, app, session: str, tree, registry: Path) -> list[str]:
    """With the flag gone, the next click lands at once and traces nothing."""
    one_more, _ = button(ctx, app, session, "1")
    current = display(tree)
    again = agent_click(ctx, session, one_more, AGENT)
    problems = []
    if not again.ok or not raw.poll(lambda: display(tree) != current, SETTLE_S):
        problems.append(f"the click after the flag was removed did not land: {again.raw[:400]}")
    if again.elapsed_ms >= ARRIVAL_TIMEOUT_MS * 2:
        problems.append(f"the click after the flag was removed took {again.elapsed_ms:.0f} ms")
    if (registry / ACK_TRACE).exists():
        problems.append("the renderer traced a held acknowledgement with no flag set")
    return problems


def arrival_ack_delayed(ctx: LegContext) -> None:
    _held_click(ctx, str(DELAY_MS), delayed_problems)


def arrival_ack_withheld(ctx: LegContext) -> None:
    _held_click(ctx, "withhold", withheld_problems)
