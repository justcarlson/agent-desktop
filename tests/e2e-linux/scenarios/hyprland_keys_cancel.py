"""A headed `type` stopped by SIGTERM, then by SIGKILL, with Shift held, on the attached `hyprland` tier.

The leg starts the product itself, so it signals the exact process it
started and no other. It waits until the key log's newest key event is a
Shift press, which means the virtual keyboard holds Shift for an uppercase
letter, and signals then. Afterwards the app's log must show every key it
saw pressed released, `j/devices` must list the keyboards it listed before,
and a scoped `press x` must give the entry exactly one unmodified `x`.
"""

from __future__ import annotations

import contextlib
import signal
import subprocess
import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner import product
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_keys import _focused_key_log
from scenarios.hyprland_keys_common import HELD_MODIFIERS, entry_text, keyboards, records

CANCELLED = "aB" * 200
TYPE_TIMEOUT_MS = "30000"
STARTED_PRESSES = 8
IN_FLIGHT_S = 10.0
SETTLE_S = 4.0
SHIFTS = ("Shift_L", "Shift_R")
LABEL = "cancel"


def running_product(tier, argv: list[str], headed: bool = True) -> subprocess.Popen:
    """The product as a child of this leg, with the run's environment and lease."""
    full = [product._binary_path(), *(["--headed"] if headed else []), *argv]
    env = product._result_env(tier)
    return subprocess.Popen(full, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                            pass_fds=product._load_bounded_process()._interaction_lease_fds(env))


def finished(running: subprocess.Popen, timeout_s: float) -> tuple[int | None, str]:
    try:
        out, _ = running.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        running.kill()
        out, _ = running.communicate(timeout=5)
    return running.returncode, out or ""


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


def next_press_problems(found: list[dict], keysym: str) -> list[str]:
    """The key events after a cancel hold the lead Shift tap and exactly one clean `keysym`, all released."""
    presses = [item for item in key_events(found) if item["event"] == "press"]
    others = [item.get("keysym") for item in presses if item.get("keysym") not in (keysym, *SHIFTS)]
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
    running = running_product(tier, ["type", ref, CANCELLED, "--timeout-ms", TYPE_TIMEOUT_MS])
    started = raw.poll(lambda: len([item for item in records(log)[start:] if item.get("event") == "press"])
                       >= STARTED_PRESSES, 10.0)
    flying = bool(started) and _in_flight(log)
    seen = len(records(log))
    running.send_signal(sent)
    code, _ = finished(running, 10.0)
    name = signal.Signals(sent).name
    if not flying:
        return [f"{name}: the type never held Shift while running (presses {len(records(log)) - start})"]
    typed = _settled_text(tier, app) or ""
    ctx.finding(f"{name}: signalled at app record {seen - start}, the app logged {len(records(log)) - start}")
    problems = [f"{name}: {item}" for item in exit_problems(sent, code)]
    problems += [f"{name}: {item}" for item in prefix_problems(typed[len(base):], CANCELLED)]
    if held_keys(records(log)[start:]):
        problems.append(f"{name}: the app saw keys {held_keys(records(log)[start:])} pressed and never released")
    after = len(records(log))
    pressed = ctx.product(["press", "x", "--window-id", app.windows[0]], headed=False)
    if not pressed.ok:
        problems.append(f"{name}: the next press failed: {pressed.raw[:300]}")
    if not raw.poll(lambda: entry_text(tier, app) == typed + "x", SETTLE_S):
        problems.append(f"{name}: the next press left {entry_text(tier, app)!r}, not {typed + 'x'!r}")
    raw.poll(lambda: not held_keys(records(log)[after:]), 2.0)
    problems += [f"{name}: {item}" for item in next_press_problems(records(log)[after:], "x")]
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
