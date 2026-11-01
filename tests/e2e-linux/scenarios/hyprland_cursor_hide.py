"""Hiding the agent cursors on the attached `hyprland` tier.

A command that reads the screen or sends headed input first hides every
agent cursor in the compositor's renderer registry: it writes a holder file,
waits for each renderer's `hide` acknowledgement, checks `j/layers` and only
then works. Every leg reads the compositor's `j/layers`, `grim` pixels, the
holder files, `/proc` and the app's own log or AT-SPI state; the command's
own `ok` is never the evidence.
"""

from __future__ import annotations

import contextlib
import json
import os
import secrets
import signal
import time
from pathlib import Path

from oracle import atspi_raw as raw, hyprland, pixels
from runner.helper import activate, changed, display, managed, one, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_common import capture_path, saved_image
from scenarios.hyprland_capture_screen import grim
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import (NAMESPACE, TOLERANCE, _seven, alive, arrow_box, ended, kill_exactly,
                                       layers_of, monitor_at, record_of, renderer_of, start_time, started)
from scenarios.hyprland_focus_controls import lua_call
from scenarios.hyprland_pointer import nested
from scenarios.hyprland_pointer_pad import (XWAYLAND, covering_layers, events, instance_env, pad_client,
                                            pad_ref, pads_left, pointer_pad, presses, settled, since)

SHOW_SETTLE_S = 3.0
HOLD_MS = 8_000
REFUSAL_KINDS = ("cursor_hide_timeout", "cursor_hide_failed")
KEEP_DRAWN_FLAG = "keep-drawn-for-test"


def registry_of(runtime: Path, session: str) -> Path:
    """The registry directory that holds the session's renderer record."""
    base = runtime / "agent-desktop" / "cursor"
    for path in sorted(base.glob("*/*.json")) if base.is_dir() else ():
        try:
            if json.loads(path.read_text()).get("session_id") == session:
                return path.parent
        except (OSError, ValueError):
            continue
    raise AssertionError(f"session {session} has no renderer record under {base}")


def holders_in(registry: Path) -> list[dict]:
    """Every holder file in the registry, read as files."""
    found = []
    for path in sorted((registry / "holders").glob("*.json")):
        try:
            found.append(json.loads(path.read_text()))
        except (OSError, ValueError):
            continue
    return found


def cursor_layer_over(layers: dict, box: dict) -> bool:
    """Whether a cursor layer on the overlay level overlaps a client box."""
    (x, y), (width, height) = box["at"], box["size"]
    for monitor in (layers or {}).values():
        for entries in (monitor.get("levels") or {}).values():
            for entry in entries:
                if entry.get("namespace") != NAMESPACE:
                    continue
                if entry["x"] < x + width and x < entry["x"] + entry["w"] and \
                        entry["y"] < y + height and y < entry["y"] + entry["h"]:
                    return True
    return False


def dpms_argv(state: str, lua: bool) -> list[str]:
    if lua:
        return ["dispatch", lua_call("hl.dsp.dpms", {"action": state})]
    return ["dispatch", "dpms", state]


def undo_all(tree, before: str, steps: int = 4) -> str:
    """One Undo per digit the leg entered, until the display reads `before` again."""
    for _ in range(steps):
        current = display(tree)
        if current == before:
            break
        activate(one(tree, role=43, name="Undo"))
        raw.poll(lambda: display(tree) != current, SHOW_SETTLE_S)
    return changed(lambda: display(tree), before)


def holder_body(pid: int, started_at: int, nonce: str, expiry_ms: int) -> dict:
    return {"pid": pid, "start_time": started_at, "nonce": nonce, "expiry": expiry_ms}


@contextlib.contextmanager
def keep_drawn(registry: Path):
    """The renderer's test flag that keeps the cursor drawn through a hide, removed on exit."""
    flag = registry / KEEP_DRAWN_FLAG
    descriptor = os.open(flag, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    os.close(descriptor)
    try:
        yield flag
    finally:
        flag.unlink(missing_ok=True)


@contextlib.contextmanager
def harness_holder(registry: Path, hold_ms: int = HOLD_MS):
    """A holder this process writes itself, as a second caller would, and removes on exit."""
    holders = registry / "holders"
    holders.mkdir(mode=0o700, exist_ok=True)
    nonce = secrets.token_hex(16)
    pid = os.getpid()
    body = holder_body(pid, start_time(pid), nonce, int(time.time() * 1000) + hold_ms)
    staging, path = holders / f".{nonce}.tmp", holders / f"{nonce}.json"
    descriptor = os.open(staging, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w") as handle:
        handle.write(json.dumps(body))
    os.replace(staging, path)
    try:
        yield body
    finally:
        path.unlink(missing_ok=True)


def hide_refusal_problems(label: str, envelope, kinds: tuple[str, ...] = REFUSAL_KINDS) -> list[str]:
    """A hide refusal: one of `kinds`, not delivered, safe to retry, no physical delivery."""
    error = envelope.error or {}
    details = error.get("details") or {}
    disposition = error.get("disposition") or {}
    problems = []
    if envelope.ok or details.get("kind") not in kinds:
        problems.append(f"{label}: expected a {'/'.join(kinds)} refusal, got {envelope.raw[:300]}")
    if disposition.get("delivery") != "not_delivered" or disposition.get("retry") != "safe":
        problems.append(f"{label}: disposition {disposition} is not not_delivered with safe retry")
    if details.get("physical_delivery_started") not in (None, False):
        problems.append(f"{label}: physical delivery started")
    return problems


class Screen:
    """`grim` of the monitor that shows `point`, read at the cursor's arrow box."""

    def __init__(self, ctx: LegContext, main: dict, point: tuple[float, float]):
        self.ctx, self.point = ctx, point
        monitors = hyprland.query(shared(ctx.tier), main["signature"], "monitors", pid=main["pid"])
        self.monitor = monitor_at(monitors, point)
        listed = ctx.product(["list-displays"], headed=False)
        require(listed.ok and isinstance(listed.data, list), f"list-displays failed: {listed.raw[:400]}")
        ids = [item.get("id") for item in listed.data]
        require(self.monitor["name"] in ids, f"list-displays does not list {self.monitor['name']}: {ids}")
        self.index = ids.index(self.monitor["name"])

    def grab(self) -> pixels.Image:
        return grim(self.ctx.tier, self.monitor["name"])

    def box(self, image: pixels.Image) -> tuple[int, int, int, int]:
        origin = (self.monitor["x"], self.monitor["y"])
        return arrow_box(origin, self.point, self.monitor["scale"], (image.width, image.height))

    def same(self, left: pixels.Image, right: pixels.Image) -> bool:
        return pixels.similar(left, right, self.box(left), tolerance=TOLERANCE)

    def product_capture(self, label: str):
        path = capture_path(self.ctx.tier, label)
        envelope = self.ctx.product(["screenshot", "--screen", str(self.index), str(path)], headed=False)
        return envelope, path


def _unlike(screen: Screen, reference: pixels.Image) -> pixels.Image | None:
    """The next grab that differs from `reference` at the arrow box, within the settle time."""
    def grabbed():
        image = screen.grab()
        return None if screen.same(image, reference) else image
    return raw.poll(grabbed, SHOW_SETTLE_S)


def _power_cycle(ctx: LegContext, main: dict, screen: Screen) -> None:
    """Turns every output off, then on, and waits until the monitor reports power again."""
    runtime = shared(ctx.tier)
    lua = hyprland.uses_lua(runtime, main["signature"])
    for state in ("off", "on"):
        output = hyprland.control(runtime, main["signature"], dpms_argv(state, lua))
        require(output == "ok", f"the compositor refused dpms {state}: {output}")
    name = screen.monitor["name"]
    powered = lambda: any(item.get("name") == name and item.get("dpmsStatus") is True
                          for item in hyprland.query(runtime, main["signature"], "monitors", pid=main["pid"]))
    require(raw.poll(powered, SHOW_SETTLE_S), f"{name} did not report power after dpms on")


@implementation("cursor.hide_ack_suppression_until_show")
def hide_ack_suppression_until_show(ctx: LegContext) -> None:
    """A screen capture never shows the cursor, and a live holder keeps it hidden until it leaves.

    The cursor-free reference is read after `disable`, so the click's own
    effect on the app is in every image and only the cursor differs.
    """
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        ctx.before("hyprland.layers", layers_of(runtime, main))
        session = started(ctx)
        try:
            owner = renderer_of(runtime, main, session)
            registry = registry_of(runtime, session)
            ctx.before("proc.holders", len(holders_in(registry)))
            seven, point = _seven(ctx, app, session)
            screen = Screen(ctx, main, point)
            placed = ctx.product(["--session", session, "click", seven], headed=False)
            require(placed.ok, f"the click that places the cursor failed: {placed.raw[:400]}")
            covered = lambda: NAMESPACE in covering_layers(
                hyprland.query(runtime, main["signature"], "layers", pid=main["pid"]), point)
            require(raw.poll(covered, SHOW_SETTLE_S), f"no cursor layer covers {point}: {layers_of(runtime, main)}")
            drawn = screen.grab()
            ctx.after("hyprland.layers", layers_of(runtime, main))
            captures = [screen.product_capture("hide-first")]
            after_capture = screen.grab()
            with harness_holder(registry):
                held = _unlike(screen, drawn)
                again = ctx.product(["--session", session, "click", seven], headed=False)
                require(again.ok, f"a click during the hold failed: {again.raw[:400]}")
                captures.append(screen.product_capture("hide-under-hold"))
                still = screen.grab()
                ctx.after("proc.holders", len(holders_in(registry)))
            released = _unlike(screen, still)
            _power_cycle(ctx, main, screen)
            captures.append(screen.product_capture("hide-after-dpms"))
            redrawn = _unlike(screen, still)
            ctx.after("atspi.display", display(tree))
            disabled = ctx.product(["--session", session, "cursor-overlay", "disable"])
            require(disabled.ok, f"disable failed: {disabled.raw[:400]}")
            require(raw.poll(lambda: not alive(owner), SHOW_SETTLE_S), "the renderer survived disable")
            bare = screen.grab()
            problems = []
            if screen.same(drawn, bare):
                problems.append(f"the screen shows no cursor at {point}: the positive control failed")
            if not screen.same(after_capture, drawn):
                problems.append("the cursor did not come back after the screen capture")
            if held is None or not screen.same(held, bare) or not screen.same(still, bare):
                problems.append("the cursor stayed drawn, or was drawn again, while a holder was live")
            if released is None or redrawn is None:
                problems.append("the cursor did not come back once the holder left or the outputs powered on")
            for label, (envelope, path) in zip(("first", "under hold", "after dpms"), captures):
                if not envelope.ok:
                    problems.append(f"screen capture {label} failed: {envelope.raw[:300]}")
                elif not screen.same(saved_image(envelope, path), bare):
                    problems.append(f"screen capture {label} shows the cursor at {point}")
            if holders_in(registry):
                problems.append(f"holders survived their commands: {holders_in(registry)}")
            ctx.restored("proc.holders", len(holders_in(registry)))
        finally:
            ended(ctx, session)
        ctx.restored("atspi.display", undo_all(tree, before))
        ctx.restored("hyprland.layers", layers_of(runtime, main))
        require(not problems, "; ".join(problems))


def _stopped(owner: dict):
    """The renderer stopped with SIGSTOP, so it answers nothing, and resumed on exit."""
    descriptor = os.pidfd_open(owner["pid"])

    @contextlib.contextmanager
    def held():
        try:
            require(alive(owner), f"renderer {owner['pid']} is not the recorded process")
            signal.pidfd_send_signal(descriptor, signal.SIGSTOP)
            yield
        finally:
            signal.pidfd_send_signal(descriptor, signal.SIGCONT)
            os.close(descriptor)

    return held()


@implementation("cursor.travel_failure_proceeds_hide_failure_refuses")
def travel_failure_proceeds_hide_failure_refuses(ctx: LegContext) -> None:
    """A renderer that answers nothing refuses headed work before any event, and never blocks a headless click.

    A stopped renderer withholds both its arrival and its `hide`
    acknowledgement. A killed renderer counts as hidden, so it blocks neither.
    """
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    headed = ctx.mode == "headed"
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        session = started(ctx)
        try:
            owner = renderer_of(runtime, main, session)
            registry = registry_of(runtime, session)
            seven, point = _seven(ctx, app, session)
            screen = Screen(ctx, main, point)
            shell = shell_covers(hyprland.query(runtime, main["signature"], "layers", pid=main["pid"]), point)
            problems = []
            with _stopped(owner):
                if headed:
                    envelope, path = screen.product_capture("hide-withheld")
                    problems += hide_refusal_problems("screen capture under a stopped renderer", envelope)
                    if path.exists():
                        problems.append("a refused screen capture wrote its file")
                    clicked = ctx.product(["--session", session, "click", seven])
                    problems += hide_refusal_problems("headed click under a stopped renderer", clicked)
                    if display(tree) != before:
                        problems.append("the refused headed click changed the display")
                else:
                    clicked = ctx.product(["--session", session, "click", seven])
                    if not clicked.ok:
                        problems.append(f"a withheld arrival blocked the headless click: {clicked.raw[:300]}")
            if headed and raw.poll(lambda: display(tree) != before, 1.0):
                problems.append("the refused headed click changed the display late")
            if holders_in(registry):
                problems.append(f"a refused hide left holders: {holders_in(registry)}")
            if headed:
                problems += _screen_after_resume(screen)
            else:
                after = raw.poll(lambda: display(tree) if display(tree) != before else None, SHOW_SETTLE_S)
                if not after:
                    problems.append("the headless click under a stopped renderer had no effect")
            kill_exactly(renderer_of(runtime, main, session))
            shown = display(tree)
            dead = ctx.product(["--session", session, "click", seven])
            problems += _dead_click_problems(ctx, dead, shell if headed else [], tree, shown)
            if headed:
                envelope, path = screen.product_capture("hide-dead")
                if not envelope.ok:
                    problems.append(f"a dead renderer blocked the screen capture: {envelope.raw[:300]}")
            ctx.after("atspi.display", display(tree))
        finally:
            ended(ctx, session)
        ctx.restored("atspi.display", undo_all(tree, before))
        require(not problems, "; ".join(problems))


def shell_covers(layers: dict, point: tuple[float, float]) -> list[str]:
    """The top and overlay layers over `point` that no agent cursor draws."""
    return [name for name in covering_layers(layers, point) if name != NAMESPACE]


def _dead_click_problems(ctx: LegContext, envelope, shell: list[str], tree, shown: str) -> list[str]:
    """A click under a killed renderer passes its hide; only a shell layer read before the click, or the
    native Wayland refusal, may stop it.

    The hit test judges covers after a confirmed hide, so a cover by a layer
    that was over the button before any headed click shows the hide passed.
    Calculator is a native Wayland client, and its headed ref click refuses with
    `native_wayland_client` only after that hit test, so that refusal shows the
    hide passed too. Either refusal must be undelivered and safe, and the
    display must still read `shown`, its value before the click.
    """
    if envelope.ok:
        return []
    error = envelope.error or {}
    details = error.get("details") or {}
    covers = [(check.get("occluder") or {}).get("name") for check in details.get("checks") or []
              if check.get("check") == "receives_events" and check.get("status") == "fail"]
    disposition = error.get("disposition") or {}
    shell_covered = bool(covers) and all(name in shell for name in covers)
    native = details.get("cause") == "native_wayland_client" and details.get("physical_delivery_started") is False
    if error.get("code") == "ACTION_FAILED" and (shell_covered or native):
        if disposition.get("delivery") != "not_delivered" or disposition.get("retry") != "safe":
            return [f"the refused click under a dead renderer is {disposition}, not not_delivered and safe"]
        if raw.poll(lambda: display(tree) != shown, 1.0):
            return [f"the refused click under a dead renderer changed the display to {display(tree)!r}"]
        met = f"the shell layer {covers}, which covered the button before any headed click" if shell_covered \
            else "the native Wayland refusal, which runs after the hit test"
        ctx.finding(f"the headed click under a dead renderer passed its hide and met {met}")
        return []
    return [f"a dead renderer blocked the click: {envelope.raw[:300]}"]


def _screen_after_resume(screen: Screen) -> list[str]:
    envelope, path = screen.product_capture("hide-resumed")
    if envelope.ok:
        return []
    path.unlink(missing_ok=True)
    return [f"a resumed renderer still blocks the screen capture: {envelope.raw[:300]}"]


def _click_under_cursor(ctx: LegContext, label: str, kept: bool) -> None:
    """A headed ref click on the pad while the session cursor layer covers it lands exactly once."""
    tier, runtime = ctx.tier, shared(ctx.tier)
    instance, _ = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    ctx.before("hyprland.layers", layers_of(runtime, instance))
    with pointer_pad(tier, instance, label, XWAYLAND) as (owned, client, log):
        session = started(ctx, env)
        try:
            renderer_of(runtime, instance, session)
            registry = registry_of(runtime, session)
            bound = dict(env, AGENT_DESKTOP_SESSION=session)
            button = pad_ref(ctx, bound, owned)
            placed = ctx.product(["click", button], env=bound, headed=False)
            require(placed.ok, f"the click that places the cursor failed: {placed.raw[:400]}")
            client = pad_client(tier, instance, owned.pid) or client
            over = lambda: cursor_layer_over(hyprland.query(runtime, instance["signature"], "layers",
                                                            pid=instance["pid"]), client)
            require(raw.poll(over, SHOW_SETTLE_S), f"no cursor layer is over the pad: {layers_of(runtime, instance)}")
            start = len(events(log))
            with keep_drawn(registry) if kept else contextlib.nullcontext():
                clicked = ctx.product(["click", button], env=bound, headed=True)
                drawn_during = over()
            found = since(settled(log, lambda found: presses(found, "button") != [] and len(found) > start), start)
            problems = []
            if not clicked.ok:
                problems.append(f"the click under the cursor refused: {clicked.raw[:400]}")
            if presses(found, "button") != [1]:
                problems.append(f"the pad logged {presses(found, 'button')} under the cursor, not one press")
            if kept and not drawn_during:
                problems.append("the kept cursor layer was gone right after the click")
            if not raw.poll(over, SHOW_SETTLE_S):
                problems.append("the cursor layer did not come back after the click")
            ctx.after("hyprland.layers", layers_of(runtime, instance))
            ctx.after("applog.pad", len(events(log)))
        finally:
            ended(ctx, session, env)
        require(record_of(runtime, session) is None, "the session's renderer record survived session end")
        ctx.restored("hyprland.layers", layers_of(runtime, instance))
        require(not problems, "; ".join(problems))
    ctx.restored("applog.pad", pads_left(tier))


@implementation("pointer.ref_receiver_evidence_under_cursor")
def ref_receiver_evidence_under_cursor(ctx: LegContext) -> None:
    """The headed click hides the session cursor, accepts its acknowledged layer as its own and lands once."""
    _click_under_cursor(ctx, "under-cursor", kept=False)


@implementation("pointer.cursor_pass_through")
def cursor_pass_through(ctx: LegContext) -> None:
    """With the renderer's test flag keeping the cursor drawn through the hide, the click still lands once."""
    _click_under_cursor(ctx, "pass-through", kept=True)
