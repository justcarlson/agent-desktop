"""Capture after compositor loss, on the nested second instance of the attached `hyprland` tier.

The nested instance holds no part of the user's desktop, so the leg can stop it. A painted foot
window on it is captured once, which proves the instance renders. The outer session shows the
nested window for that capture, since a hidden window gets no frame callbacks and the nested
instance renders nothing. The instance is then stopped by
its exact recorded pid, once through the record's own teardown (SIGTERM, so its sockets go) and once
by SIGKILL (so its sockets stay and nobody serves them). Each later capture of the same window id
must refuse with a typed code, not_delivered, exit 1 and within a budget, and return no frame and
write no file.
"""

from __future__ import annotations

import contextlib
import os
import signal
from pathlib import Path

from oracle import atspi_raw as raw, proc
from runner import apps, second_instance
from runner.apps import OwnedApp
from runner.helper import require
from runner.verdict import LegContext
from scenarios.capture import decode
from scenarios.hyprland_capture_common import (capture_path, colour, dominant, ids_by_pid, near, painter_command,
                                               written)
from scenarios.hyprland_common import bystander, desktop, instance_env, instance_query, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_focus_controls import nested_frames
from scenarios.hyprland_pointer_pad import NATIVE, pad_env
from scenarios.hyprland_toplevel import terminal_argv
from scenarios.hyprland_windows import cursor, restore_desktop
from scenarios.wayland_outputs import nested, nested_running

APP_ID = "ad-e2e-cap-loss"
PAINT = "a05000"
SHARE = 0.9
LOSSES = ("stopped", "killed")
REFUSAL_CODES = ("PLATFORM_NOT_SUPPORTED", "WINDOW_NOT_FOUND")
REFUSAL_BUDGET_MS = 3000.0
STOP_S = 5.0


def _new_client(tier, instance: dict, app_id: str, known: set[str]) -> dict | None:
    found = [item for item in instance_query(tier, instance, "clients")
             if item.get("class") == app_id and item.get("address") not in known]
    require(len(found) <= 1, f"several new {app_id} clients appeared on the nested instance: {found}")
    return found[0] if found and found[0].get("mapped") else None


@contextlib.contextmanager
def nested_painted(tier, instance: dict, app_id: str, value: str):
    """A foot window on `instance` painted `value`, started in the run's slice; yields it and its client."""
    from runner import attached_session
    log = tier.runtime_dir / f"painter-{app_id}.log"
    log.unlink(missing_ok=True)
    known = {item.get("address") for item in instance_query(tier, instance, "clients")}
    attached_session.spawn(tier, terminal_argv(app_id, painter_command(log, "static", value)),
                           pad_env(tier, instance, NATIVE), tier.runtime_dir / f"{app_id}-launch.log")
    found = raw.poll(lambda: _new_client(tier, instance, app_id, known), 10.0)
    require(found, f"the {app_id} window never mapped on the nested instance")
    pid = found["pid"]
    start = proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]
    require(proc.owned(tier, pid, start, {}), f"the {app_id} window is not in the run's slice")
    owned = OwnedApp(app_id, pid, start, [], [], "")
    apps._TIERS[pid, start] = tier
    try:
        require(raw.poll(lambda: written(log), 5.0), f"{app_id} never painted")
        yield owned, found
    finally:
        apps.stop(owned)
        apps._TIERS.pop((pid, start), None)


def lose(tier, instance: dict, how: str) -> None:
    """Stop the recorded nested instance by its exact pid and start time, and wait until it is gone."""
    from runner import attached
    if how == "stopped":
        found = second_instance.current(attached.load(tier.runtime_dir))
        require(found is not None and (found["pid"], found["start"]) == (instance["pid"], instance["start"]),
                f"the record's live instance {found} is not the one the leg captured")
        second_instance.stop(tier)
    else:
        require(nested_running(instance), "the nested instance is not the recorded process")
        os.kill(instance["pid"], signal.SIGKILL)
    require(raw.poll(lambda: not nested_running(instance), STOP_S), f"the nested instance survived {how}")


def rendered_problems(label: str, image) -> list[str]:
    found, share = dominant(image)
    if not near(found, colour(PAINT)) or share < SHARE:
        return [f"{label}: the live capture shows {found} at {share:.2f}, not #{PAINT}"]
    return []


def loss_problems(label: str, envelope, path: Path) -> list[str]:
    """A typed not_delivered refusal within the budget, with no frame and no file."""
    problems = []
    if envelope.ok or envelope.data:
        problems.append(f"{label}: a capture after the loss returned a frame: {envelope.raw[:300]}")
    problems += boundary_problems(label, envelope, REFUSAL_CODES)
    if envelope.elapsed_ms > REFUSAL_BUDGET_MS:
        problems.append(f"{label}: the refusal took {envelope.elapsed_ms:.0f} ms")
    if path.exists():
        problems.append(f"{label}: a capture after the loss wrote {path.name}")
    return problems


def _live_capture(ctx, env: dict[str, str], window_id: str, how: str) -> None:
    path = capture_path(ctx.tier, f"loss-{how}-live")
    envelope = ctx.product(["screenshot", "--window-id", window_id, str(path)], env=env, headed=False)
    require(envelope.ok and path.exists(),
            f"{how}: the nested instance did not render; the live capture failed: {envelope.raw[:400]}")
    try:
        image = decode(path.read_bytes(), "png")
    finally:
        path.unlink(missing_ok=True)
    problems = rendered_problems(how, image)
    require(not problems, f"the nested instance did not render: {'; '.join(problems)}")


def _round(ctx, how: str) -> list[str]:
    tier = ctx.tier
    instance, _ = nested(tier)
    env = instance_env(tier, instance)
    with nested_painted(tier, instance, f"{APP_ID}-{how}", PAINT) as (_, found):
        listed = ctx.product(["list-windows"], env=env, headed=False)
        window_id = ids_by_pid(listed, [found["pid"]])[found["pid"]]
        with nested_frames(ctx, instance):
            _live_capture(ctx, env, window_id, how)
        lose(tier, instance, how)
        path = capture_path(tier, f"loss-{how}-lost")
        envelope = ctx.product(["screenshot", "--window-id", window_id, str(path)], env=env, headed=False)
        problems = loss_problems(how, envelope, path)
        path.unlink(missing_ok=True)
    error = envelope.error or {}
    ctx.finding(f"{how}: {error.get('code')} {(error.get('details') or {}).get('kind')} "
                f"in {envelope.elapsed_ms:.0f} ms")
    return problems


def leftovers(tier) -> list[str]:
    return sorted(path.name for path in tier.runtime_dir.glob("capture-loss-*.png"))


def compositor_loss_refused(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    ctx.before("proc.capture_files", leftovers(tier))
    try:
        problems = [problem for how in LOSSES for problem in _round(ctx, how)]
        ctx.after("hyprland.desktop", read())
        ctx.after("proc.capture_files", leftovers(tier))
        problems += keeps_focus(tier, read())
    finally:
        second_instance.stop(tier)
        restore_desktop(tier, holder, point)
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())
    ctx.restored("proc.capture_files", leftovers(tier))
