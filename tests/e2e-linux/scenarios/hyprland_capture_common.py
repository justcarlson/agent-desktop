"""Painted terminals and pixel verdicts for the Hyprland capture legs.

A painted terminal is a foot toplevel whose whole surface is one colour the
leg chose, so a capture shows whose pixels it holds. A ticker repaints with a
new colour every period and logs when it wrote each one, so the colour of a
capture names the tick it shows. A capture is fresh when it shows a tick no
older than the last one the ticker had written, with a margin for the
terminal to draw it, before the product was asked. A slow painter is a GTK
window that draws only when the compositor asks and takes a set time to do
it; a capture of one is drawn after the request only when the tick it shows
was first drawn at or after the request, with no margin. A slow painter in
view draws for every frame, so a capture of it is current when it shows the
newest tick committed before the request, drawn within one draw of it.
"""

from __future__ import annotations

import collections
import contextlib
import sys
import time
from pathlib import Path

from oracle import atspi_raw as raw, pixels
from runner.helper import require
from runner.session import HARNESS_ROOT
from scenarios.capture import decode
from scenarios.hyprland_common import main_instance, shared
from oracle import hyprland
from scenarios.hyprland_windows import terminal, toplevel

PAINTER = HARNESS_ROOT / "apps" / "terminal_painter.py"
SLOW_PAINTER = HARNESS_ROOT / "apps" / "slow_painter.py"
TICK_BLUE = 0x40
DRAW_MARGIN_NS = 100_000_000
COMMIT_MARGIN_NS = 10_000_000
IN_VIEW_AGE_NS = 300_000_000
CHANNEL_DELTA = 3
SAMPLE_STEP = 7


def painter_command(log: Path, mode: str, value: str) -> list[str]:
    return [sys.executable, "-I", str(PAINTER), mode, value, str(log)]


def written(log: Path) -> list[tuple[int, int]]:
    """Each tick the painter logged and when, skipping a line it is still writing."""
    try:
        lines = log.read_text().splitlines()
    except FileNotFoundError:
        return []
    ticks = []
    for line in lines:
        parts = line.split()
        if len(parts) == 2 and all(part.isdigit() for part in parts):
            ticks.append((int(parts[0]), int(parts[1])))
    return ticks


@contextlib.contextmanager
def painted(tier, app_id: str, mode: str, value: str):
    """A painted foot toplevel on the launch special workspace, and its log."""
    log = tier.runtime_dir / f"painter-{app_id}.log"
    log.unlink(missing_ok=True)
    with terminal(tier, app_id, painter_command(log, mode, value)) as (owned, client):
        require(raw.poll(lambda: written(log), 5.0), f"{app_id} never painted")
        yield owned, client, log


@contextlib.contextmanager
def slow_painted(tier, app_id: str, delay_ms: int):
    """A slow painter on the launch special workspace that takes `delay_ms` to draw, and its log."""
    log = tier.runtime_dir / f"painter-{app_id}.log"
    log.unlink(missing_ok=True)
    with toplevel(tier, app_id, [sys.executable, "-I", str(SLOW_PAINTER), app_id, str(delay_ms), str(log)]) as (
            owned, client):
        require(raw.poll(lambda: written(log), 5.0), f"{app_id} never painted")
        yield owned, client, log


def colour(text: str) -> tuple[int, int, int]:
    return (int(text[0:2], 16), int(text[2:4], 16), int(text[4:6], 16))


def dominant(image: pixels.Image) -> tuple[tuple[int, int, int], float]:
    """The commonest sampled colour and the share of samples that hold it."""
    counts = collections.Counter(image.pixel(x, y)
                                 for y in range(0, image.height, SAMPLE_STEP)
                                 for x in range(0, image.width, SAMPLE_STEP))
    found, count = counts.most_common(1)[0]
    return found, count / sum(counts.values())


def near(a: tuple[int, int, int], b: tuple[int, int, int]) -> bool:
    return max(abs(left - right) for left, right in zip(a, b)) <= CHANNEL_DELTA


def tick_of(found: tuple[int, int, int]) -> int | None:
    """The tick a ticker colour encodes, or None for a colour no tick has."""
    red, green, blue = found
    return red + 256 * green if blue == TICK_BLUE else None


def settled_tick(ticks: list[tuple[int, int]], mark_ns: int, margin_ns: int = DRAW_MARGIN_NS) -> int:
    """The last tick written at least `margin_ns` before `mark_ns`."""
    return max((tick for tick, at in ticks if at <= mark_ns - margin_ns), default=0)


def fresh(tick: int | None, ticks: list[tuple[int, int]], mark_ns: int) -> bool:
    return tick is not None and tick >= settled_tick(ticks, mark_ns)


def first_drawn(tick: int | None, ticks: list[tuple[int, int]]) -> int | None:
    return min((at for logged, at in ticks if logged == tick), default=None)


def drawn_after(tick: int | None, ticks: list[tuple[int, int]], mark_ns: int) -> bool:
    """The tick was first drawn at or after `mark_ns`, so no pixel of it is older than the request."""
    at = first_drawn(tick, ticks)
    return at is not None and at >= mark_ns


def newest_drawn(tick: int | None, ticks: list[tuple[int, int]], mark_ns: int) -> bool:
    """The tick is no older than the newest one committed before `mark_ns`, and was drawn one draw before it at most.

    A window Hyprland shows draws for every frame, so the capture holds its
    newest commit. A window Hyprland stopped asking for frames holds a tick
    drawn seconds before, which this rejects.
    """
    at = first_drawn(tick, ticks)
    return (at is not None and tick >= settled_tick(ticks, mark_ns, COMMIT_MARGIN_NS)
            and at >= mark_ns - IN_VIEW_AGE_NS)


def saved_image(envelope, path: Path) -> pixels.Image:
    """The image the product wrote to `path`, read and removed."""
    require(envelope.ok, f"{envelope.argv[1:3]} failed: {envelope.raw[:400]}")
    try:
        return decode(path.read_bytes(), "png")
    finally:
        path.unlink(missing_ok=True)


def capture_path(tier, name: str) -> Path:
    path = tier.runtime_dir / f"capture-{name}.png"
    path.unlink(missing_ok=True)
    return path


def monotonic_ns() -> int:
    return time.monotonic_ns()


def option(tier, name: str) -> dict:
    main = main_instance(tier)
    return hyprland.query(shared(tier), main["signature"], f"getoption {name}", pid=main["pid"])


def ids_by_pid(envelope, pids: list[int]) -> dict[int, str]:
    """The one listed id of each pid's window."""
    require(envelope.ok and isinstance(envelope.data, list), f"list-windows failed: {envelope.raw[:400]}")
    found = {}
    for pid in pids:
        entries = [item.get("id") for item in envelope.data if item.get("pid") == pid]
        require(len(entries) == 1, f"pid {pid} is listed {len(entries)} times")
        found[pid] = entries[0]
    return found
