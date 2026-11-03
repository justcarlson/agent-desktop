"""A headed `type` killed with a key down, and the key guard that releases it, on the attached `hyprland` tier.

Hyprland keeps a destroyed virtual keyboard's pressed keys held while
`input:virtualkeyboard:release_pressed_on_close` is false, so a `type` killed
with a key down leaves that key held, and the app repeats it. The product's
key guard, a process in its own session, sends the key-ups through a new
virtual keyboard when the command dies with keys still held.

Each round types into the key log and kills the product with a key down: by
SIGKILL of its pid, or by SIGKILL of its whole process group, with the product
started in a group of its own. To find a key down, the leg stops the product
with SIGSTOP and waits until the key log stops growing. If the app saw a key
pressed and not released, the leg kills the stopped product. Otherwise it
continues the product and tries again. The held key is a letter, in a text
with no uppercase, or Shift, which stays held across the focus gate before an
uppercase letter. No keyboard command runs after the kill. The app must see
every held key released before the compositor's repeat delay has passed since
its press, and the entry must gain no character. When the release is seen,
focus moves to a second key log, the bystander, which must log no key event.
GTK does not log the keys that a keyboard enter carries. The bystander check
therefore depends on the target seeing every held key released before the
focus moved: the seat then held no key for the enter to carry.
"""

from __future__ import annotations

import contextlib
import json
import os
import random
import shutil
import signal
import subprocess
import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner import apps, attached_session, product
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_common import option
from scenarios.hyprland_close import _register, _release, private_interpreter
from scenarios.hyprland_common import desktop, main_instance
from scenarios.hyprland_focus_controls import move
from scenarios.hyprland_keys import _focused_key_log
from scenarios.hyprland_keys_cancel import SHIFTS, _final_release, held_keys, key_events
from scenarios.hyprland_keys_common import (KEY_LOG_CLASS, KEY_LOG_NAMES, entry_text, give_focus, key_log_argv,
                                            keyboards, listener_anchor, records, toplevel)
from scenarios.hyprland_keys_process import DEBUG_FILTER, finished, running_product

LABEL = "guard"
BYSTANDER_LABEL = "guard-bystander"
LETTERS = "abcdefghij" * 200
SHIFTED = "aB" * 400
TYPE_TIMEOUT_MS = "120000"
ROUNDS = (("pid", "letter"), ("pid", "shift"), ("group", "letter"), ("group", "shift"))
STOPS = 3000
PROBE_S = 60.0
RUN_MAX_S = 0.003
QUIET_S = 0.003
HELD_QUIET_S = 0.03
STOP_WAIT_S = 0.2
RELEASE_MARGIN_S = 1.0
BYSTANDER_SETTLE_S = 0.5
SETTLE_S = 4.0


class Tail:
    """The complete records of a growing key log, each read continuing where the last one stopped.
    The app may still be writing the text after the last newline."""

    def __init__(self, path: Path, offset: int = 0):
        self.path, self.offset, self.found = path, offset, []

    @classmethod
    def at_end(cls, path: Path) -> "Tail":
        return cls(path, path.stat().st_size)

    def size(self) -> int:
        return self.path.stat().st_size

    def records(self) -> list[dict]:
        with self.path.open("rb") as handle:
            handle.seek(self.offset)
            chunk = handle.read()
        complete = chunk.rpartition(b"\n")[0]
        if complete:
            self.offset += len(complete) + 1
            self.found += [json.loads(line) for line in complete.decode().splitlines() if line.strip()]
        return self.found


def held_presses(found: list[dict]) -> list[dict]:
    """The press records of the keys the app saw pressed and not yet released, in press order."""
    held: dict[int, dict] = {}
    for item in key_events(found):
        if item["event"] == "press":
            held.setdefault(item.get("code"), item)
        else:
            held.pop(item.get("code"), None)
    return list(held.values())


def wanted(kind: str, press: dict) -> bool:
    shift = press.get("keysym") in SHIFTS
    return shift if kind == "shift" else not shift


def stopped_state(stat: str) -> bool:
    return stat.rpartition(")")[2].split()[:1] in (["T"], ["t"])


def _stopped(pid: int) -> bool:
    with contextlib.suppress(OSError):
        return stopped_state(Path(f"/proc/{pid}/stat").read_text())
    return False


def _quiet(tail: Tail, quiet_s: float) -> None:
    """Waits until the log has not grown for `quiet_s`."""
    size, still = tail.size(), time.monotonic()
    limit = still + STOP_WAIT_S
    while time.monotonic() - still < quiet_s and time.monotonic() < limit:
        time.sleep(0.0005)
        now = tail.size()
        if now != size:
            size, still = now, time.monotonic()


def _held_while_stopped(running: subprocess.Popen, tail: Tail, kind: str) -> list[dict]:
    running.send_signal(signal.SIGSTOP)
    limit = time.monotonic() + STOP_WAIT_S
    while not _stopped(running.pid) and running.poll() is None and time.monotonic() < limit:
        time.sleep(0.0002)
    _quiet(tail, QUIET_S)
    if not any(wanted(kind, item) for item in held_presses(tail.records())):
        return []
    _quiet(tail, HELD_QUIET_S)
    held = held_presses(tail.records())
    return held if any(wanted(kind, item) for item in held) else []


def catch_held(running: subprocess.Popen, tail: Tail, kind: str) -> tuple[list[dict], int]:
    """Stops the product until the app shows a `kind` key held, and leaves it stopped then; otherwise it runs on.
    Returns the held press records and the number of stops."""
    limit = time.monotonic() + PROBE_S
    for stop in range(1, STOPS + 1):
        if running.poll() is not None:
            return [], stop - 1
        held = _held_while_stopped(running, tail, kind)
        if held:
            return held, stop
        running.send_signal(signal.SIGCONT)
        if time.monotonic() > limit:
            return [], stop
        time.sleep(random.uniform(0.0, RUN_MAX_S))
    return [], STOPS


def guard_measure(found: list[dict], killed: float, held: list[dict], delay_ms: int) -> dict:
    """How long each held key stayed held after its press and after the kill, and the presses after the kill.
    Times are the app's monotonic clock, which is the leg's."""
    after = key_events([item for item in found if item.get("t", 0.0) > killed])
    finals = {item["code"]: _final_release(after, item["code"]) for item in held}
    open_codes = [code for code, final in finals.items() if final is None]
    press_ms = [round((finals[item["code"]] - item["t"]) * 1000) for item in held if finals[item["code"]] is not None]
    kill_ms = [round((final - killed) * 1000) for final in finals.values() if final is not None]
    return {"held": [item.get("keysym") for item in held], "held_open": open_codes, "repeat_delay_ms": delay_ms,
            "release_after_press_ms": None if open_codes else max(press_ms, default=0),
            "release_after_kill_ms": None if open_codes else max(kill_ms, default=0),
            "extra_presses": len([item for item in after if item["event"] == "press"])}


def extra_chars(entry: str, expected: str) -> int:
    return len(entry) - len(os.path.commonprefix([entry, expected]))


def guard_problems(name: str, measure: dict, code: int | None) -> list[str]:
    problems = [] if code == -signal.SIGKILL else [f"{name}: the product exited {code}, not by SIGKILL"]
    delay = measure["repeat_delay_ms"]
    if measure["held_open"]:
        problems.append(f"{name}: the app saw keys {measure['held_open']} still held "
                        f"{delay + RELEASE_MARGIN_S * 1000:.0f} ms after the kill")
    elif measure["release_after_press_ms"] >= delay:
        problems.append(f"{name}: the held keys were released {measure['release_after_press_ms']} ms after "
                        f"their press, not before the {delay} ms repeat delay")
    for field in ("extra_presses", "extra_chars", "bystander_events"):
        if measure.get(field):
            problems.append(f"{name}: {field} {measure[field]}, not 0")
    return problems


def bystander_events(found: list[dict]) -> list[dict]:
    return [item for item in found if item.get("event") in ("press", "release", "text")]


def group_product(tier, argv: list[str], debug_log: Path) -> subprocess.Popen:
    """The product as a child of this leg in a process group of its own, so the leg can kill the whole group."""
    env = product._result_env(tier)
    env["RUST_LOG"] = DEBUG_FILTER
    with debug_log.open("ab") as stderr:
        return subprocess.Popen([product._binary_path(), "--headed", *argv], env=env, stdout=subprocess.PIPE,
                                stderr=stderr, text=True, process_group=0,
                                pass_fds=product._load_bounded_process()._interaction_lease_fds(env))


def _kill(running: subprocess.Popen, mode: str) -> None:
    if mode == "group":
        os.killpg(running.pid, signal.SIGKILL)
    else:
        running.send_signal(signal.SIGKILL)


def _bystander_owner(tier, target_pid: int):
    found = [app for app in apps._owners(tier, names=KEY_LOG_NAMES)
             if app.pid != target_pid and len(app.windows) == 1]
    return found[0] if len(found) == 1 else None


@contextlib.contextmanager
def bystander_log(tier, target_pid: int, workspace: str):
    """A second key log on `workspace`, registered to the run. Yields its toplevel address and its log path."""
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    python = shutil.which("python3", path=env.get("PATH"))
    require(python, "no python3 on the session PATH")
    interpreter = private_interpreter(tier.runtime_dir, Path(python).resolve(), KEY_LOG_CLASS)
    log = tier.runtime_dir / f"key-log-{BYSTANDER_LABEL}.jsonl"
    log.write_text("")
    attached_session.spawn(tier, key_log_argv(interpreter, log, False), env,
                           tier.runtime_dir / f"key-log-{BYSTANDER_LABEL}.log", toplevels=(KEY_LOG_CLASS, 1))
    owner = raw.poll(lambda: _bystander_owner(tier, target_pid), SETTLE_S)
    require(owner, "the bystander key log never registered its window")
    owned = _register(tier, owner)
    with contextlib.ExitStack() as stack:
        try:
            stack.enter_context(listener_anchor(tier, owned.owner_names[0]))
        finally:
            stack.callback(_release, tier, owned)
        address = toplevel(tier, owned)["address"]
        move(tier, address, workspace)
        yield address, log


def _start(tier, mode: str, ref: str, text: str) -> subprocess.Popen:
    argv = ["type", ref, text, "--timeout-ms", TYPE_TIMEOUT_MS]
    debug_log = tier.runtime_dir / f"product-{LABEL}-{mode}.log"
    if mode == "group":
        return group_product(tier, argv, debug_log)
    return running_product(tier, argv, debug_log=debug_log)


def _visit_bystander(tier, address: str, log: Path) -> int:
    tail = Tail.at_end(log)
    give_focus(tier, address)
    time.sleep(BYSTANDER_SETTLE_S)
    return len(bystander_events(tail.records()))


def _clean_up(ctx: LegContext, app, log: Path, start: int) -> None:
    """A key the guard left held is released by the next delivery, so later rounds start clean."""
    give_focus(ctx.tier, toplevel(ctx.tier, app)["address"])
    ctx.product(["press", "x", "--window-id", app.windows[0]], headed=False)
    raw.poll(lambda: not held_keys(records(log)[start:]), SETTLE_S)


def _round(ctx: LegContext, scene: tuple, mode: str, kind: str) -> list[str]:
    tier, (app, log, ref, bystander, bystander_path, delay_ms) = ctx.tier, scene
    name = f"{mode} {kind}"
    give_focus(tier, toplevel(tier, app)["address"])
    base, start = entry_text(tier, app) or "", len(records(log))
    text = SHIFTED if kind == "shift" else LETTERS
    tail = Tail.at_end(log)
    running = _start(tier, mode, ref, text)
    started = raw.poll(lambda: any(item.get("event") == "press" for item in tail.records()), 10.0)
    held, stops = catch_held(running, tail, kind) if started else ([], 0)
    killed = time.monotonic()
    if not held:
        running.send_signal(signal.SIGTERM)
        finished(running, 10.0)
        return [f"{name}: the app never showed a {kind} held while the product was stopped ({stops} stops)"]
    _kill(running, mode)
    code, _ = finished(running, 10.0)
    codes = [item["code"] for item in held]
    raw.poll(lambda: not set(codes) & set(held_keys(records(log)[start:])), delay_ms / 1000 + RELEASE_MARGIN_S)
    measure = guard_measure(records(log)[start:], killed, held, delay_ms)
    entry = raw.poll(lambda: entry_text(tier, app) is not None, SETTLE_S) and entry_text(tier, app) or ""
    measure.update(round=name, stops=stops, exit=code, extra_chars=extra_chars(entry, base + text),
                   bystander_events=_visit_bystander(tier, bystander, bystander_path))
    ctx.finding("guard " + json.dumps(measure, sort_keys=True))
    if held_keys(records(log)[start:]):
        _clean_up(ctx, app, log, start)
    return guard_problems(name, measure, code)


def _repeat_delay(ctx: LegContext) -> int:
    delay = option(ctx.tier, "input:repeat_delay").get("int")
    require(isinstance(delay, int) and delay > 0, f"no repeat delay from getoption: {delay}")
    ctx.finding(f"repeat_delay_ms {delay}; repeat_rate {option(ctx.tier, 'input:repeat_rate').get('int')}; "
                f"release_pressed_on_close "
                f"{option(ctx.tier, 'input:virtualkeyboard:release_pressed_on_close')}")
    return delay


@implementation("input.hyprland_killed_type_guard_releases_keys")
def killed_type_guard_releases_keys(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    delay_ms = _repeat_delay(ctx)
    with _focused_key_log(ctx, LABEL) as (app, log, ref):
        shown = desktop(tier, main_instance(tier))["workspace"]
        with bystander_log(tier, app.pid, shown) as (bystander, bystander_path):
            ctx.before("applog.held_keys", held_keys(records(log)))
            scene = (app, log, ref, bystander, bystander_path, delay_ms)
            problems = []
            for mode, kind in ROUNDS:
                problems += _round(ctx, scene, mode, kind)
                if not raw.poll(lambda: keyboards(tier) == before, 3.0):
                    problems.append(f"{mode} {kind}: the keyboards changed: {before} to {keyboards(tier)}")
            ctx.after("hyprland.keyboards", keyboards(tier))
            ctx.after("applog.held_keys", held_keys(records(log)))
            ctx.restored("applog.held_keys", held_keys(records(log)))
            require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))
