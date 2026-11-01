"""Per-agent cursors on the attached `hyprland` tier.

A multi-agent session's renderer draws one cursor per agent id, each on its
own `agent-desktop-cursor` layer surface, and a desktop action that names no
agent is refused before it reaches the app or the renderer. Every leg reads
the compositor's own `j/layers` (with each surface's geometry), the renderer
records and `/proc`, and the Calculator display through AT-SPI; the
command's own `ok` is never the evidence.
"""

from __future__ import annotations

import threading
import time
from pathlib import Path

from oracle import atspi_raw as raw, hyprland
from runner.helper import activate, display, managed, one, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import (NAMESPACE, OVERLAY_LEVEL, SETTLE_S, alive, app_shown, ended, live_button,
                                       record_of)

AGENT_ENV = "AGENT_DESKTOP_AGENT_ID"
TIP_SLACK = 3.0
REFUSAL_WATCH_S = 1.5
UNDO_LIMIT = 6


def cursor_surfaces(answer: dict) -> list[dict]:
    """Every cursor layer surface in a `j/layers` answer, with its geometry."""
    found = []
    for monitor, entry in (answer or {}).items():
        for level, surfaces in (entry.get("levels") or {}).items():
            for surface in surfaces:
                if surface.get("namespace") != NAMESPACE:
                    continue
                found.append({"monitor": monitor, "level": str(level), "pid": surface.get("pid"),
                              "x": surface.get("x"), "y": surface.get("y"),
                              "w": surface.get("w"), "h": surface.get("h")})
    return sorted(found, key=lambda item: (item["pid"] or 0, item["x"] or 0, item["y"] or 0))


def surfaces_of(runtime: Path, instance: dict) -> list[dict]:
    return cursor_surfaces(hyprland.query(runtime, instance["signature"], "layers", pid=instance["pid"]))


def tip_at(surface: dict, point: tuple[float, float]) -> bool:
    """Whether the arrow's tip, one pixel inside the surface's top left, sits on `point`."""
    return (abs(surface["x"] + 1 - point[0]) <= TIP_SLACK
            and abs(surface["y"] + 1 - point[1]) <= TIP_SLACK)


def surface_at(surfaces: list[dict], point: tuple[float, float]) -> dict | None:
    found = [surface for surface in surfaces if tip_at(surface, point)]
    return found[0] if len(found) == 1 else None


def placed(surfaces: list[dict], points: list[tuple[float, float]], count: int) -> list[dict] | None:
    """`surfaces` when there are `count` of them and one cursor tip sits on each point."""
    if len(surfaces) == count and all(surface_at(surfaces, point) for point in points):
        return surfaces
    return None


def gone_problems(label: str, runtime: Path, instance: dict, session: str, owner: dict) -> list[str]:
    """After a stop: no cursor layer of that renderer, the renderer exited, and its record is gone."""
    own = lambda: [surface for surface in surfaces_of(runtime, instance) if surface["pid"] == owner["pid"]]
    raw.poll(lambda: not own() and not alive(owner) and record_of(runtime, session) is None, SETTLE_S)
    problems = []
    if own():
        problems.append(f"{label}: the compositor still lists {own()}")
    if alive(owner):
        problems.append(f"{label}: renderer {owner['pid']} is still running")
    if record_of(runtime, session) is not None:
        problems.append(f"{label}: the renderer record survived")
    return problems


def undone(tree, before: str) -> str:
    """Presses Undo until the display reads `before` again."""
    for _ in range(UNDO_LIMIT):
        if display(tree) == before:
            break
        shown = display(tree)
        activate(one(tree, role=43, name="Undo"))
        raw.poll(lambda: display(tree) != shown, SETTLE_S)
    require(display(tree) == before, f"undo never restored {before!r}; final={display(tree)!r}")
    return display(tree)


def started_multi(ctx: LegContext) -> str:
    envelope = ctx.product(["session", "start", "--cursor", "--multi-agent"])
    require(envelope.ok, f"session start --cursor --multi-agent failed: {envelope.raw[:400]}")
    return envelope.data["session_id"]


def button(ctx: LegContext, app, session: str, name: str) -> tuple[str, tuple[float, float]]:
    return live_button(ctx, app, session, name)


def agent_click(ctx: LegContext, session: str, ref: str, agent: str, by_env: bool = False):
    if by_env:
        return ctx.product(["--session", session, "click", ref], env={**ctx.tier.env, AGENT_ENV: agent})
    return ctx.product(["--session", session, "--agent-id", agent, "click", ref])


def owned_problems(label: str, surfaces: list[dict], owner: dict | None) -> list[str]:
    if owner is None:
        return [f"{label}: the session has no renderer record"]
    problems = [f"{label}: surface {surface} is not on the overlay level"
                for surface in surfaces if surface["level"] != OVERLAY_LEVEL]
    if any(surface["pid"] != owner["pid"] for surface in surfaces):
        problems.append(f"{label}: surfaces {surfaces} are not all drawn by renderer {owner['pid']}")
    if not alive(owner):
        problems.append(f"{label}: renderer {owner['pid']} is not running as recorded")
    return problems


def _two_agents(ctx: LegContext, app, runtime: Path, main: dict, session: str) -> tuple[list[str], dict]:
    """Two agents, one by flag and one by environment, get two cursors in one renderer."""
    sized = ctx.product(["--session", session, "--agent-id", "agent-b", "cursor-overlay", "enable",
                         "--size", "2"])
    require(sized.ok and sized.data.get("agent_id") == "agent-b", f"agent-b profile: {sized.raw[:400]}")
    seven, at_seven = button(ctx, app, session, "7")
    eight, at_eight = button(ctx, app, session, "8")
    for label, clicked in (("agent-a", agent_click(ctx, session, seven, "agent-a")),
                           ("agent-b", agent_click(ctx, session, eight, "agent-b", by_env=True))):
        require(clicked.ok, f"{label}'s click failed: {clicked.raw[:400]}")
    found = raw.poll(lambda: placed(surfaces_of(runtime, main), [at_seven, at_eight], 2), SETTLE_S)
    require(found, f"expected agent-a's cursor at {at_seven} and agent-b's at {at_eight}: "
                   f"{surfaces_of(runtime, main)}")
    owner = record_of(runtime, session)
    problems = owned_problems("two agents", found, owner)
    small, large = surface_at(found, at_seven), surface_at(found, at_eight)
    if not (large["w"] > small["w"] * 1.5 and large["h"] > small["h"] * 1.5):
        problems.append(f"agent-b's size 2 profile did not enlarge its cursor: a {small} b {large}")
    return problems, owner


def _second_session(ctx: LegContext, app, runtime: Path, main: dict, first: dict) -> list[str]:
    """Another session's agent gets its own renderer; the first keeps its two cursors."""
    other = started_multi(ctx)
    owner = None
    try:
        nine, at_nine = button(ctx, app, other, "9")
        clicked = agent_click(ctx, other, nine, "agent-a")
        require(clicked.ok, f"the second session's click failed: {clicked.raw[:400]}")
        found = raw.poll(lambda: placed(surfaces_of(runtime, main), [at_nine], 3), SETTLE_S)
        require(found, f"expected a third cursor at {at_nine}: {surfaces_of(runtime, main)}")
        owner = record_of(runtime, other)
        problems = owned_problems("second session", [surface_at(found, at_nine)], owner)
        if owner and owner["pid"] == first["pid"]:
            problems.append(f"the second session reused renderer {first['pid']}")
        kept = [surface for surface in found if surface["pid"] == first["pid"]]
        if len(kept) != 2:
            problems.append(f"the first session's renderer draws {len(kept)} cursors, not 2: {found}")
    finally:
        closing = ended(ctx, other)
    if owner:
        problems += gone_problems("second session end", runtime, main, other, owner)
    if not closing.ok:
        problems.append(f"the second session did not end: {closing.raw[:400]}")
    return problems


@implementation("cursor.agent_identity_and_profiles")
def agent_identity_and_profiles(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    ctx.before("hyprland.layers", surfaces_of(runtime, main))
    require(not surfaces_of(runtime, main), f"a cursor layer exists before the leg: {surfaces_of(runtime, main)}")
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started_multi(ctx)
        owner = None
        try:
            problems, owner = _two_agents(ctx, app, runtime, main, session)
            problems += _second_session(ctx, app, runtime, main, owner)
            ctx.after("hyprland.layers", surfaces_of(runtime, main))
            ctx.after("atspi.display", display(tree))
        finally:
            closing = ended(ctx, session)
        if owner:
            problems += gone_problems("session end", runtime, main, session, owner)
        if not closing.ok:
            problems.append(f"the session did not end: {closing.raw[:400]}")
        ctx.restored("atspi.display", undone(tree, before))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.layers", surfaces_of(runtime, main))


def refusal_problems(envelope) -> list[str]:
    code = (envelope.error or {}).get("code")
    if not envelope.ok and code == "INVALID_ARGS" and envelope.exit_code == 1:
        return []
    return [f"a click with no agent id was not refused with INVALID_ARGS and exit 1: {envelope.raw[:400]}"]


@implementation("cursor.missing_agent_id_refused")
def missing_agent_id_refused(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    ctx.before("hyprland.layers", surfaces_of(runtime, main))
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started_multi(ctx)
        owner = None
        try:
            seven, at_seven = button(ctx, app, session, "7")
            problems = refusal_problems(ctx.product(["--session", session, "click", seven]))
            changed = raw.poll(lambda: display(tree) != before, REFUSAL_WATCH_S)
            if changed:
                problems.append(f"the refused click reached Calculator: {display(tree)!r}")
            if surfaces_of(runtime, main) or record_of(runtime, session):
                problems.append(f"the refused click started a renderer: {surfaces_of(runtime, main)}")
            ctx.after("atspi.display", display(tree))
            ctx.after("hyprland.layers", surfaces_of(runtime, main))
            named = agent_click(ctx, session, seven, "agent-a")
            require(named.ok, f"the positive control click failed: {named.raw[:400]}")
            if not raw.poll(lambda: display(tree) != before, SETTLE_S):
                problems.append("the positive control click had no effect on the display")
            if not raw.poll(lambda: surface_at(surfaces_of(runtime, main), at_seven), SETTLE_S):
                problems.append(f"the positive control drew no cursor at {at_seven}: {surfaces_of(runtime, main)}")
            owner = record_of(runtime, session)
        finally:
            ended(ctx, session)
        if owner:
            problems += gone_problems("session end", runtime, main, session, owner)
        ctx.restored("atspi.display", undone(tree, before))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.layers", surfaces_of(runtime, main))


class ArrivalWatch:
    """Reads the display, then the cursor layers, until the display changes.

    The display is read first: once it shows the click, the click was
    dispatched before the layer read that follows, so a cursor that arrives
    before dispatch is already at the target in that read.
    """

    def __init__(self, read_display, read_surfaces, before, target):
        self.read_display, self.read_surfaces = read_display, read_surfaces
        self.before, self.target = before, target
        self.verdict: tuple[bool, list[dict]] | None = None
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self) -> None:
        while not self._stop.is_set():
            if self.read_display() != self.before:
                surfaces = self.read_surfaces()
                self.verdict = (surface_at(surfaces, self.target) is not None, surfaces)
                return
            time.sleep(0.005)

    def __enter__(self):
        self._thread.start()
        return self

    def __exit__(self, *exc):
        self._thread.join(SETTLE_S)
        self._stop.set()
        self._thread.join()


@implementation("cursor.arrival_ack_before_dispatch")
def arrival_ack_before_dispatch(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    ctx.before("hyprland.layers", surfaces_of(runtime, main))
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started_multi(ctx)
        owner = None
        try:
            seven, at_seven = button(ctx, app, session, "7")
            placed = agent_click(ctx, session, seven, "agent-a")
            require(placed.ok, f"the click that starts the renderer failed: {placed.raw[:400]}")
            owner = record_of(runtime, session)
            require(raw.poll(lambda: display(tree) != before, SETTLE_S), "the first click had no effect")
            shown = display(tree)
            far, at_far = button(ctx, app, session, "3")
            with ArrivalWatch(lambda: display(tree), lambda: surfaces_of(runtime, main), shown, at_far) as watch:
                clicked = agent_click(ctx, session, far, "agent-a")
            require(clicked.ok, f"the watched click failed: {clicked.raw[:400]}")
            require(watch.verdict, "the watched click had no effect on the display")
            arrived, surfaces = watch.verdict
            problems = [] if arrived else [
                f"the click reached Calculator before the cursor arrived at {at_far}: {surfaces}"]
            ctx.after("hyprland.layers", surfaces)
            ctx.after("atspi.display", display(tree))
        finally:
            ended(ctx, session)
        if owner:
            problems += gone_problems("session end", runtime, main, session, owner)
        ctx.restored("atspi.display", undone(tree, before))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.layers", surfaces_of(runtime, main))
