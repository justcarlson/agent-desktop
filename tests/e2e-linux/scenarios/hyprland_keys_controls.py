"""Return, Ctrl+A then Backspace, and Tab through a virtual keyboard on the attached `hyprland` tier.

Return goes to the zoo's multi-line text view and must add exactly one
newline. Ctrl+A then Backspace goes to the key log's entry and must clear
it, with Control released before Backspace. Tab goes to the zoo's entry and
must move focus to the next control, the password entry, without inserting
a tab. Each result is read from AT-SPI and from the app's own log, and
`j/devices` must list the same keyboards afterwards.
"""

from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import find_ref
from scenarios.hyprland_common import bystander, desktop, main_instance
from scenarios.hyprland_focus_controls import cursor, move, restore_desktop
from scenarios.hyprland_keys import _focused_key_log, _pressed, _typed
from scenarios.hyprland_keys_common import CONTROL, entry_text, give_focus, keyboards, labelled, records, toplevel
from scenarios.parallel_zoo import Zoo, zoos

TEXT_VIEW, ENTRY, PASSWORD = "zoo-text", "zoo-entry", "zoo-password"
FOCUSED = "FOCUSED"
TYPE_TIMEOUT_MS = "30000"


def newline_problems(text: str | None, expected: str) -> list[str]:
    if text != expected:
        count = text.count("\n") if isinstance(text, str) else None
        return [f"the text reads {text!r} with {count} newlines, not {expected!r}"]
    return []


def backspace_problems(found: list[dict]) -> list[str]:
    backspaces = [item for item in found if item.get("event") == "press" and item.get("keysym") == "BackSpace"]
    if len(backspaces) != 1:
        return [f"{len(backspaces)} BackSpace presses arrived, not 1"]
    if backspaces[0].get("state", 0) & CONTROL:
        return [f"BackSpace arrived with state {backspaces[0].get('state')}: Control stayed held"]
    return []


def tab_problems(text: str | None, expected: str, *, focused_password: bool, password_focus_events: int) -> list[str]:
    problems = [] if text == expected else [f"Tab left the entry reading {text!r}, not {expected!r}"]
    if not focused_password:
        problems.append("the password entry does not carry the focused state after Tab")
    if password_focus_events < 1:
        problems.append("the zoo logged no focus event on the password entry after Tab")
    return problems


def zoo_ref(ctx: LegContext, zoo: Zoo, name: str) -> str:
    envelope = ctx.product(["snapshot", "-i", "--window-id", zoo.app.windows[0]], headed=False)
    require(envelope.ok, envelope.raw[:400])
    found = find_ref((envelope.data or {}).get("tree"), name=name)
    require(found, f"the zoo exposes no ref named {name}")
    return found


def zoo_node(tier, zoo: Zoo, name: str):
    found = raw.app_root(tier, zoo.app.owner_names[0]).find(name=name)
    require(len(found) == 1, f"the zoo has {len(found)} nodes named {name}")
    return found[0]


def zoo_text(tier, zoo: Zoo, name: str) -> str | None:
    with contextlib.suppress(Exception):
        return zoo_node(tier, zoo, name).text()
    return None


@contextlib.contextmanager
def focused_zoo(ctx: LegContext, record: bool = True):
    """The GTK3 zoo on the shown workspace with focus, and the desktop given back to the bystander after.
    With `record`, the desktop is the leg's `hyprland.desktop` channel."""
    tier = ctx.tier
    read = lambda: desktop(tier, main_instance(tier))
    note = (lambda stage: getattr(ctx, stage)("hyprland.desktop", read())) if record else (lambda stage: None)
    holder, point = bystander(tier), cursor(tier)
    shown = read()["workspace"]
    note("before")
    try:
        with zoos(tier, "A") as launched:
            zoo = launched["A"]
            address = toplevel(tier, zoo.app)["address"]
            move(tier, address, shown)
            restore_desktop(tier, holder, point)
            give_focus(tier, address)
            yield zoo
            note("after")
    finally:
        restore_desktop(tier, holder, point)
    note("restored")


def zoo_typed(ctx: LegContext, zoo: Zoo, ref: str, name: str, text: str, expected: str) -> list[str]:
    envelope = ctx.product(["type", ref, text, "--timeout-ms", TYPE_TIMEOUT_MS])
    problems = labelled(f"type {text!r}", envelope, "VirtualKeyboard.type_text")
    if not raw.poll(lambda: zoo_text(ctx.tier, zoo, name) == expected, 4.0):
        problems.append(f"{name} reads {zoo_text(ctx.tier, zoo, name)!r}, not {expected!r}")
    return problems


def zoo_pressed(ctx: LegContext, zoo: Zoo, combo: str) -> list[str]:
    envelope = ctx.product(["press", combo, "--window-id", zoo.app.windows[0]], headed=False)
    return labelled(f"press {combo}", envelope, "VirtualKeyboard.press_key")


def tab_moves_focus(ctx: LegContext, zoo: Zoo) -> list[str]:
    """Types into the zoo's entry, presses Tab, and checks that focus moved to the password entry."""
    tier = ctx.tier
    before = zoo_text(tier, zoo, ENTRY) or ""
    problems = zoo_typed(ctx, zoo, zoo_ref(ctx, zoo, ENTRY), ENTRY, "xy", before + "xy")
    focus_events = len(zoo.events(PASSWORD, "focus"))
    problems += zoo_pressed(ctx, zoo, "tab")
    focused = raw.poll(lambda: FOCUSED in zoo_node(tier, zoo, PASSWORD).states(), 3.0)
    raw.poll(lambda: len(zoo.events(PASSWORD, "focus")) > focus_events, 2.0)
    return problems + tab_problems(zoo_text(tier, zoo, ENTRY), before + "xy", focused_password=bool(focused),
                                   password_focus_events=len(zoo.events(PASSWORD, "focus")) - focus_events)


@implementation("input.hyprland_press_return_one_newline")
def press_return_one_newline(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with focused_zoo(ctx) as zoo:
        ref = zoo_ref(ctx, zoo, TEXT_VIEW)
        problems = zoo_typed(ctx, zoo, ref, TEXT_VIEW, "ab", "ab")
        problems += zoo_pressed(ctx, zoo, "return")
        problems += zoo_typed(ctx, zoo, ref, TEXT_VIEW, "c", "ab\nc")
        problems += newline_problems(zoo_text(tier, zoo, TEXT_VIEW), "ab\nc")
        logged = [item.get("value") for item in zoo.events(TEXT_VIEW, "changed")]
        problems += [f"the zoo logged {item}" for item in newline_problems(logged[-1] if logged else None, "ab\nc")]
        ctx.after("hyprland.keyboards", keyboards(tier))
        problems += [] if keyboards(tier) == before else [f"the keyboards changed: {before} to {keyboards(tier)}"]
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))


@implementation("input.hyprland_press_select_all_backspace_clears")
def press_select_all_backspace_clears(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with _focused_key_log(ctx, "backspace") as (app, log, ref):
        problems = _typed(ctx, app, ref, "abc", "abc")
        start = len(records(log))
        problems += _pressed(ctx, app, "ctrl+a") + _pressed(ctx, app, "backspace")
        if not raw.poll(lambda: entry_text(tier, app) == "", 4.0):
            problems.append(f"ctrl+a then backspace left {entry_text(tier, app)!r}, not ''")
        problems += backspace_problems(records(log)[start:])
        problems += _typed(ctx, app, ref, "q", "q")
        ctx.after("hyprland.keyboards", keyboards(tier))
        problems += [] if keyboards(tier) == before else [f"the keyboards changed: {before} to {keyboards(tier)}"]
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))


@implementation("input.hyprland_press_tab_moves_focus")
def press_tab_moves_focus(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with focused_zoo(ctx) as zoo:
        problems = tab_moves_focus(ctx, zoo)
        ctx.after("hyprland.keyboards", keyboards(tier))
        problems += [] if keyboards(tier) == before else [f"the keyboards changed: {before} to {keyboards(tier)}"]
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))
