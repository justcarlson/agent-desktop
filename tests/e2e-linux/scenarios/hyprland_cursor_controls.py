"""Cursor hide controls on the attached `hyprland` tier: a late acknowledgement, and a hold across agents.

A renderer stopped with SIGSTOP cannot acknowledge a `hide`, so a screen
capture started then must wait. Resumed 200 ms later, inside the hide
deadline, it acknowledges, and the capture succeeds with the image written
only after the resume and no cursor in it. In a multi-agent session, a holder
the harness writes keeps every agent's cursor hidden: another agent's click
and capture under the hold draw nothing, and both cursors draw once the
holder leaves. Every leg reads `grim` pixels, the compositor's `j/layers`,
the holder files, `/proc` and the Calculator display through AT-SPI.
"""

from __future__ import annotations

import os
import signal
import threading
import time
from pathlib import Path

from oracle import atspi_raw as raw, hyprland
from runner.helper import display, managed, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_common import capture_path, saved_image
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import (NAMESPACE, _seven, alive, app_shown, ended, layers_of, record_of,
                                       renderer_of, started)
from scenarios.hyprland_cursor_agents import (button, gone_problems, placed, started_multi, surfaces_of,
                                              undone)
from scenarios.hyprland_cursor_hide import (SHOW_SETTLE_S, Screen, _grab_until, harness_holder, holders_in,
                                            registry_of, undo_all)
from scenarios.hyprland_pointer_pad import covering_layers

ACK_DELAY_MS = 200
STOP_WAIT_S = 1.0
PENDING_WAIT_S = 2.0
CAPTURE_JOIN_S = 30.0
CROSS_HOLD_MS = 20_000
HOLD_MARGIN_MS = 1_000
HELD_GRABS = 3
DELAYED_KEYS = {
    "stopped": (True, "the renderer was not in the stopped state"),
    "pending": (True, "no holder was written while the renderer was stopped, so no hide was pending"),
    "running_at_resume": (True, "the capture answered before the renderer resumed"),
    "file_at_resume": (False, "the image existed before the renderer resumed"),
    "written_after": (True, "the image was not written after the renderer resumed"),
    "shows": (False, "the image shows the cursor, or could not be read"),
}


def delayed_ack_problems(envelope, observed: dict) -> list[str]:
    """A screen capture that waited for a hide acknowledged `ACK_DELAY_MS` late, and then succeeded."""
    problems = [message for key, (good, message) in DELAYED_KEYS.items() if observed.get(key) is not good]
    if not envelope.ok:
        problems.append(f"the capture under a late acknowledgement failed: {envelope.raw[:400]}")
    elif envelope.elapsed_ms < ACK_DELAY_MS:
        problems.append(f"the capture answered in {envelope.elapsed_ms:.0f} ms, before the {ACK_DELAY_MS} ms "
                        "acknowledgement delay, so it did not wait for the hide")
    return problems


def held_problems(grabs: list[tuple[str, bool, bool]]) -> list[str]:
    """No grab taken under a live holder shows either agent's cursor."""
    if not grabs:
        return ["no grab was taken under the hold"]
    return [f"{label}: grim shows {' and '.join(name for name, shown in (('agent-a', a), ('agent-b', b)) if shown)}"
            f"'s cursor under a live holder" for label, a, b in grabs if a or b]


def hold_outlasts(expiry_ms: int, now_ms: int) -> bool:
    """Whether the holder stays live at least `HOLD_MARGIN_MS` past `now_ms`."""
    return expiry_ms - now_ms >= HOLD_MARGIN_MS


def _state(pid: int) -> str:
    try:
        text = (Path("/proc") / str(pid) / "stat").read_text()
    except OSError:
        return ""
    return text[text.rindex(")") + 2:].split(" ", 1)[0]


def _delayed_capture(ctx: LegContext, screen: Screen, owner: dict, registry: Path):
    """`screenshot --screen` started under a stopped renderer, which resumes `ACK_DELAY_MS` after the hide."""
    path = capture_path(ctx.tier, "hide-delayed-ack")
    marker = path.with_name(f"{path.stem}.resumed")
    path.unlink(missing_ok=True)
    answer: dict = {}
    argv = ["screenshot", "--screen", str(screen.index), str(path)]
    thread = threading.Thread(target=lambda: answer.update(envelope=ctx.product(argv, headed=False)), daemon=True)
    observed: dict = {}
    descriptor = os.pidfd_open(owner["pid"])
    try:
        require(alive(owner), f"renderer {owner['pid']} is not the recorded process")
        signal.pidfd_send_signal(descriptor, signal.SIGSTOP)
        observed["stopped"] = bool(raw.poll(lambda: _state(owner["pid"]) == "T", STOP_WAIT_S))
        thread.start()
        observed["pending"] = bool(raw.poll(lambda: holders_in(registry), PENDING_WAIT_S)) and thread.is_alive()
        time.sleep(ACK_DELAY_MS / 1000)
        observed["running_at_resume"] = thread.is_alive()
        observed["file_at_resume"] = path.exists()
        marker.write_bytes(b"")
    finally:
        signal.pidfd_send_signal(descriptor, signal.SIGCONT)
        os.close(descriptor)
    thread.join(CAPTURE_JOIN_S)
    require(not thread.is_alive(), f"the capture did not answer within {CAPTURE_JOIN_S} s of the resume")
    envelope = answer["envelope"]
    resumed = marker.stat().st_mtime_ns
    marker.unlink(missing_ok=True)
    observed["written_after"] = path.exists() and path.stat().st_mtime_ns >= resumed
    observed["shows"] = screen.shows(saved_image(envelope, path)) if envelope.ok and path.exists() else None
    path.unlink(missing_ok=True)
    ctx.finding(f"the capture under a renderer stopped for {ACK_DELAY_MS} ms after its hide answered in "
                f"{envelope.elapsed_ms:.0f} ms: {observed}")
    return envelope, observed


@implementation("cursor.hide_ack_delayed_within_deadline")
def hide_ack_delayed_within_deadline(ctx: LegContext) -> None:
    """A hide acknowledged 200 ms late, inside the deadline, delays the capture and does not refuse it."""
    runtime, main = shared(ctx.tier), main_instance(ctx.tier)
    with managed(ctx, "Calculator") as app, app_shown(ctx.tier, app.pid):
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
            clicked = ctx.product(["--session", session, "click", seven], headed=False)
            require(clicked.ok, f"the click that places the cursor failed: {clicked.raw[:400]}")
            covered = lambda: NAMESPACE in covering_layers(
                hyprland.query(runtime, main["signature"], "layers", pid=main["pid"]), point)
            require(raw.poll(covered, SHOW_SETTLE_S), f"no cursor layer covers {point}: {layers_of(runtime, main)}")
            require(_grab_until(screen, True), f"the screen shows no cursor at {point}: the positive control failed")
            ctx.after("hyprland.layers", layers_of(runtime, main))
            envelope, observed = _delayed_capture(ctx, screen, owner, registry)
            problems = delayed_ack_problems(envelope, observed)
            if not alive(owner):
                problems.append(f"renderer {owner['pid']} did not survive the stop and resume")
            if _grab_until(screen, True) is None:
                problems.append("the cursor did not come back after the delayed capture")
            ctx.after("atspi.display", display(tree))
            ctx.after("proc.holders", len(holders_in(registry)))
            if holders_in(registry):
                problems.append(f"holders survived the capture: {holders_in(registry)}")
            ctx.restored("proc.holders", len(holders_in(registry)))
        finally:
            ended(ctx, session)
        ctx.restored("atspi.display", undo_all(tree, before))
        ctx.restored("hyprland.layers", layers_of(runtime, main))
        require(not problems, "; ".join(problems))


def _agent_click(ctx: LegContext, session: str, ref: str, agent: str):
    clicked = ctx.product(["--session", session, "--agent-id", agent, "click", ref], headed=False)
    require(clicked.ok, f"{agent}'s click failed: {clicked.raw[:400]}")
    return clicked


def _shows(screens: dict[str, Screen]) -> dict[str, bool]:
    """Whether each agent's arrow box shows its cursor, one grab per monitor."""
    images = {}
    for screen in screens.values():
        images.setdefault(screen.monitor["name"], screen.grab())
    return {agent: screen.shows(images[screen.monitor["name"]]) for agent, screen in screens.items()}


def _under_hold(ctx: LegContext, session: str, screens: dict[str, Screen], eight: str, body: dict):
    """agent-b clicks and captures while the harness holder is live; every grab is judged."""
    require(raw.poll(lambda: not any(_shows(screens).values()), SHOW_SETTLE_S),
            "agent-a's cursor stayed drawn once the harness holder was written")
    grabs = []
    _agent_click(ctx, session, eight, "agent-b")
    seen = _shows(screens)
    grabs.append(("after agent-b's click", seen["agent-a"], seen["agent-b"]))
    path = capture_path(ctx.tier, "hold-across-agents")
    envelope = ctx.product(["--session", session, "--agent-id", "agent-b", "screenshot", "--screen",
                            str(screens["agent-b"].index), str(path)], headed=False)
    problems = []
    if envelope.ok:
        image = saved_image(envelope, path)
        grabs.append(("agent-b's capture", screens["agent-a"].shows(image), screens["agent-b"].shows(image)))
    else:
        path.unlink(missing_ok=True)
        problems.append(f"agent-b's capture under the hold failed: {envelope.raw[:400]}")
    for index in range(HELD_GRABS):
        seen = _shows(screens)
        grabs.append((f"grab {index + 1} after agent-b's capture", seen["agent-a"], seen["agent-b"]))
    if not hold_outlasts(body["expiry"], int(time.time() * 1000)):
        problems.append("the harness holder was about to expire before the last grab under it")
    return problems + held_problems(grabs)


@implementation("cursor.hide_hold_across_agents")
def hide_hold_across_agents(ctx: LegContext) -> None:
    """A live holder keeps every agent's cursor hidden through another agent's click and capture."""
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
            seven, at_seven = button(ctx, app, session, "7")
            eight, at_eight = button(ctx, app, session, "8")
            _agent_click(ctx, session, seven, "agent-a")
            require(raw.poll(lambda: placed(surfaces_of(runtime, main), [at_seven], 1), SHOW_SETTLE_S),
                    f"agent-a's cursor is not at {at_seven}: {surfaces_of(runtime, main)}")
            owner = record_of(runtime, session)
            require(owner, f"session {session} has no renderer record")
            registry = registry_of(runtime, session)
            ctx.before("proc.holders", len(holders_in(registry)))
            screens = {"agent-a": Screen(ctx, main, at_seven), "agent-b": Screen(ctx, main, at_eight)}
            require(raw.poll(lambda: _shows(screens)["agent-a"], SHOW_SETTLE_S),
                    f"grim shows no cursor at {at_seven}: the positive control failed")
            clicked_a = display(tree)
            with harness_holder(registry, CROSS_HOLD_MS) as body:
                problems = _under_hold(ctx, session, screens, eight, body)
                ctx.after("proc.holders", len(holders_in(registry)))
            both = raw.poll(lambda: all(_shows(screens).values()), SHOW_SETTLE_S)
            if not both:
                problems.append(f"both cursors did not draw once the holder left: {_shows(screens)}")
            if not raw.poll(lambda: placed(surfaces_of(runtime, main), [at_seven, at_eight], 2), SHOW_SETTLE_S):
                problems.append(f"expected agent-a's cursor at {at_seven} and agent-b's at {at_eight}: "
                                f"{surfaces_of(runtime, main)}")
            if not raw.poll(lambda: display(tree) != clicked_a, SHOW_SETTLE_S):
                problems.append(f"agent-b's click under the hold did not change the display from {clicked_a!r}")
            ctx.after("hyprland.layers", surfaces_of(runtime, main))
            ctx.after("atspi.display", display(tree))
            if holders_in(registry):
                problems.append(f"holders survived their commands: {holders_in(registry)}")
            ctx.restored("proc.holders", len(holders_in(registry)))
        finally:
            closing = ended(ctx, session)
        if owner:
            problems += gone_problems("session end", runtime, main, session, owner)
        if not closing.ok:
            problems.append(f"the session did not end: {closing.raw[:400]}")
        ctx.restored("atspi.display", undone(tree, before))
    ctx.restored("hyprland.layers", surfaces_of(runtime, main))
    require(not problems, "; ".join(problems))
