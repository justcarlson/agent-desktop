"""Ref pointer legs on the attached `hyprland` tier, and the Hyprland side of the shared pointer legs.

Positive ref input goes to the nested second instance, where no shell layer covers the pad.
Covers the leg places are its own: a slurp selection layer, and a foot terminal it floats or
pins over the pad. The session instance is used only for refusals that send no event.
"""

from __future__ import annotations

import contextlib
from pathlib import Path

from oracle import atspi_raw as raw
from oracle import hyprland, proc
from runner import attached_session
from runner.apps import OwnedApp
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_close import _register, _release
from scenarios.hyprland_common import bystander, main_instance, shared
from scenarios.hyprland_focus_controls import cursor, cursor_argv, lua_call, restore_desktop, toggle_argv
from scenarios.hyprland_pointer import (delivered, nested, refusal_problems, drag_problems,
                                        click_problems, wheel_problems)
from scenarios.hyprland_pointer_pad import (COVER_CLASS, PAD_BUTTON, PAD_SCROLL, apart_from_last_click,
                                            covering_layers, events, gdk3_presses, global_point, instance_control, instance_env,
                                            instance_query, pad_client, pad_ref, pads_left, pointer_pad,
                                            presses, scrolls, settled, since)
from scenarios.hyprland_window_ops import float_argv

SLURP_NAMESPACE = "selection"
BUTTON_AIM_Y = 60
GESTURES = (("click", gdk3_presses(1)), ("double-click", gdk3_presses(2)), ("triple-click", gdk3_presses(3)))


def gesture_problems(ctx, env, log: Path, button: str, verb: str, want: list[int]) -> list[str]:
    apart_from_last_click(log)
    start = len(events(log))
    delivered(ctx.product([verb, button], env=env))
    found = presses(since(settled(log, lambda all_: presses(since(all_, start), "button") == want), start),
                    "button")
    return [] if found == want else [f"{verb} by ref: the pad button logged {found}, expected {want}"]


def scroll_problems(ctx, env, log: Path, pane: str) -> list[str]:
    start = len(events(log))
    delivered(ctx.product(["scroll", pane, "--direction", "down", "--amount", "2"], env=env))
    want = ["down", "down"]
    found = scrolls(since(settled(log, lambda all_: scrolls(since(all_, start), "scroll") == want), start),
                    "scroll")
    return [] if found == want else [f"scroll by ref: the pad scroll pane logged {found}, expected {want}"]


def receiver_refusal(label: str, envelope, cause: str) -> list[str]:
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != "ACTION_FAILED":
        problems.append(f"{label}: answered {envelope.raw[:300]}")
    if details.get("kind") != "receiver_evidence" or details.get("cause") != cause:
        problems.append(f"{label}: refused with {details.get('kind')}/{details.get('cause')}, expected {cause}")
    if details.get("physical_delivery_started") is not False:
        problems.append(f"{label}: physical_delivery_started is {details.get('physical_delivery_started')!r}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: not reported not_delivered")
    return problems


def _spawn_owned(tier, instance: dict, argv: list[str], label: str, found) -> OwnedApp:
    env = dict(tier.env, WAYLAND_DISPLAY=instance["wayland"], GDK_BACKEND="wayland")
    env.pop("AT_SPI_BUS_ADDRESS", None)
    env.pop("HYPRLAND_INSTANCE_SIGNATURE", None)
    attached_session.spawn(tier, argv, env, tier.runtime_dir / f"pointer-{label}.log")
    pid = raw.poll(found, 10.0)
    require(pid, f"the {label} never appeared on {instance['signature']}")
    start = proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]
    return _register(tier, OwnedApp(label, pid, start, [], [], ""))


def _cover_client(tier, instance: dict) -> dict | None:
    found = [item for item in instance_query(tier, instance, "clients") if item.get("class") == COVER_CLASS]
    return found[0] if len(found) == 1 and found[0].get("mapped") else None


def window_argv(verb: str, address: str, lua: bool, x: int = 0, y: int = 0) -> list[str]:
    target = f"address:{address}"
    if lua:
        fields = {"action": "enable", "window": target} if verb == "pin" else {"x": x, "y": y, "window": target}
        return ["dispatch", lua_call(f"hl.dsp.window.{verb}", fields)]
    hyprland.lua_string(target)
    if verb == "pin":
        return ["dispatch", "pin", target]
    return ["dispatch", f"{verb}windowpixel", f"exact {x} {y},{target}"]


@contextlib.contextmanager
def window_cover(tier, instance: dict, lua: bool, pid: int, pinned: bool):
    """A floating, or pinned, foot terminal over the whole pad. Yields its toplevel."""
    owned = _spawn_owned(tier, instance, ["foot", "--app-id", COVER_CLASS, "--title", COVER_CLASS, "sleep",
                                          "infinity"], "cover", lambda: (_cover_client(tier, instance) or {}).get("pid"))
    try:
        cover = _cover_client(tier, instance)
        instance_control(tier, instance, float_argv(cover["address"], lua))
        require(raw.poll(lambda: (_cover_client(tier, instance) or {}).get("floating") is True, 3.0),
                "the cover did not float")
        pad = raw.poll(lambda: pad_client(tier, instance, pid), 3.0)
        (x, y), (width, height) = pad["at"], pad["size"]
        instance_control(tier, instance, window_argv("resize", cover["address"], lua, width, height))
        instance_control(tier, instance, window_argv("move", cover["address"], lua, x, y))
        if pinned:
            instance_control(tier, instance, window_argv("pin", cover["address"], lua))
        placed = lambda: (lambda now: now if now and now["at"] == [x, y] and now["size"] == [width, height]
                          and bool(now.get("pinned")) == pinned else None)(_cover_client(tier, instance))
        require(raw.poll(placed, 3.0), f"the cover did not take the pad's box: {_cover_client(tier, instance)}")
        yield _cover_client(tier, instance)
    finally:
        _release(tier, owned)


@contextlib.contextmanager
def layer_cover(tier, instance: dict):
    """A slurp selection layer over the nested output, as overlay."""
    def mapped():
        for monitor in instance_query(tier, instance, "layers").values():
            for entries in (monitor.get("levels") or {}).values():
                for entry in entries:
                    if entry.get("namespace") == SLURP_NAMESPACE:
                        return entry.get("pid")
        return None
    owned = _spawn_owned(tier, instance, ["slurp"], "layer-cover", mapped)
    try:
        yield
    finally:
        _release(tier, owned)
        require(raw.poll(lambda: mapped() is None, 3.0), "the slurp layer stayed mapped")


def covered_click(ctx, env, log: Path, button: str, cause: str, label: str) -> list[str]:
    start = len(events(log))
    problems = receiver_refusal(label, ctx.product(["click", button], env=env), cause)
    if since(settled(log, lambda found: len(found) > start, 0.5), start):
        problems.append(f"{label}: the pad logged an event under the cover")
    return problems


def clients_summary(tier, instance: dict) -> list[tuple]:
    return sorted((item.get("class"), item.get("floating"), item.get("pinned"))
                  for item in instance_query(tier, instance, "clients"))


@implementation("pointer.ref_receiver_evidence")
def ref_receiver_evidence(ctx: LegContext) -> None:
    tier = ctx.tier
    instance, lua = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    with pointer_pad(tier, instance, "receiver") as (owned, _, log):
        button = pad_ref(ctx, env, owned)
        ctx.before("hyprland.clients", clients_summary(tier, instance))
        problems = [problem for verb, want in GESTURES
                    for problem in gesture_problems(ctx, env, log, button, verb, want)]
        problems += scroll_problems(ctx, env, log, button)
        with layer_cover(tier, instance):
            problems += covered_click(ctx, env, log, button, "layer_cover", "slurp layer cover")
        problems += gesture_problems(ctx, env, log, button, "click", [1])
        for pinned in (False, True):
            label = "pinned cover" if pinned else "floating cover"
            with window_cover(tier, instance, lua, owned.pid, pinned):
                problems += covered_click(ctx, env, log, button, "window_cover", label)
            problems += gesture_problems(ctx, env, log, button, "click", [1])
        ctx.after("hyprland.clients", clients_summary(tier, instance))
        ctx.after("applog.pad", len(events(log)))
        require(not problems, "; ".join(problems))
        ctx.restored("hyprland.clients", clients_summary(tier, instance))
    ctx.restored("applog.pad", pads_left(tier))


def _session_refusals(ctx, owned, client: dict, log: Path) -> list[str]:
    tier, main = ctx.tier, main_instance(ctx.tier)
    env = instance_env(tier, main)
    button = pad_ref(ctx, env, owned)
    problems = covered_click(ctx, env, log, button, "target_not_shown", "pad on a hidden special workspace")
    special = (client.get("workspace") or {}).get("name", "")
    require(special.startswith("special:"), f"the pad is not on a special workspace: {special}")
    instance_control(tier, main, toggle_argv(special, hyprland.uses_lua(shared(tier), main["signature"])))
    shown = raw.poll(lambda: pad_client(tier, main, owned.pid), 3.0)
    aim = (shown["at"][0] + shown["size"][0] / 2, shown["at"][1] + BUTTON_AIM_Y)
    layers = covering_layers(instance_query(tier, main, "layers"), aim)
    require(layers, f"no top or overlay layer holds {aim}; this leg expects the stock Omarchy shell layers")
    ctx.finding(f"shell layers over the pad button: {layers}")
    return problems + covered_click(ctx, env, log, button, "layer_cover", f"shell layer {layers[0]}")


@implementation("pointer.cover_or_missing_evidence_refused")
def cover_or_missing_evidence_refused(ctx: LegContext) -> None:
    """Unreadable layers cannot be made from outside the compositor; unit tests cover that refusal."""
    tier = ctx.tier
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.cursor", point)
    ctx.before("applog.pad", pads_left(tier))
    with pointer_pad(tier, main_instance(tier), "session-refusals") as (owned, client, log):
        try:
            problems = _session_refusals(ctx, owned, client, log)
        finally:
            restore_desktop(tier, holder, point)
    instance, lua = nested(tier)
    env = instance_env(tier, instance)
    with pointer_pad(tier, instance, "cover-refusals") as (owned, _, log):
        button = pad_ref(ctx, env, owned)
        ctx.before("hyprland.clients", clients_summary(tier, instance))
        for pinned in (False, True):
            with window_cover(tier, instance, lua, owned.pid, pinned) as cover:
                problems += covered_click(ctx, env, log, button, "window_cover",
                                          "pinned cover" if pinned else "floating cover")
                if _cover_client(tier, instance) != cover:
                    problems.append(f"the refusal changed the cover: {_cover_client(tier, instance)}")
        ctx.after("hyprland.clients", clients_summary(tier, instance))
        ctx.after("applog.pad", len(events(log)))
        ctx.restored("hyprland.clients", clients_summary(tier, instance))
    ctx.after("hyprland.cursor", cursor(tier))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.cursor", cursor(tier))
    ctx.restored("applog.pad", pads_left(tier))


def ref_gesture(ctx: LegContext, verb: str) -> None:
    """The Hyprland side of the shared double-click, triple-click and scroll legs."""
    tier = ctx.tier
    instance, _ = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    with pointer_pad(tier, instance, verb) as (owned, _, log):
        button = pad_ref(ctx, env, owned, PAD_SCROLL if verb == "scroll" else PAD_BUTTON)
        argv = ["scroll", button, "--direction", "down", "--amount", "2"] if verb == "scroll" else [verb, button]
        if ctx.mode == "headless":
            problems = refusal_problems(verb, ctx.product(argv, env=env))
            if settled(log, lambda found: bool(found), 0.5):
                problems.append(f"the refused {verb} reached the pad")
        elif verb == "scroll":
            problems = scroll_problems(ctx, env, log, button)
        else:
            problems = gesture_problems(ctx, env, log, button, verb, dict(GESTURES)[verb])
        ctx.after("applog.pad", len(events(log)))
        require(not problems, "; ".join(problems))
    ctx.restored("applog.pad", pads_left(tier))


def delivery_report(ctx: LegContext, verb: str) -> None:
    """The Hyprland side of the shared delivery-report legs. Each verb moves the nested cursor, so the
    leg puts it back where it found it."""
    tier = ctx.tier
    instance, lua = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    with pointer_pad(tier, instance, f"report-{verb}") as (_, client, log):
        before = instance_query(tier, instance, "cursorpos")
        ctx.before("hyprland.cursor", before)
        aim = global_point(client, (0.5, 0.7))
        if verb == "mouse-click":
            problems = click_problems(ctx, env, log, client, aim, 1)
            refused = ["mouse-click", "--xy=-5000,-5000"]
        elif verb == "mouse-wheel":
            problems = wheel_problems(ctx, env, log, aim, ["--dy=-1"], ["down"])
            refused = ["mouse-wheel", "--x=-5000", "--y=-5000"]
        else:
            problems = drag_problems(ctx, env, log, client, aim, global_point(client, (0.7, 0.9)))
            refused = ["drag", "--from-xy=-5000,-5000", "--to-xy=-4990,-4990"]
        ctx.after("hyprland.cursor", instance_query(tier, instance, "cursorpos"))
        start = len(events(log))
        envelope = ctx.product(refused, env=env)
        error = envelope.error or {}
        if envelope.ok or (error.get("disposition") or {}).get("delivery") != "not_delivered":
            problems.append(f"an off-output {verb} answered {envelope.raw[:300]}")
        if since(settled(log, lambda found: len(found) > start, 0.5), start):
            problems.append(f"the refused {verb} reached the pad")
        ctx.after("applog.pad", len(events(log)))
        instance_control(tier, instance, cursor_argv(before["x"], before["y"], lua))
        require(raw.poll(lambda: instance_query(tier, instance, "cursorpos") == before, 3.0),
                f"the nested cursor did not return to {before}")
        require(not problems, "; ".join(problems))
        ctx.restored("hyprland.cursor", instance_query(tier, instance, "cursorpos"))
    ctx.restored("applog.pad", pads_left(tier))
