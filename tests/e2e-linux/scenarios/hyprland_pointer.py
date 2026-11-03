"""Coordinate pointer legs on the attached `hyprland` tier.

Coordinate click, wheel and drag go to the session instance, into a pointer pad the leg owns and
shows on the focused monitor beside the bystander. A sampler reads the bystander's `j/clients` box
and keeps the cursor on the pad while the leg acts, so no event can reach the bystander. The nested
second instance keeps the cases the session cannot give: an output moved to a negative origin and a
fractional scale, and a compositor killed during a drag. The pad's log says what arrived and where;
`j/cursorpos`, `j/clients` and `j/monitors` say where the compositor put it.
"""

from __future__ import annotations

import contextlib
import json
import os
import signal
import subprocess
from pathlib import Path

from oracle import atspi_raw as raw
from oracle import hyprland
from runner import product, second_instance
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, main_instance, shared
from scenarios.hyprland_cursor import app_shown
from scenarios.hyprland_focus_controls import cursor_argv
from scenarios.hyprland_pointer_pad import (PAD_SCROLL, apart_from_last_click, cursor_at, events,
                                            gdk3_presses, global_point, holds, instance_control, instance_env,
                                            instance_query, lands_at,
                                            motions_after, pad_client, pad_ref, pad_saw, pads_left, pointer_pad, presses,
                                            releases,
                                            scrolls, settled, since, window_point, xy)
from scenarios.wayland_outputs import monitor_command, settle_outputs

NEGATIVE_ORIGIN = (-1024, -640)
FRACTIONAL = 1.25
CANCEL_DRAG_MS = 3000
LATE_EVENT_S = 1.0
OFF_OUTPUT = "-5000,-5000"
BYSTANDER_KEYS = ("address", "pid", "at", "size", "workspace", "floating", "fullscreen", "pinned", "mapped", "hidden")
REFUSED = {"mouse-click": ["mouse-click", f"--xy={OFF_OUTPUT}"],
           "mouse-wheel": ["mouse-wheel", "--x=-5000", "--y=-5000", "--dy=1"],
           "drag": ["drag", f"--from-xy={OFF_OUTPUT}", "--to-xy=-4990,-4990"]}


def nested(tier) -> tuple[dict, bool]:
    """The nested instance with its outputs settled, and whether it takes Lua."""
    instance = second_instance.ensure(tier)
    lua = hyprland.uses_lua(shared(tier), instance["signature"])
    settle_outputs(shared(tier), instance["signature"], lua)
    return instance, lua


def nested_running(instance: dict) -> bool:
    """Whether the recorded nested instance still runs as the same process."""
    return second_instance._start_of(instance["pid"], Path("/proc")) == instance["start"]


def monitor_of(tier, instance: dict, client: dict) -> dict:
    found = [item for item in instance_query(tier, instance, "monitors") if item["id"] == client["monitor"]]
    require(len(found) == 1, f"the pad's monitor {client['monitor']} is not listed once")
    return found[0]


def layout(tier, instance: dict) -> list[tuple]:
    return sorted((item["name"], item["x"], item["y"], round(float(item["scale"]), 3))
                  for item in instance_query(tier, instance, "monitors"))


@contextlib.contextmanager
def moved_output(tier, instance: dict, lua: bool, name: str, origin: tuple[int, int], scale: float):
    """One output of `instance` at `origin` and `scale`, put back afterwards."""
    before = [item for item in instance_query(tier, instance, "monitors") if item["name"] == name][0]

    def placed(at, at_scale):
        return any(item["name"] == name and (item["x"], item["y"]) == tuple(at)
                   and abs(float(item["scale"]) - at_scale) < 0.01
                   for item in instance_query(tier, instance, "monitors"))

    command = monitor_command(lua, name, f"{origin[0]}x{origin[1]}", scale)
    reply = hyprland.control(shared(tier), instance["signature"], command)
    try:
        require(raw.poll(lambda: placed(origin, scale), 5.0),
                f"{name} never took {origin} at {scale:g}; the {'Lua' if lua else 'legacy'} config replied "
                f"{reply!r} to {' '.join(command)}; outputs {outputs_seen(tier, instance)}")
        yield
    finally:
        back = (before["x"], before["y"])
        hyprland.control(shared(tier), instance["signature"],
                         monitor_command(lua, name, f"{back[0]}x{back[1]}", float(before["scale"])))
        require(raw.poll(lambda: placed(back, float(before["scale"])), 5.0),
                f"{name} did not return to {back}; outputs {outputs_seen(tier, instance)}")


def outputs_seen(tier, instance: dict) -> str:
    """Each output as name, origin, scale and mode size."""
    return "; ".join(f"{item['name']} {item['x']},{item['y']} scale {float(item['scale']):g} "
                     f"{item['width']}x{item['height']}" for item in instance_query(tier, instance, "monitors"))


def settled_client(tier, instance: dict, pid: int, monitor: dict) -> dict:
    """The pad's toplevel once it lies inside `monitor` again."""
    def inside():
        found = pad_client(tier, instance, pid)
        if not found:
            return None
        (x, y), (width, height) = found["at"], found["size"]
        right, bottom = monitor["x"] + monitor["width"] / monitor["scale"], monitor["y"] + monitor["height"] / monitor["scale"]
        return found if monitor["x"] <= x and x + width <= right + 1 and monitor["y"] <= y and y + height <= bottom + 1 else None
    found = raw.poll(inside, 5.0)
    require(found, f"the pad did not settle inside {monitor['name']}")
    return found


def delivered(envelope) -> dict:
    require(envelope.ok, f"{envelope.argv[1:]} failed: {envelope.raw[:400]}")
    return envelope.data or {}


def no_receiver_proof(label: str, data: dict) -> list[str]:
    details = data.get("details") or {}
    problems = []
    if details.get("receiver_proof") is not False:
        problems.append(f"{label}: receiver_proof is {details.get('receiver_proof')!r}, not false")
    if not isinstance(details.get("delivered_events"), int) or details["delivered_events"] < 1:
        problems.append(f"{label}: delivered_events is {details.get('delivered_events')!r}")
    return problems


def click_problems(ctx, env, log: Path, client: dict, point, count: int) -> list[str]:
    apart_from_last_click(log)
    start = len(events(log))
    data = delivered(ctx.product(["mouse-click", f"--xy={xy(point)}", "--count", str(count)], env=env))
    want = gdk3_presses(count)
    found = since(settled(log, lambda all_: presses(since(all_, start), "area") == want), start)
    problems = no_receiver_proof(f"mouse-click --count {count}", data)
    if presses(found, "area") != want:
        problems.append(f"mouse-click --count {count} at {point}: the pad logged {presses(found, 'area')}"
                        f"{missed_click(found)}")
    elif not all(lands_at(item, window_point(point, client)) for item in found if item["ev"] == "press"):
        problems.append(f"mouse-click at {point} landed at {[(i['wx'], i['wy']) for i in found]}")
    return problems


def missed_click(found: list[dict]) -> str:
    """What the pad saw instead of the expected presses, so a lost click shows whether the pointer
    reached the window at all."""
    elsewhere = [(item["widget"], item.get("n")) for item in found
                 if item.get("ev") == "press" and item.get("widget") != "area"]
    motions = sum(1 for item in found if item.get("ev") == "motion")
    return f" (presses elsewhere: {elsewhere}, area motions: {motions})"


def held_still(log: Path, start: int, count) -> list[dict]:
    """The pad's events after `start` once `count` of them stayed the same for `LATE_EVENT_S`, so a late
    duplicate is counted, not missed."""
    first = count(since(events(log), start))
    return since(settled(log, lambda all_: count(since(all_, start)) != first, LATE_EVENT_S), start)


def wheel_problems(ctx, env, log: Path, point, deltas: list[str], want: list[str]) -> list[str]:
    start = len(events(log))
    data = delivered(ctx.product(["mouse-wheel", f"--x={int(point[0])}", f"--y={int(point[1])}", *deltas], env=env))
    settled(log, lambda all_: scrolls(since(all_, start), "area") == want)
    found = held_still(log, start, lambda found: scrolls(found, "area"))
    problems = no_receiver_proof(f"mouse-wheel {deltas}", data)
    if scrolls(found, "area") != want:
        problems.append(f"mouse-wheel {deltas}: the pad logged {scrolls(found, 'area')}, expected {want}")
    return problems


def drag_problems(ctx, env, log: Path, client: dict, start_at, end_at) -> list[str]:
    apart_from_last_click(log)
    start = len(events(log))
    data = delivered(ctx.product(["drag", f"--from-xy={xy(start_at)}", f"--to-xy={xy(end_at)}",
                                  "--duration", "300"], env=env))
    settled(log, lambda all_: bool(releases(since(all_, start), "area")))
    found = held_still(log, start, lambda found: (presses(found, "area"), len(releases(found, "area"))))
    problems = no_receiver_proof("drag", data)
    pressed, released = presses(found, "area"), releases(found, "area")
    if pressed != [1] or len(released) != 1:
        problems.append(f"drag: the pad logged presses {pressed} and {len(released)} releases{pad_saw(found)}")
    elif not lands_at(released[0], window_point(end_at, client)):
        problems.append(f"drag released at {(released[0]['wx'], released[0]['wy'])}, not {window_point(end_at, client)}")
    return problems


def bystander_box(tier, main: dict) -> dict | None:
    """The bystander's `j/clients` placement on the session instance; None unless listed once."""
    address = bystander(tier)["address"]
    found = [item for item in instance_query(tier, main, "clients") if item.get("address") == address]
    return {key: found[0].get(key) for key in BYSTANDER_KEYS} if len(found) == 1 else None


def session_client(tier, main: dict, pid: int) -> dict:
    """The shown pad's toplevel once two reads in a row agree on its place."""
    last: dict = {}

    def still():
        now = pad_client(tier, main, pid)
        place = now and (now["at"], now["size"], now["monitor"])
        same = place is not None and last.get("place") == place
        last["place"] = place
        return now if same else None
    found = raw.poll(still, 5.0)
    require(found, f"the shown pad never held still on the session instance: {last.get('place')}")
    return found


@contextlib.contextmanager
def session_pad(ctx: LegContext, label: str):
    """A pad on the session instance, shown on the focused monitor beside the bystander. Yields the
    app, its settled toplevel and its event log."""
    tier = ctx.tier
    main = main_instance(tier)
    with pointer_pad(tier, main, label) as (owned, _, log), app_shown(tier, owned.pid):
        yield owned, session_client(tier, main, owned.pid), log


@contextlib.contextmanager
def beside_bystander(tier, main: dict, client: dict):
    """Parks the cursor on the pad, then samples the bystander's box and whether the cursor is still
    on the pad until the block ends. Yields a list that holds every sampled change afterwards."""
    lua = hyprland.uses_lua(shared(tier), main["signature"])
    centre = global_point(client, (0.5, 0.5))
    instance_control(tier, main, cursor_argv(centre[0], centre[1], lua))
    require(raw.poll(lambda: holds(cursor_at(tier, main), client), 3.0),
            f"the cursor did not reach the pad: j/cursorpos reads {cursor_at(tier, main)}")
    found: list[str] = []
    sampler = Sampler({"bystander": lambda: bystander_box(tier, main),
                       "cursor on the pad": lambda: holds(cursor_at(tier, main), client)})
    with sampler:
        yield found
    found.extend(sampler.changes)


def cursor_problems(tier, instance: dict, point, label: str) -> list[str]:
    at = cursor_at(tier, instance)
    if lands_at(dict(zip(("wx", "wy"), at)), point):
        return []
    return [f"{label}: j/cursorpos reads {at}, not {point}"]


def off_output_problems(ctx, env, log: Path, verb: str) -> list[str]:
    """A point on no output refuses with point_off_output, not_delivered, and reaches no pad."""
    start = len(events(log))
    refused = ctx.product(REFUSED[verb], env=env)
    error = refused.error or {}
    details = error.get("details") or {}
    problems = []
    if refused.ok or error.get("code") != "INVALID_ARGS" or details.get("cause") != "point_off_output":
        problems.append(f"an off-output {verb} answered {refused.raw[:300]}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"the off-output {verb} is not not_delivered: {error.get('disposition')}")
    if since(settled(log, lambda found: len(found) > start, 0.5), start):
        problems.append(f"the off-output {verb} reached the pad")
    return problems


def session_reads(ctx: LegContext, stage: str, main: dict, log: Path | None = None) -> None:
    """One read of the pad log, the bystander's box and the session cursor at `stage`."""
    record = getattr(ctx, stage)
    record("applog.pad", len(events(log)) if log is not None else pads_left(ctx.tier))
    record("hyprland.bystander", bystander_box(ctx.tier, main))
    record("hyprland.cursor", cursor_at(ctx.tier, main))


def session_delivery_report(ctx: LegContext, verb: str) -> None:
    """The Hyprland side of the shared wheel and drag delivery-report legs, on the session instance:
    the pad logs the event, j/cursorpos ends on the aim, an off-output point refuses, and the
    bystander's box and the cursor never leave their place."""
    tier = ctx.tier
    main = main_instance(tier)
    session_reads(ctx, "before", main)
    with session_pad(ctx, f"report-{verb}") as (_, client, log):
        env = instance_env(tier, main)
        aim, end_at = global_point(client, (0.5, 0.7)), global_point(client, (0.7, 0.9))
        with beside_bystander(tier, main, client) as found:
            if verb == "mouse-wheel":
                problems = wheel_problems(ctx, env, log, aim, ["--dy=-1"], ["down"])
                problems += cursor_problems(tier, main, aim, verb)
            else:
                problems = drag_problems(ctx, env, log, client, aim, end_at)
                problems += cursor_problems(tier, main, end_at, verb)
            problems += off_output_problems(ctx, env, log, verb)
        session_reads(ctx, "after", main, log)
        require(not problems and not found, "; ".join(problems + found))
    session_reads(ctx, "restored", main)


@implementation("pointer.global_layout_coordinates")
def global_layout_coordinates(ctx: LegContext) -> None:
    tier = ctx.tier
    instance, lua = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    with pointer_pad(tier, instance, "layout") as (owned, client, log):
        output = monitor_of(tier, instance, client)["name"]
        ctx.before("hyprland.monitors", layout(tier, instance))
        before = instance_query(tier, instance, "cursorpos")
        ctx.before("hyprland.cursor", before)
        try:
            with moved_output(tier, instance, lua, output, NEGATIVE_ORIGIN, FRACTIONAL):
                monitor = monitor_of(tier, instance, pad_client(tier, instance, owned.pid) or client)
                client = settled_client(tier, instance, owned.pid, monitor)
                ctx.after("hyprland.monitors", layout(tier, instance))
                problems = []
                aim = global_point(client, (0.5, 0.75))
                delivered(ctx.product(["mouse-move", f"--xy={xy(aim)}"], env=env))
                if not lands_at(dict(zip(("wx", "wy"), cursor_at(tier, instance))), aim):
                    problems.append(f"mouse-move to {aim}: j/cursorpos reads {cursor_at(tier, instance)}")
                problems += click_problems(ctx, env, log, client, global_point(client, (0.4, 0.7)), 1)
                problems += wheel_problems(ctx, env, log, aim, ["--dy=-2"], ["down", "down"])
                problems += wheel_problems(ctx, env, log, aim, ["--dx=1"], ["right"])
                problems += drag_problems(ctx, env, log, client, global_point(client, (0.3, 0.65)),
                                          global_point(client, (0.7, 0.85)))
                other = [item for item in instance_query(tier, instance, "monitors") if item["name"] != output]
                require(other, "the nested instance has one output")
                far = (other[0]["x"] + 40, other[0]["y"] + 40)
                delivered(ctx.product(["mouse-move", f"--xy={xy(far)}"], env=env))
                if not lands_at(dict(zip(("wx", "wy"), cursor_at(tier, instance))), far):
                    problems.append(f"mouse-move to {far} on {other[0]['name']}: j/cursorpos reads {cursor_at(tier, instance)}")
                problems += [f"from {other[0]['name']}, {problem}" for problem in
                             click_problems(ctx, env, log, client, global_point(client, (0.6, 0.7)), 1)]
                ctx.after("hyprland.cursor", instance_query(tier, instance, "cursorpos"))
                ctx.after("applog.pad", len(events(log)))
                require(not problems, "; ".join(problems))
        finally:
            ctx.restored("hyprland.monitors", layout(tier, instance))
            instance_control(tier, instance, cursor_argv(before["x"], before["y"], lua))
            require(raw.poll(lambda: instance_query(tier, instance, "cursorpos") == before, 3.0),
                    f"the nested cursor did not return to {before}")
            ctx.restored("hyprland.cursor", instance_query(tier, instance, "cursorpos"))
    ctx.restored("applog.pad", pads_left(tier))


@implementation("pointer.coordinates_have_no_receiver_proof")
def coordinates_have_no_receiver_proof(ctx: LegContext) -> None:
    tier = ctx.tier
    main = main_instance(tier)
    session_reads(ctx, "before", main)
    with session_pad(ctx, "coordinates") as (_, client, log):
        env = instance_env(tier, main)
        clicks = [(count, global_point(client, fraction)) for count, fraction in
                  ((1, (0.3, 0.6)), (2, (0.5, 0.6)), (3, (0.7, 0.6)))]
        aim, drag_from, drag_to = (global_point(client, fraction) for fraction in ((0.5, 0.8), (0.3, 0.7), (0.6, 0.9)))
        with beside_bystander(tier, main, client) as found:
            problems = []
            for count, point in clicks:
                problems += click_problems(ctx, env, log, client, point, count)
            problems += wheel_problems(ctx, env, log, aim, ["--dy=1"], ["up"])
            problems += cursor_problems(tier, main, aim, "mouse-wheel")
            problems += drag_problems(ctx, env, log, client, drag_from, drag_to)
            problems += cursor_problems(tier, main, drag_to, "drag")
            problems += off_output_problems(ctx, env, log, "mouse-click")
        session_reads(ctx, "after", main, log)
        require(not problems and not found, "; ".join(problems + found))
    session_reads(ctx, "restored", main)


def refusal_problems(label: str, envelope) -> list[str]:
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != "POLICY_DENIED":
        problems.append(f"{label} answered {envelope.raw[:300]}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label} is not not_delivered: {error.get('disposition')}")
    if details.get("physical_delivery_started") not in (None, False):
        problems.append(f"{label} started physical delivery")
    return problems


@implementation("pointer.headless_or_cancelled_input")
def headless_or_cancelled_input(ctx: LegContext) -> None:
    tier = ctx.tier
    main = main_instance(tier)
    session_reads(ctx, "before", main)
    with session_pad(ctx, "headless") as (owned, client, log):
        env = instance_env(tier, main)
        point = global_point(client, (0.5, 0.7))
        button = pad_ref(ctx, env, owned)
        pane = pad_ref(ctx, env, owned, PAD_SCROLL)
        with beside_bystander(tier, main, client) as found:
            start = len(events(log))
            start += len(held_still(log, start, len))
            cursor = cursor_at(tier, main)
            problems = []
            for argv in (["mouse-click", f"--xy={xy(point)}"],
                         ["mouse-wheel", f"--x={int(point[0])}", f"--y={int(point[1])}", "--dy=-1"],
                         ["drag", f"--from-xy={xy(point)}", f"--to-xy={int(point[0]) + 40},{int(point[1])}"],
                         ["double-click", button], ["triple-click", button],
                         ["scroll", pane, "--direction", "down", "--amount", "1"]):
                problems += refusal_problems(" ".join(argv[:1]), ctx.product(argv, env=env, headed=False))
            after = cursor_at(tier, main)
        session_reads(ctx, "after", main, log)
        if after != cursor:
            problems.append(f"the cursor moved from {cursor} to {after}")
        if since(settled(log, lambda seen: len(seen) > start, 0.5), start):
            problems.append(f"the pad logged events: {since(events(log), start)[:5]}")
        require(not problems and not found, "; ".join(problems + found))
    session_reads(ctx, "restored", main)


def _running_drag(tier, env, start_at, end_at) -> subprocess.Popen:
    argv = [product._binary_path(), "--headed", "drag", f"--from-xy={xy(start_at)}", f"--to-xy={xy(end_at)}",
            "--duration", str(CANCEL_DRAG_MS), "--timeout-ms", str(CANCEL_DRAG_MS + 5000)]
    run_env = product._result_env(tier)
    run_env.update(env)
    return subprocess.Popen(argv, env=run_env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                            pass_fds=product._load_bounded_process()._interaction_lease_fds(run_env))


def _pressed(log: Path, start: int) -> dict | None:
    found = [item for item in since(events(log), start) if item.get("ev") == "press" and item.get("widget") == "area"]
    return found[0] if found else None


def signal_problems(tier, env, log: Path, client: dict) -> list[str]:
    start_at, end_at = global_point(client, (0.2, 0.7)), global_point(client, (0.8, 0.9))
    start = len(events(log))
    running = _running_drag(tier, env, start_at, end_at)
    press = raw.poll(lambda: _pressed(log, start), 5.0)
    if not press:
        running.kill()
        running.wait(timeout=5)
        return ["the drag never pressed inside the pad"]
    running.send_signal(signal.SIGTERM)
    code = running.wait(timeout=5)
    found = since(settled(log, lambda all_: bool(releases(since(all_, start), "area"))), start)
    released = releases(found, "area")
    problems = []
    if code != -signal.SIGTERM:
        problems.append(f"the cancelled drag exited {code}, not by SIGTERM")
    if len(released) != 1:
        problems.append(f"the cancelled drag left {len(released)} releases in the pad{pad_saw(found)}")
        return problems
    if motions_after(found, released[0]["t"]):
        problems.append("motion reached the pad after the release")
    if lands_at(released[0], window_point(end_at, client), 4):
        problems.append("the cancelled drag reached its destination anyway")
    return problems


def disconnect_problems(tier, instance: dict, env, log: Path, client: dict) -> list[str]:
    start = len(events(log))
    running = _running_drag(tier, env, global_point(client, (0.2, 0.7)), global_point(client, (0.8, 0.9)))
    if not raw.poll(lambda: _pressed(log, start), 5.0):
        running.kill()
        running.wait(timeout=5)
        return ["the drag never pressed inside the pad"]
    require(nested_running(instance),
            "the nested instance is not the recorded process")
    os.kill(instance["pid"], signal.SIGKILL)
    stdout, _ = running.communicate(timeout=CANCEL_DRAG_MS / 1000 + 10)
    try:
        envelope = json.loads(stdout)
    except ValueError:
        return [f"the disconnected drag printed no envelope: {stdout[:300]}"]
    error = envelope.get("error") or {}
    details = error.get("details") or {}
    disposition = error.get("disposition") or {}
    problems = []
    if envelope.get("ok") is not False:
        problems.append("the disconnected drag reported success")
    if disposition != {"delivery": "delivery_uncertain", "retry": "unsafe"}:
        problems.append(f"the disconnected drag carries {disposition}")
    if details.get("physical_delivery_started") is not True or (details.get("delivered_events") or 0) < 2:
        problems.append(f"the disconnected drag reports {details}")
    if details.get("buttons_released") is not False:
        problems.append(f"a dead compositor was taken as a delivered release: {details.get('buttons_released')!r}")
    return problems


@implementation("pointer.cancel_or_disconnect_releases_buttons")
def cancel_or_disconnect_releases_buttons(ctx: LegContext) -> None:
    """SIGTERM and the next drag run on the session instance beside the bystander. Only the
    disconnect, which kills a compositor, runs on the nested instance."""
    tier = ctx.tier
    main = main_instance(tier)
    session_reads(ctx, "before", main)
    with session_pad(ctx, "cancel") as (_, client, log):
        env = instance_env(tier, main)
        with beside_bystander(tier, main, client) as found:
            problems = signal_problems(tier, env, log, client)
            problems += drag_problems(ctx, env, log, client, global_point(client, (0.3, 0.7)),
                                      global_point(client, (0.6, 0.9)))
        session_reads(ctx, "after", main, log)
        require(not problems and not found, "; ".join(problems + found))
    session_reads(ctx, "restored", main)
    instance, _ = nested(tier)
    ctx.before("applog.nested_pad", pads_left(tier))
    with pointer_pad(tier, instance, "disconnect") as (_, client, log):
        ctx.before("proc.nested", nested_running(instance))
        problems = disconnect_problems(tier, instance, instance_env(tier, instance), log, client)
        ctx.after("applog.nested_pad", len(events(log)))
        ctx.after("proc.nested", nested_running(instance))
        require(not problems, "; ".join(problems))
    ctx.restored("applog.nested_pad", pads_left(tier))
    ctx.restored("proc.nested", nested_running(nested(tier)[0]))
