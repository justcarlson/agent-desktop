"""Capture refusals on the attached `hyprland` tier.

With `ecosystem:enforce_permissions` on, Hyprland withholds capture from an
unlisted client without telling it, so the product refuses with PERM_DENIED
before it asks, and no dialog, portal or layer appears. A capture whose
budget ends mid-frame answers TIMEOUT, and a capture killed mid-frame leaves
no file; neither leaves the compositor unable to capture again.
"""

from __future__ import annotations

import json
import signal
import subprocess
import time
from pathlib import Path

from runner import product
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_capture_common import capture_path, ids_by_pid, option, painted, saved_image
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import NOT_DELIVERED, boundary_problems, keeps_focus
from scenarios.hyprland_windows import control, cursor, query, restore_desktop

ENFORCE = "ecosystem:enforce_permissions"
ASKERS = ("portal", "dialog", "guiutils", "polkit")
REFUSAL_BUDGET_MS = 1000.0
BATCH_BUDGETS_MS = (10, 20, 30, 40, 50, 60, 80, 250, 1000)
CANCEL_DELAYS_S = (0.005, 0.02, 0.05)
COMMAND_BUDGET_MS = 5000.0


def enforce_argv(on: bool, lua: bool) -> list[str]:
    value = "true" if on else "false"
    if lua:
        return ["eval", f"hl.config({{ ecosystem = {{ enforce_permissions = {value} }} }})"]
    return ["keyword", ENFORCE, value]


def enforced(tier) -> bool:
    found = option(tier, ENFORCE)
    return bool(found.get("bool", found.get("int")))


def askers(proc_root: Path = Path("/proc")) -> set[tuple[int, str]]:
    """Every process whose name says it could ask a person something."""
    found = set()
    for directory in proc_root.glob("[0-9]*"):
        try:
            name = (directory / "comm").read_text().strip()
        except OSError:
            continue
        if any(word in name.lower() for word in ASKERS):
            found.add((int(directory.name), name))
    return found


def layers(tier) -> list[str]:
    return sorted(str(layer.get("namespace")) for monitor in query(tier, "layers").values()
                  for level in monitor.get("levels", {}).values() for layer in level)


def permission_problems(label: str, envelope, path: Path) -> list[str]:
    problems = boundary_problems(label, envelope, ("PERM_DENIED",))
    details = (envelope.error or {}).get("details") or {}
    if details.get("option") != ENFORCE or details.get("enforced") is not True:
        problems.append(f"{label}: details {details} do not name {ENFORCE}")
    if envelope.elapsed_ms > REFUSAL_BUDGET_MS:
        problems.append(f"{label}: the refusal took {envelope.elapsed_ms:.0f} ms")
    if path.exists():
        problems.append(f"{label}: a refused capture wrote an image")
    return problems


@implementation("capture.hyprland_permission_denied_no_dialog")
def permission_denied_no_dialog(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    original = enforced(tier)
    ctx.before("hyprland.desktop", read())
    ctx.before("hyprland.enforce_permissions", original)
    try:
        with painted(tier, "ad-e2e-cap-denied", "static", "808000") as (_, found, _log):
            window_id = ids_by_pid(ctx.product(["list-windows"]), [found["pid"]])[found["pid"]]
            control(tier, lambda lua: enforce_argv(True, lua))
            require(enforced(tier), f"{ENFORCE} did not turn on")
            restore_desktop(tier, holder, point)
            quiet = {"askers": askers(), "layers": layers(tier)}
            with Sampler({"desktop": read, "askers": askers, "layers": lambda: layers(tier)}) as sampler:
                path = capture_path(tier, "denied")
                problems = permission_problems("window", ctx.product(
                    ["screenshot", "--window-id", window_id, str(path)]), path)
                problems += permission_problems("screen", ctx.product(["screenshot", "--screen", "0", str(path)]), path)
            ctx.after("hyprland.enforce_permissions", enforced(tier))
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            if askers() != quiet["askers"] or layers(tier) != quiet["layers"]:
                problems.append("a process or layer that could ask a person appeared")
            control(tier, lambda lua: enforce_argv(original, lua))
            path = capture_path(tier, "allowed")
            allowed = ctx.product(["screenshot", "--window-id", window_id, str(path)])
            if not allowed.ok:
                problems.append(f"capture did not work again once enforcement was off: {allowed.raw[:300]}")
            path.unlink(missing_ok=True)
            require(not problems, "; ".join(problems))
    finally:
        if enforced(tier) != original:
            control(tier, lambda lua: enforce_argv(original, lua))
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.enforce_permissions", enforced(tier))
    ctx.restored("hyprland.desktop", read())


def batch_entry_problems(budget_ms: int, envelope) -> tuple[list[str], str]:
    """One budget's batch: the entry captured, or answered TIMEOUT not delivered, within its budget."""
    label = f"batch {budget_ms} ms"
    results = (envelope.data or {}).get("results") or []
    if not envelope.ok or len(results) != 1:
        return [f"{label}: {envelope.raw[:300]}"], "broken"
    entry = results[0]
    problems = []
    if envelope.elapsed_ms > budget_ms + COMMAND_BUDGET_MS / 5:
        problems.append(f"{label}: took {envelope.elapsed_ms:.0f} ms")
    if entry.get("ok") is True:
        return problems, "captured"
    error = entry.get("error") or {}
    if error.get("code") != "TIMEOUT" or error.get("disposition") != NOT_DELIVERED:
        problems.append(f"{label}: expected a captured frame or TIMEOUT {NOT_DELIVERED}, got {error}")
        return problems, "broken"
    return problems, (error.get("details") or {}).get("kind") or "timeout"


def _deadlines(ctx, window_id: str) -> list[str]:
    outcomes, problems = [], []
    for budget in BATCH_BUDGETS_MS:
        path = capture_path(ctx.tier, f"deadline-{budget}")
        entries = json.dumps([{"command": "screenshot", "args": {"window_id": window_id, "output_path": str(path)}}])
        found, outcome = batch_entry_problems(budget, ctx.product(["batch", entries, "--timeout-ms", str(budget)]))
        problems += found
        if outcome != "captured" and path.exists():
            problems.append(f"batch {budget} ms: a timed-out capture wrote an image")
        path.unlink(missing_ok=True)
        outcomes.append((budget, outcome))
    if not any(outcome == "capture_frame" for _, outcome in outcomes):
        problems.append(f"no budget ended while the frame was awaited: {outcomes}")
    if not any(outcome == "captured" for _, outcome in outcomes):
        problems.append(f"no budget captured the window: {outcomes}")
    ctx.finding(f"batch budgets and outcomes: {outcomes}")
    return problems


def _cancelled(ctx, window_id: str) -> list[str]:
    problems = []
    for delay in CANCEL_DELAYS_S:
        path = capture_path(ctx.tier, f"cancel-{delay}")
        argv = [product._binary_path(), "screenshot", "--window-id", window_id, str(path)]
        running = subprocess.Popen(argv, env=product._result_env(ctx.tier), stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL)
        time.sleep(delay)
        running.send_signal(signal.SIGTERM)
        code = running.wait(timeout=COMMAND_BUDGET_MS / 1000 + 1)
        if code != -signal.SIGTERM and path.exists():
            path.unlink()
            continue
        if path.exists():
            problems.append(f"a capture cancelled after {delay} s left {path.name}")
            path.unlink()
    path = capture_path(ctx.tier, "after-cancel")
    envelope = ctx.product(["screenshot", "--window-id", window_id, str(path)])
    if not envelope.ok:
        problems.append(f"capture failed after cancelled captures: {envelope.raw[:300]}")
    else:
        saved_image(envelope, path)
    return problems


@implementation("capture.hyprland_deadline_or_cancel_released")
def deadline_or_cancel_released(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with painted(tier, "ad-e2e-cap-deadline", "static", "008080") as (_, found, _log):
            window_id = ids_by_pid(ctx.product(["list-windows"]), [found["pid"]])[found["pid"]]
            with Sampler({"desktop": read}) as sampler:
                problems = _deadlines(ctx, window_id) + _cancelled(ctx, window_id)
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())
