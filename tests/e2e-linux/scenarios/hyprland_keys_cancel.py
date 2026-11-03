"""A headed `type` stopped by SIGTERM, then by SIGKILL, with Shift held, on the attached `hyprland` tier.

The leg starts the product itself, so it signals the exact process it
started and no other. It waits until the key log's newest key event is a
Shift press, which means the virtual keyboard holds Shift for an uppercase
letter, and signals then. After SIGTERM the app's log must show every key it
saw pressed released. SIGKILL gives the product no chance to release, and
Hyprland keeps a dead virtual keyboard's keys held while
`input:virtualkeyboard:release_pressed_on_close` is false, so a key held
then is a finding, not a failure, and so are the characters its autorepeat
adds to the entry before the next delivery releases it. After either signal
`j/devices` must list the keyboards it listed before, and a scoped `press x`
must release what was held and give the entry exactly one unmodified `x`. Each
round records a `hold` finding: the keys held at the signal and at the next
command, how long until the app saw them released, the repeated characters the
entry gained beyond the typed text, and whether keyboard focus stayed on it.
Wayland sends key events only to the focused surface, so with focus kept no
repeat reached a bystander.
"""

from __future__ import annotations

import contextlib
import json
import os
import signal
import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_keys import _focused_key_log
from scenarios.hyprland_focus_controls import active
from scenarios.hyprland_keys_common import HELD_MODIFIERS, entry_text, keyboards, records, toplevel
from scenarios.hyprland_keys_process import finished, running_product

CANCELLED = "aB" * 200
TYPE_TIMEOUT_MS = "30000"
STARTED_PRESSES = 8
IN_FLIGHT_S = 10.0
SETTLE_S = 4.0
SHIFTS = ("Shift_L", "Shift_R")
LABEL = "cancel"


def key_events(found: list[dict]) -> list[dict]:
    return [item for item in found if item.get("event") in ("press", "release")]


def held_keys(found: list[dict]) -> list[int]:
    """The hardware codes the app saw pressed and not yet released, in press order."""
    held: list[int] = []
    for item in key_events(found):
        code = item.get("code")
        if item["event"] == "press" and code not in held:
            held.append(code)
        elif item["event"] == "release" and code in held:
            held.remove(code)
    return held


def modifier_in_flight(found: list[dict]) -> bool:
    events = key_events(found)
    return bool(events) and events[-1]["event"] == "press" and events[-1].get("keysym") in SHIFTS


def next_press_problems(found: list[dict], keysym: str, repeating: set[int] = frozenset()) -> list[str]:
    """The key events after a cancel hold the lead Shift tap and exactly one clean `keysym`, all released.
    Presses of `repeating` codes are autorepeat of a key the signal left held."""
    presses = [item for item in key_events(found) if item["event"] == "press"]
    others = [item.get("keysym") for item in presses
              if item.get("keysym") not in (keysym, *SHIFTS) and item.get("code") not in repeating]
    wanted = [item for item in presses if item.get("keysym") == keysym]
    problems = [f"other keys arrived: {others}"] if others else []
    if len(wanted) != 1:
        problems.append(f"{len(wanted)} {keysym} presses arrived, not 1: {presses}")
    elif wanted[0].get("state", 0) & HELD_MODIFIERS:
        problems.append(f"{keysym} arrived with state {wanted[0].get('state')}: a modifier stayed held")
    if held_keys(found):
        problems.append(f"keys stayed held: {held_keys(found)}")
    return problems


def exit_problems(sent: int, code: int | None) -> list[str]:
    return [] if code == -sent else [f"the product exited {code}, not by signal {int(sent)}"]


def _last_text(found: list[dict]) -> str:
    texts = [item.get("text") or "" for item in found if item.get("event") == "text"]
    return texts[-1] if texts else ""


def collapse_repeats(text: str) -> str:
    """The text with its trailing run of one character cut to a single character."""
    return text.rstrip(text[-1:]) + text[-1:]


def next_text_matches(entry: str | None, typed: str, key: str, repeated: bool) -> bool:
    """The entry reads the typed text and then `key`; with `repeated`, autorepeat may have lengthened its last run."""
    if entry is None or not entry.endswith(key):
        return False
    if not repeated:
        return entry == typed + key
    return collapse_repeats(entry[:-len(key)]) == collapse_repeats(typed)


def _final_release(events: list[dict], code: int) -> float | None:
    """When the app last saw `code` released with no press after it. Autorepeat is a release and a press, so
    only the final release ends a hold."""
    last = None
    for item in events:
        if item.get("code") == code:
            last = item["t"] if item["event"] == "release" else None
    return last


def _next_delivery(events: list[dict], next_command: float, key: str) -> float:
    """When the next command's own keys start: its lead Shift tap or `key`."""
    return next((item["t"] for item in events if item["t"] > next_command and item["event"] == "press"
                 and item.get("keysym") in (*SHIFTS, key)), float("inf"))


def hold_measure(found: list[dict], signalled: float, next_command: float, expected: str, key: str) -> dict:
    """The keys held at the signal and at the next command, how long until the app saw the last of them released,
    and the characters the entry gained beyond `expected` until then. Autorepeat reaches the app as a release and a
    press per character, so the repeats are counted from the entry text. A hold ends before the next command's own
    keys, which press `key` after a Shift tap. Times are the app's monotonic clock."""
    before = [item for item in found if item.get("t", 0.0) <= signalled]
    after = key_events([item for item in found if item.get("t", 0.0) > signalled])
    held = held_keys(before)
    pressed = [item.get("code") for item in after if item["event"] == "press" and item["t"] <= next_command]
    bounded = [item for item in after if item["t"] < _next_delivery(after, next_command, key)]
    finals = {code: _final_release(bounded, code) for code in dict.fromkeys(held + pressed)}
    at_next = [code for code, final in finals.items() if final is None or final > next_command]
    if at_next:
        end = None if any(finals[code] is None for code in at_next) else max(finals[code] for code in at_next)
    else:
        end = max((item["t"] for item in after if item["t"] <= next_command), default=signalled)
    until = next_command if end is None else max(next_command, end)
    text = _last_text([item for item in found if item.get("t", 0.0) <= until])
    window = [item for item in found if signalled < item.get("t", 0.0) <= next_command]
    return {"held": held, "held_at_next": at_next,
            "held_s": None if end is None else round(end - signalled, 3),
            "window_s": round(next_command - signalled, 3),
            "repeat_chars": len(text) - len(os.path.commonprefix([text, expected])),
            "focus_left": any(item.get("event") == "focus" and not item.get("in") for item in window)}


def held_after_signal_problems(sent: int, held: list[int]) -> list[str]:
    """Keys still held after the signal fail SIGTERM; after SIGKILL only the next delivery can release them."""
    if not held or sent == signal.SIGKILL:
        return []
    return [f"{signal.Signals(sent).name}: the app saw keys {held} pressed and never released"]


def prefix_problems(typed: str | None, text: str) -> list[str]:
    if not typed or not text.startswith(typed) or typed == text:
        return [f"the entry reads {typed!r}, not a strict prefix of the text"]
    return []


def app_evidence(runtime_dir: Path, label: str, pid: int, found: list[dict]) -> str:
    """Whether the key log app still runs, the tail of its stderr, and its last records."""
    alive = "alive" if Path(f"/proc/{pid}").exists() else "gone"
    stderr = ""
    with contextlib.suppress(OSError):
        stderr = (runtime_dir / f"key-log-{label}.log").read_text()[-1500:]
    return f"the key log app (pid {pid}) is {alive}; stderr: {stderr!r}; last records: {found[-12:]}"


def product_log(runtime_dir: Path, sent: int) -> Path:
    """Where the signalled product writes its debug records, one file per signal."""
    return runtime_dir / f"product-{LABEL}-{signal.Signals(sent).name}.log"


def _settled_text(tier, app) -> str | None:
    last = [entry_text(tier, app)]

    def still() -> bool:
        now = entry_text(tier, app)
        same = now == last[0]
        last[0] = now
        return same

    raw.poll(still, SETTLE_S, floor_s=0.5)
    return last[0]


def _in_flight(log) -> bool:
    deadline = time.monotonic() + IN_FLIGHT_S
    while time.monotonic() < deadline:
        if modifier_in_flight(records(log)):
            return True
        time.sleep(0.002)
    return False


def _round(ctx: LegContext, app, log, ref: str, sent: int) -> list[str]:
    tier = ctx.tier
    base, start = entry_text(tier, app) or "", len(records(log))
    running = running_product(tier, ["type", ref, CANCELLED, "--timeout-ms", TYPE_TIMEOUT_MS],
                              debug_log=product_log(tier.runtime_dir, sent))
    started = raw.poll(lambda: len([item for item in records(log)[start:] if item.get("event") == "press"])
                       >= STARTED_PRESSES, 10.0)
    flying = bool(started) and _in_flight(log)
    seen = len(records(log))
    signalled = time.monotonic()
    running.send_signal(sent)
    code, _ = finished(running, 10.0)
    name = signal.Signals(sent).name
    if not flying:
        return [f"{name}: the type never held Shift while running (presses {len(records(log)) - start})"]
    typed = _settled_text(tier, app) or ""
    ctx.finding(f"{name}: signalled at app record {seen - start}, the app logged {len(records(log)) - start}")
    problems = [f"{name}: {item}" for item in exit_problems(sent, code)]
    cut = typed[len(base):]
    killed = sent == signal.SIGKILL
    problems += [f"{name}: {item}" for item in prefix_problems(collapse_repeats(cut) if killed else cut, CANCELLED)]
    held = held_keys(records(log)[start:])
    if held and sent == signal.SIGKILL:
        ctx.finding(f"{name}: the app saw keys {held} held until the next delivery")
    problems += held_after_signal_problems(sent, held)
    after = len(records(log))
    repeating = {item.get("code") for item in key_events(records(log)[start:after])} if killed else set()
    focused = active(tier).get("address") == toplevel(tier, app)["address"]
    next_command = time.monotonic()
    pressed = ctx.product(["press", "x", "--window-id", app.windows[0]], headed=False)
    if not pressed.ok:
        problems.append(f"{name}: the next press failed: {pressed.raw[:300]}")
    if not raw.poll(lambda: next_text_matches(entry_text(tier, app), typed, "x", killed), SETTLE_S):
        problems.append(f"{name}: the next press left {entry_text(tier, app)!r}, not {typed + 'x'!r}")
    raw.poll(lambda: not held_keys(records(log)[after:]), 2.0)
    problems += [f"{name}: {item}" for item in next_press_problems(records(log)[after:], "x", repeating)]
    measure = dict(hold_measure(records(log)[start:], signalled, next_command, base + CANCELLED, "x"), signal=name,
                   target_focused=focused)
    ctx.finding("hold " + json.dumps(measure, sort_keys=True))
    return problems


@implementation("input.hyprland_cancel_releases_keys")
def cancel_releases_keys(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with _focused_key_log(ctx, LABEL) as (app, log, ref):
        ctx.before("applog.held_keys", held_keys(records(log)))
        problems = []
        for sent in (signal.SIGTERM, signal.SIGKILL):
            try:
                problems += _round(ctx, app, log, ref, sent)
            except Exception:
                ctx.finding(f"{signal.Signals(sent).name}: "
                            f"{app_evidence(tier.runtime_dir, LABEL, app.pid, records(log))}")
                raise
            if not raw.poll(lambda: keyboards(tier) == before, 3.0):
                problems.append(f"{signal.Signals(sent).name}: the keyboards changed: {before} to {keyboards(tier)}")
        ctx.after("hyprland.keyboards", keyboards(tier))
        ctx.after("applog.held_keys", held_keys(records(log)))
        ctx.restored("applog.held_keys", held_keys(records(log)))
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))
