"""Headed `type` and scoped `press` on the attached `hyprland` tier, through a virtual keyboard.

Each leg focuses the key log app, a GTK3 entry that logs every key event it
receives with its keysym, modifier state and layout group, then types into
it with headed `type` or presses into it with headless `press --window-id`,
which sends keys only while the live focus guard confirms the window. The result is read from the entry's accessible text and from
the app's log, never from the command's own `ok`. `j/devices` shows that no
virtual keyboard stays behind and that the layout state is what it was. Each
leg gives focus and the cursor back to the bystander.
"""

from __future__ import annotations

import contextlib
import threading

from oracle import atspi_raw as raw
from oracle import hyprland
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import bystander, desktop, main_instance, shared
from scenarios.hyprland_focus_controls import active, cursor, move, restore_desktop
from scenarios.hyprland_keys_common import (HELD_MODIFIERS, SHIFT, entry_ref, entry_text, give_focus, key_log,
                                            keyboards, labelled, main_layout_index, presses, toplevel)

TYPED = "Hello, World! 123 ~ab"
RUSSIAN = "привет"
INTERRUPTED = "aB" * 150
INTERRUPT_AFTER_PRESSES = 8
TYPE_TIMEOUT_MS = "30000"


def _typed(ctx: LegContext, app, ref: str, text: str, expected: str) -> list[str]:
    envelope = ctx.product(["type", ref, text, "--timeout-ms", TYPE_TIMEOUT_MS])
    problems = labelled(f"type {text!r}", envelope, "VirtualKeyboard.type_text")
    if not raw.poll(lambda: entry_text(ctx.tier, app) == expected, 4.0):
        problems.append(f"the entry reads {entry_text(ctx.tier, app)!r}, not {expected!r}")
    return problems


def _pressed(ctx: LegContext, app, combo: str) -> list[str]:
    envelope = ctx.product(["press", combo, "--window-id", app.windows[0]], headed=False)
    return labelled(f"press {combo}", envelope, "VirtualKeyboard.press_key")


def _press_state(log, keysym: str) -> int | None:
    found = [item for item in presses(log) if item.get("keysym") == keysym]
    return found[-1].get("state") if found else None


@contextlib.contextmanager
def _focused_key_log(ctx: LegContext, label: str):
    tier = ctx.tier
    read = lambda: desktop(tier, main_instance(tier))
    holder, point = bystander(tier), cursor(tier)
    shown = read()["workspace"]
    ctx.before("hyprland.desktop", read())
    try:
        with key_log(tier, label) as (app, log):
            address = toplevel(tier, app)["address"]
            move(tier, address, shown)
            restore_desktop(tier, holder, point)
            give_focus(tier, address)
            yield app, log, entry_ref(ctx, app)
            ctx.after("hyprland.desktop", read())
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


@implementation("input.hyprland_type_text_virtual_keyboard")
def type_text_virtual_keyboard(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with _focused_key_log(ctx, "type") as (app, log, ref):
        problems = _typed(ctx, app, ref, TYPED, TYPED)
        names = [item.get("keysym") or "" for item in presses(log)]
        logged = "".join(name for name in names if len(name) == 1)
        if logged != "".join(char for char in TYPED if char.isalnum()):
            problems.append(f"the app logged letter and digit presses {logged!r}")
        problems += _typed(ctx, app, ref, "x", TYPED + "x")
        ctx.after("hyprland.keyboards", keyboards(tier))
        problems += [] if keyboards(tier) == before else [f"the keyboards changed: {before} to {keyboards(tier)}"]
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))


@implementation("input.hyprland_press_chord_virtual_keyboard")
def press_chord_virtual_keyboard(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    with _focused_key_log(ctx, "chord") as (app, log, ref):
        problems = _typed(ctx, app, ref, "abc", "abc")
        problems += _pressed(ctx, app, "ctrl+a") + _pressed(ctx, app, "x")
        if not raw.poll(lambda: entry_text(tier, app) == "x", 4.0):
            problems.append(f"ctrl+a then x left {entry_text(tier, app)!r}, not 'x'")
        problems += _pressed(ctx, app, "shift+a")
        if not raw.poll(lambda: entry_text(tier, app) == "xA", 4.0):
            problems.append(f"shift+a left {entry_text(tier, app)!r}, not 'xA'")
        state = _press_state(log, "A")
        if state is None or not state & SHIFT:
            problems.append(f"the app saw A pressed with state {state!r}, not Shift")
        problems += [] if keyboards(tier) == before else [f"the keyboards changed: {before} to {keyboards(tier)}"]
        require(not problems, "; ".join(problems))


def _kb_layout(tier) -> str:
    """The configured layouts, from getoption, or else from the one layout every physical keyboard names."""
    main = main_instance(tier)
    with contextlib.suppress(ValueError, RuntimeError, OSError):
        found = hyprland.query(shared(tier), main["signature"], "getoption input:kb_layout", pid=main["pid"])
        if isinstance(found, dict) and isinstance(found.get("str"), str) and found["str"]:
            return found["str"]
    physical = {item[2] for item in keyboards(tier) if not item[1]}
    require(len(physical) == 1, f"the configured kb_layout cannot be read: {keyboards(tier)}")
    return physical.pop()


def _set_kb_layout(tier, value: str) -> None:
    main = main_instance(tier)
    runtime, signature = shared(tier), main["signature"]
    quoted = hyprland.lua_string(value)
    argv = (["eval", f"hl.config({{ input = {{ kb_layout = {quoted} }} }})"]
            if hyprland.uses_lua(runtime, signature) else ["keyword", "input:kb_layout", value])
    output = hyprland.control(runtime, signature, argv)
    require(output == "ok", f"the compositor refused kb_layout {value!r}: {output}")


def _switch_layout(tier, index: int) -> None:
    main = main_instance(tier)
    with contextlib.suppress(RuntimeError):
        hyprland.control(shared(tier), main["signature"], ["switchxkblayout", "all", str(index)])
    require(raw.poll(lambda: main_layout_index(tier) == index, 3.0),
            f"the main keyboard never took layout {index}: {keyboards(tier)}")


@contextlib.contextmanager
def _russian_layout(tier):
    original = _kb_layout(tier)
    _set_kb_layout(tier, "us,ru")
    try:
        _switch_layout(tier, 1)
        yield
    finally:
        _set_kb_layout(tier, original)


@implementation("input.hyprland_type_non_us_layout")
def type_non_us_layout(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with _focused_key_log(ctx, "layout") as (app, log, ref):
        with _russian_layout(tier):
            switched = keyboards(tier)
            problems = _typed(ctx, app, ref, RUSSIAN, RUSSIAN)
            groups = {item.get("group") for item in presses(log) if item.get("keysym", "").startswith("Cyrillic")}
            if groups != {1}:
                problems.append(f"the Cyrillic presses arrived in layout groups {groups}, not {{1}}")
            if keyboards(tier) != switched:
                problems.append(f"typing changed the keyboards: {switched} to {keyboards(tier)}")
        restored = raw.poll(lambda: keyboards(tier) == before, 3.0)
        ctx.after("hyprland.keyboards", keyboards(tier))
        problems += [] if restored else [f"the layout did not come back: {before} to {keyboards(tier)}"]
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))


def _background_type(ctx: LegContext, ref: str, text: str) -> tuple[threading.Thread, dict]:
    result: dict = {}

    def run() -> None:
        result["envelope"] = ctx.product(["type", ref, text, "--timeout-ms", TYPE_TIMEOUT_MS])

    thread = threading.Thread(target=run, daemon=True)
    thread.start()
    return thread, result


def uncertain_problems(envelope, events_in_text: int) -> list[str]:
    """The stop is a delivery_uncertain error that counts some, but not all, of the events."""
    error = envelope.error or {}
    details = error.get("details") or {}
    delivered = details.get("delivered_events")
    problems = []
    if envelope.ok or (error.get("disposition") or {}).get("delivery") != "delivery_uncertain":
        problems.append(f"the interrupted type is not delivery_uncertain: {envelope.raw[:400]}")
    if not isinstance(delivered, int) or not 0 < delivered < events_in_text:
        problems.append(f"delivered_events is {delivered!r}, not between 0 and {events_in_text}")
    if details.get("physical_delivery_started") is not True:
        problems.append(f"physical_delivery_started is {details.get('physical_delivery_started')!r}")
    return problems


@implementation("input.hyprland_focus_change_mid_batch_uncertain")
def focus_change_mid_batch_uncertain(ctx: LegContext) -> None:
    tier = ctx.tier
    holder = bystander(tier)
    before = keyboards(tier)
    with _focused_key_log(ctx, "interrupt") as (app, log, ref):
        thread, result = _background_type(ctx, ref, INTERRUPTED)
        started = raw.poll(lambda: len(presses(log)) >= INTERRUPT_AFTER_PRESSES, 10.0)
        give_focus(tier, holder["address"])
        thread.join(40.0)
        require(started, f"the app logged only {len(presses(log))} presses")
        require("envelope" in result, "the interrupted type never answered")
        events = sum(4 if char.isupper() else 2 for char in INTERRUPTED)
        problems = uncertain_problems(result["envelope"], events)
        typed = entry_text(tier, app) or ""
        if not typed or not INTERRUPTED.startswith(typed) or typed == INTERRUPTED:
            problems.append(f"the entry reads {typed!r}, not a strict prefix of the text")
        if active(tier).get("address") != holder["address"]:
            problems.append(f"the bystander lost focus to {active(tier)}")
        problems += [] if keyboards(tier) == before else [f"the keyboards changed: {before} to {keyboards(tier)}"]
        give_focus(tier, toplevel(tier, app)["address"])
        problems += _typed(ctx, app, ref, "q", typed + "q")
        state = _press_state(log, "q")
        if state is None or state & HELD_MODIFIERS:
            problems.append(f"q arrived with state {state!r}: a modifier stayed held")
        require(not problems, "; ".join(problems))
