"""The agent cursor renderer on the attached `hyprland` tier.

On Hyprland the cursor overlay is drawn by a renderer process that one
`agent-desktop` command starts and later commands reach over its socket. It
draws one `zwlr_layer_shell_v1` surface on the overlay layer, namespace
`agent-desktop-cursor`. Every leg reads the compositor's own `j/layers`, the
renderer record under the shared runtime directory, `/proc` and the pixels
`grim` reads; the command's own `ok` is never the evidence.
"""

from __future__ import annotations

import contextlib
import json
import os
import signal
from pathlib import Path

from oracle import atspi_raw as raw, hyprland, pixels, proc
from runner import second_instance
from runner.helper import command, display, managed, nodes, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_binding import pair_env
from scenarios.hyprland_capture_common import capture_path, ids_by_pid, saved_image
from scenarios.hyprland_capture_screen import grim
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance, shared
from scenarios.hyprland_desktop import _undo, keeps_focus
from scenarios.hyprland_focus_controls import (SPECIAL, control, cursor, lua_call, restore_desktop, shown,
                                               toggle_argv)

NAMESPACE = "agent-desktop-cursor"
OVERLAY_LEVEL = "3"
SETTLE_S = 5.0
ARROW_BOX = 14
TOLERANCE = 0.05
LEASE_ENV = "AGENT_DESKTOP_INTERACTION_LEASE"


def cursor_layers(answer: dict) -> list[dict]:
    """Every layer surface with the cursor namespace in a `j/layers` answer."""
    found = []
    for monitor, entry in (answer or {}).items():
        for level, surfaces in (entry.get("levels") or {}).items():
            for surface in surfaces:
                if surface.get("namespace") == NAMESPACE:
                    found.append({"monitor": monitor, "level": str(level), "pid": surface.get("pid")})
    return sorted(found, key=lambda item: (item["monitor"], item["pid"] or 0))


def layers_of(runtime: Path, instance: dict) -> list[dict]:
    return cursor_layers(hyprland.query(runtime, instance["signature"], "layers", pid=instance["pid"]))


def records(runtime: Path) -> list[dict]:
    """Every renderer record under the runtime directory, read as files."""
    base = runtime / "agent-desktop" / "cursor"
    found = []
    for path in sorted(base.glob("*/*.json")) if base.is_dir() else ():
        try:
            found.append(json.loads(path.read_text()))
        except (OSError, ValueError):
            found.append({"unreadable": path.name})
    return found


def record_of(runtime: Path, session: str) -> dict | None:
    own = [item for item in records(runtime) if item.get("session_id") == session]
    require(len(own) <= 1, f"session {session} has {len(own)} renderer records: {own}")
    return own[0] if own else None


def start_time(pid: int) -> int | None:
    try:
        return proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]
    except (OSError, ValueError, IndexError):
        return None


def alive(owner: dict) -> bool:
    return start_time(owner["pid"]) == owner["start_time"]


def sole_layer(runtime: Path, instance: dict) -> dict | None:
    found = layers_of(runtime, instance)
    return found[0] if len(found) == 1 else None


def started(ctx: LegContext, env: dict | None = None) -> str:
    envelope = ctx.product(["session", "start", "--cursor"], env=env)
    require(envelope.ok, f"session start --cursor failed: {envelope.raw[:400]}")
    require(envelope.data.get("rendered") is True, f"the cursor was not rendered: {envelope.raw[:400]}")
    return envelope.data["session_id"]


def renderer_of(runtime: Path, instance: dict, session: str) -> dict:
    """The live renderer's record: one cursor layer whose pid the session's record names."""
    layer = raw.poll(lambda: sole_layer(runtime, instance), SETTLE_S)
    require(layer, f"expected one {NAMESPACE} layer, found {layers_of(runtime, instance)}")
    require(layer["level"] == OVERLAY_LEVEL, f"the cursor is on layer level {layer['level']}, not overlay")
    owner = record_of(runtime, session)
    require(owner, f"session {session} has no renderer record")
    require(layer["pid"] == owner["pid"], f"the layer pid {layer['pid']} is not the record's {owner['pid']}")
    require(alive(owner), f"the recorded renderer {owner['pid']} is not running as recorded")
    return owner


def gone_problems(label: str, runtime: Path, instance: dict | None, session: str, owner: dict) -> list[str]:
    """After a stop: no cursor layer, the renderer exited, and its record is gone."""
    layers = (lambda: layers_of(runtime, instance)) if instance else (lambda: [])
    raw.poll(lambda: not layers() and not alive(owner) and record_of(runtime, session) is None, SETTLE_S)
    problems = []
    if layers():
        problems.append(f"{label}: the compositor still lists {layers()}")
    if alive(owner):
        problems.append(f"{label}: renderer {owner['pid']} is still running")
    if record_of(runtime, session) is not None:
        problems.append(f"{label}: the renderer record survived")
    return problems


def outcome_problems(label: str, envelope, expected: str) -> list[str]:
    found = ((envelope.data or {}).get("details") or {}).get("cursor_renderer")
    if envelope.ok and found == expected:
        return []
    return [f"{label}: expected cursor_renderer {expected!r}, got {found!r}: {envelope.raw[:400]}"]


def hygiene_problems(owner: dict) -> list[str]:
    """The renderer leads its own process group and inherited no lease variable."""
    pid = owner["pid"]
    _, group, _, env = proc.facts(pid)
    problems = []
    if group != pid:
        problems.append(f"renderer {pid} is in process group {group}, not its own")
    leaked = sorted(key for key in env if key.startswith(LEASE_ENV))
    if leaked:
        problems.append(f"renderer {pid} inherited {leaked}")
    return problems


def kill_exactly(owner: dict) -> None:
    """SIGKILL the recorded renderer through a pidfd, only while its start time matches."""
    descriptor = os.pidfd_open(owner["pid"])
    try:
        require(alive(owner), f"renderer {owner['pid']} is not the recorded process")
        signal.pidfd_send_signal(descriptor, signal.SIGKILL)
    finally:
        os.close(descriptor)
    require(raw.poll(lambda: not alive(owner), SETTLE_S), f"renderer {owner['pid']} survived SIGKILL")


def ended(ctx: LegContext, session: str, env: dict | None = None):
    return ctx.product(["session", "end", session], env=env)


def _reads(ctx: LegContext, stage: str, runtime: Path, main: dict) -> None:
    getattr(ctx, stage)("hyprland.layers", layers_of(runtime, main))
    getattr(ctx, stage)("proc.renderer_records", records(runtime))


@implementation("cursor.session_renderer_lifetime")
def session_renderer_lifetime(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    _reads(ctx, "before", runtime, main)
    require(not layers_of(runtime, main), f"a cursor layer exists before the leg: {layers_of(runtime, main)}")
    session = started(ctx)
    try:
        owner = renderer_of(runtime, main, session)
        problems = hygiene_problems(owner)
        again = ctx.product(["--session", session, "cursor-overlay", "enable"])
        if not again.ok or again.data.get("rendered") is not True:
            problems.append(f"a second enable was not rendered: {again.raw[:400]}")
        kept = renderer_of(runtime, main, session)
        if (kept["pid"], kept["generation"]) != (owner["pid"], owner["generation"]):
            problems.append(f"a second enable replaced renderer {owner['pid']} with {kept['pid']}")
        _reads(ctx, "after", runtime, main)
    finally:
        envelope = ended(ctx, session)
    problems += outcome_problems("session end", envelope, "stopped")
    problems += gone_problems("session end", runtime, main, session, owner)
    require(not problems, "; ".join(problems))
    _reads(ctx, "restored", runtime, main)


def live_button(ctx: LegContext, app, session: str, name: str) -> tuple[str, tuple[float, float]]:
    """The ref of the one button named `name` and the centre of its live bounds. `find` prints
    bounds only for a match without a ref, so the bounds come from `get`."""
    found = command(ctx, ["--session", session, "find", "--exact", "--limit", "0", "--role", "button",
                          "--name", name], app).data
    items = [item for item in nodes(found) if isinstance(item.get("ref_id"), str)]
    require(len(items) == 1, f"expected one ref for button {name}, found {len(items)}")
    ref = items[0]["ref_id"]
    read = command(ctx, ["--session", session, "get", ref, "--property", "bounds"]).data
    bounds = read.get("value") or {}
    require(read.get("live") is True and bounds, f"button {name} has no live bounds: {read}")
    return ref, (bounds["x"] + bounds["width"] / 2, bounds["y"] + bounds["height"] / 2)


def _seven(ctx: LegContext, app, session: str) -> tuple[str, tuple[float, float]]:
    return live_button(ctx, app, session, "7")


@implementation("cursor.no_focus_or_input_interference")
def no_focus_or_input_interference(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    read = lambda: desktop(ctx.tier, main)
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        ctx.before("hyprland.desktop", read())
        problems = keeps_focus(ctx.tier, read())
        with Sampler({"desktop": read}) as sampler:
            session = started(ctx)
            try:
                owner = renderer_of(runtime, main, session)
                seven, _ = _seven(ctx, app, session)
                clicked = ctx.product(["--session", session, "click", seven])
                require(clicked.ok, f"the click under the cursor failed: {clicked.raw[:400]}")
                after = raw.poll(lambda: display(tree) if display(tree) != before else None, SETTLE_S)
                require(after, "the click under the cursor had no effect on the display")
                ctx.after("atspi.display", after)
                disabled = ctx.product(["--session", session, "cursor-overlay", "disable"])
                problems += outcome_problems("disable", disabled, "stopped")
                problems += gone_problems("disable", runtime, main, session, owner)
            finally:
                ended(ctx, session)
        ctx.after("hyprland.desktop", read())
        problems += sampler.changes + keeps_focus(ctx.tier, read())
        ctx.restored("atspi.display", _undo(tree, before))
        require(not problems, "; ".join(problems))
        ctx.restored("hyprland.desktop", read())


def monitor_at(monitors: list[dict], point: tuple[float, float]) -> dict:
    """The monitor whose logical rectangle holds `point`."""
    for monitor in monitors:
        width, height = monitor["width"] / monitor["scale"], monitor["height"] / monitor["scale"]
        if monitor["x"] <= point[0] < monitor["x"] + width and monitor["y"] <= point[1] < monitor["y"] + height:
            return monitor
    raise AssertionError(f"no monitor shows {point}")


def arrow_box(origin: tuple[float, float], point: tuple[float, float], scale: float,
              size: tuple[int, int]) -> tuple[int, int, int, int]:
    """The pixel region an arrow with its tip at `point` covers, in an image whose top left is `origin`."""
    x = max(0, int((point[0] - origin[0]) * scale) - 1)
    y = max(0, int((point[1] - origin[1]) * scale) - 1)
    side = int(ARROW_BOX * scale)
    return (x, y, max(1, min(side, size[0] - x)), max(1, min(side, size[1] - y)))


def _window(ctx: LegContext, window_id: str, name: str) -> pixels.Image:
    path = capture_path(ctx.tier, name)
    return saved_image(ctx.product(["screenshot", "--window-id", window_id, str(path)]), path)


def _frame(runtime: Path, main: dict, pid: int) -> dict:
    found = [item for item in hyprland.query(runtime, main["signature"], "clients", pid=main["pid"])
             if item.get("pid") == pid]
    require(len(found) == 1, f"pid {pid} has {len(found)} toplevels")
    return found[0]


def focus_monitor_argv(name: str, lua: bool) -> list[str]:
    if lua:
        return ["dispatch", lua_call("hl.dsp.focus", {"monitor": name})]
    hyprland.lua_string(name)
    return ["dispatch", "focusmonitor", name]


def focused_monitor(tier) -> str:
    main = main_instance(tier)
    found = [item["name"] for item in hyprland.query(shared(tier), main["signature"], "monitors", pid=main["pid"])
             if item.get("focused")]
    require(len(found) == 1, f"expected one focused monitor, found {found}")
    return found[0]


def _monitor_id(tier, name: str) -> int | None:
    main = main_instance(tier)
    found = [item["id"] for item in hyprland.query(shared(tier), main["signature"], "monitors", pid=main["pid"])
             if item["name"] == name]
    return found[0] if found else None


@contextlib.contextmanager
def app_shown(tier, pid: int, monitor: str | None = None):
    """The app's special workspace open on `monitor`, the focused monitor when None, for the leg.

    The run starts every app on a silent special workspace, and the cursor is parked over a hidden
    target, so a leg that needs the cursor drawn over its app opens that workspace first. `j/monitors`
    confirms it is shown and `j/clients` that the app is on that monitor before the leg acts. On exit
    every special workspace is closed, the bystander has focus and the OS pointer is back where it
    was. Yields the monitor's name.
    """
    runtime, main = shared(tier), main_instance(tier)
    holder, point = bystander(tier), cursor(tier)
    workspace = (_frame(runtime, main, pid).get("workspace") or {}).get("name", "")
    require(workspace.startswith(SPECIAL), f"the app is on {workspace!r}, not on a special workspace")
    require(not any(special for _, special in shown(tier).values()),
            f"a special workspace is open before the leg: {shown(tier)}")
    try:
        target = monitor or focused_monitor(tier)
        if focused_monitor(tier) != target:
            control(tier, lambda lua: focus_monitor_argv(target, lua))
            require(raw.poll(lambda: focused_monitor(tier) == target, SETTLE_S), f"{target} never took focus")
        control(tier, lambda lua: toggle_argv(workspace, lua))
        require(raw.poll(lambda: shown(tier).get(target, ("", ""))[1] == workspace, SETTLE_S),
                f"{workspace} is not shown on {target}: {shown(tier)}")
        require(raw.poll(lambda: _frame(runtime, main, pid).get("monitor") == _monitor_id(tier, target), SETTLE_S),
                f"the app did not move to {target}")
        yield target
    finally:
        restore_desktop(tier, holder, point)


@implementation("cursor.window_capture_excludes_overlay")
def window_capture_excludes_overlay(ctx: LegContext) -> None:
    """The cursor shows on the screen and never in a window capture.

    Both captures are taken with the cursor shown and again after `disable`,
    so the click's own effect on the app is in both and only the cursor differs.
    """
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
        window_id = ids_by_pid(ctx.product(["list-windows"]), [app.pid])[app.pid]
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started(ctx)
        try:
            owner = renderer_of(runtime, main, session)
            seven, point = _seven(ctx, app, session)
            monitor = monitor_at(hyprland.query(runtime, main["signature"], "monitors", pid=main["pid"]), point)
            pressed = ctx.product(["--session", session, "click", seven])
            require(pressed.ok, f"the click that places the cursor failed: {pressed.raw[:400]}")
            require(raw.poll(lambda: display(tree) != before, SETTLE_S), "the click had no effect")
            ctx.after("atspi.display", display(tree))
            shown = grim(ctx.tier, monitor["name"]), _window(ctx, window_id, "cursor-shown")
            ctx.before("pixels.window", [shown[1].width, shown[1].height])
            disabled = ctx.product(["--session", session, "cursor-overlay", "disable"])
            problems = outcome_problems("disable", disabled, "stopped")
            problems += gone_problems("disable", runtime, main, session, owner)
            hidden = grim(ctx.tier, monitor["name"]), _window(ctx, window_id, "cursor-hidden")
            ctx.after("pixels.window", [hidden[1].width, hidden[1].height])
            frame = _frame(runtime, main, app.pid)
            screen = arrow_box((monitor["x"], monitor["y"]), point, monitor["scale"],
                               (shown[0].width, shown[0].height))
            window = arrow_box(tuple(frame["at"]), point, shown[1].width / frame["size"][0],
                               (shown[1].width, shown[1].height))
            if pixels.similar(shown[0], hidden[0], screen, tolerance=TOLERANCE):
                problems.append(f"the screen shows no cursor at {point}: the positive control failed")
            if not pixels.similar(shown[1], hidden[1], window, tolerance=TOLERANCE):
                problems.append(f"the window capture shows the cursor at {point}")
        finally:
            ended(ctx, session)
        ctx.restored("atspi.display", _undo(tree, before))
        require(not problems, "; ".join(problems))
        ctx.restored("pixels.window", [hidden[1].width, hidden[1].height])


@contextlib.contextmanager
def _nested(ctx: LegContext):
    nested = second_instance.ensure(ctx.tier)
    try:
        yield nested
    finally:
        second_instance.stop(ctx.tier)


def _crash_and_replace(ctx: LegContext, runtime: Path, main: dict, session: str) -> tuple[list[str], dict]:
    """`disable` stops the renderer; a killed renderer leaves no layer and the next enable replaces it."""
    first = renderer_of(runtime, main, session)
    disabled = ctx.product(["--session", session, "cursor-overlay", "disable"])
    problems = outcome_problems("disable", disabled, "stopped")
    problems += gone_problems("disable", runtime, main, session, first)
    enabled = ctx.product(["--session", session, "cursor-overlay", "enable"])
    require(enabled.ok and enabled.data.get("rendered") is True, f"re-enable: {enabled.raw[:400]}")
    crashed = renderer_of(runtime, main, session)
    kill_exactly(crashed)
    if not raw.poll(lambda: not layers_of(runtime, main), SETTLE_S):
        problems.append(f"the compositor kept the crashed cursor: {layers_of(runtime, main)}")
    again = ctx.product(["--session", session, "cursor-overlay", "enable"])
    require(again.ok and again.data.get("rendered") is True, f"enable after a crash: {again.raw[:400]}")
    replacement = renderer_of(runtime, main, session)
    if replacement["generation"] == crashed["generation"]:
        problems.append(f"the replacement {replacement} reuses the crashed renderer's generation")
    return problems, replacement


def _compositor_loss(ctx: LegContext, runtime: Path) -> list[str]:
    """A renderer whose compositor stops exits and removes its record; `session end` then finds none."""
    with _nested(ctx) as nested:
        env = pair_env(ctx.tier, nested["signature"], nested["wayland"])
        lost = started(ctx, env)
        try:
            orphan = renderer_of(runtime, nested, lost)
            second_instance.stop(ctx.tier)
            problems = gone_problems("compositor loss", runtime, None, lost, orphan)
        finally:
            closing = ended(ctx, lost, env)
    return problems + outcome_problems("session end after compositor loss", closing, "not_running")


@implementation("cursor.disable_end_crash_and_compositor_loss")
def disable_end_crash_and_compositor_loss(ctx: LegContext) -> None:
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    _reads(ctx, "before", runtime, main)
    session = started(ctx)
    try:
        problems, replacement = _crash_and_replace(ctx, runtime, main, session)
        _reads(ctx, "after", runtime, main)
    finally:
        envelope = ended(ctx, session)
    problems += outcome_problems("session end", envelope, "stopped")
    problems += gone_problems("session end", runtime, main, session, replacement)
    problems += _compositor_loss(ctx, runtime)
    require(not problems, "; ".join(problems))
    _reads(ctx, "restored", runtime, main)
