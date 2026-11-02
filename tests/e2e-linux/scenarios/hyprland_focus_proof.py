"""Focus proof before keys and clicks on the attached `hyprland` tier.

A headless command never activates a window: it proves that the window
already holds focus or it refuses. A headed command activates the window and
then needs every focus witness to agree before any key or click: the
compositor's active window, the toplevel's activated state, a workspace in
view and the frame's ACTIVE state. Each leg reads those witnesses itself,
`j/activewindow` from the compositor and ACTIVE from the raw accessibility
tree, and reads delivery from the app's own log, never from the command's
`ok`. Focus, cursor and workspaces go back to the bystander.
"""

from __future__ import annotations

import contextlib
import shutil
import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner import apps, attached_session
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_close import _register, _release, private_interpreter
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_focus_controls import active, cursor, move, restore_desktop, shown
from scenarios.hyprland_keys_common import entry_ref, entry_text, give_focus, key_log, presses, records, toplevel

APP_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "inactive_frame_log.py"
APP_CLASS = "ad-inactive-frame"
APP_NAMES = ("inactive frame", APP_CLASS)
HIDDEN = "special:ad-e2e-focus-hidden"
SETTLE_S = 10.0
DEADLINE_MS = 1500
DEADLINE_SLACK_MS = 1000
UNFOCUSED = ("STALE_REF",)
UNFOCUSED_CAUSE = "exact_window_focus_unconfirmed"


def existing_focus_problems(envelope) -> list[str]:
    """A headless focus-window on the focused window proves focus and activates nothing."""
    activation = (envelope.data or {}).get("activation") if envelope.ok else None
    if not isinstance(activation, dict):
        return [f"headless focus-window on the focused window: {envelope.raw[:400]}"]
    if activation.get("activated") is not False:
        return [f"headless focus-window on the focused window reports activation {activation}"]
    return []


def agreement_problems(label: str, target: dict, now_active: dict, in_view: bool, frame_active: bool) -> list[str]:
    """Every witness the leg reads itself names the target as focused."""
    problems = []
    if now_active.get("address") != target["address"] or now_active.get("pid") != target["pid"]:
        problems.append(f"{label}: j/activewindow names {now_active.get('address')}, not {target['address']}")
    if not in_view:
        problems.append(f"{label}: no monitor shows the target's workspace")
    if not frame_active:
        problems.append(f"{label}: the raw frame does not report ACTIVE")
    return problems


def activation_report_problems(label: str, envelope, cause: str) -> list[str]:
    """A refused headed command reports the activation it already sent and never claims no side effect."""
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != "ACTION_FAILED" or envelope.exit_code != 1:
        problems.append(f"{label}: expected ACTION_FAILED and exit 1, got {envelope.raw[:400]}")
    if details.get("cause") != cause:
        problems.append(f"{label}: cause is {details.get('cause')!r}, not {cause!r}")
    if details.get("activation_sent") is not True:
        problems.append(f"{label}: activation_sent is {details.get('activation_sent')!r}, not true")
    if details.get("physical_delivery_started") is not False:
        problems.append(f"{label}: physical_delivery_started is {details.get('physical_delivery_started')!r}")
    if (error.get("disposition") or {}).get("delivery") != "delivered_unverified":
        problems.append(f"{label}: disposition {error.get('disposition')} hides the activation it sent")
    return problems


def deadline_problems(label: str, envelope, timeout_ms: int) -> list[str]:
    limit = timeout_ms + DEADLINE_SLACK_MS
    if envelope.elapsed_ms > limit:
        return [f"{label}: answered after {envelope.elapsed_ms:.0f} ms, past {limit} ms"]
    return []


def frame_active(tier, app) -> bool:
    """Whether the app's frame reports ACTIVE on the raw accessibility tree."""
    frames = [frame for frame in raw.app_root(tier, app.owner_names[0]).children()
              if apps.window_id(frame) in app.windows]
    require(len(frames) == 1, f"{app.name} has {len(frames)} frames for its window")
    return "ACTIVE" in frames[0].states


def _in_view(tier, workspace: str) -> bool:
    return any(workspace in pair for pair in shown(tier).values())


def _witnesses(tier, app, target: dict, label: str) -> list[str]:
    workspace = (toplevel(tier, app).get("workspace") or {}).get("name") or ""
    return agreement_problems(label, target, active(tier), _in_view(tier, workspace), frame_active(tier, app))


def _owner(tier):
    found = [app for app in apps._owners(tier, names=APP_NAMES) if len(app.windows) == 1]
    return found[0] if len(found) == 1 else None


@contextlib.contextmanager
def inactive_frame(tier, flag: Path):
    """The inactive frame app, registered to the run. Yields the app and its log path."""
    require(not apps._owners(tier, names=APP_NAMES), "an inactive frame app from an earlier leg still runs")
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    python = shutil.which("python3", path=env.get("PATH"))
    require(python, "no python3 on the session PATH")
    interpreter = private_interpreter(tier.runtime_dir, Path(python).resolve(), APP_CLASS)
    log = tier.runtime_dir / "inactive-frame.jsonl"
    log.write_text("")
    attached_session.spawn(tier, [str(interpreter), str(APP_SCRIPT), str(log), str(flag)], env,
                           tier.runtime_dir / "inactive-frame.log", toplevels=(APP_CLASS, 1))
    with raw.bounded_until(time.monotonic() + SETTLE_S):
        owner = raw.poll(lambda: _owner(tier), SETTLE_S)
    require(owner, "the inactive frame app never registered its window")
    owned = _register(tier, owner)
    try:
        yield owned, log
    finally:
        _release(tier, owned)


@contextlib.contextmanager
def _bystander_desktop(ctx: LegContext):
    """Record the bystander's desktop before and after, and give it back whatever the leg did."""
    tier = ctx.tier
    read = lambda: desktop(tier, main_instance(tier))
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        yield read, holder, point
        restore_desktop(tier, holder, point)
        ctx.after("hyprland.desktop", read())
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _refused_headless(ctx: LegContext, app, log, label: str, cause: str, workspace: str | None) -> list[str]:
    """Headless focus-window and press refuse, no key reaches the app, and the desktop holds still."""
    tier, window = ctx.tier, app.windows[0]
    read = lambda: desktop(tier, main_instance(tier))
    keys_before, text_before = len(presses(log)), entry_text(tier, app)
    with Sampler({"desktop": read}) as sampler:
        focus = ctx.product(["focus-window", "--window-id", window])
        press = ctx.product(["press", "x", "--window-id", window])
    problems = boundary_problems(f"{label} focus-window", focus, ("POLICY_DENIED",), physical=True, cause=cause)
    named = ((focus.error or {}).get("details") or {}).get("workspace")
    if named != workspace:
        problems.append(f"{label} focus-window: details.workspace is {named!r}, not {workspace!r}")
    problems += boundary_problems(f"{label} press", press, UNFOCUSED, physical=True, cause=UNFOCUSED_CAUSE)
    if len(presses(log)) != keys_before or entry_text(tier, app) != text_before:
        problems.append(f"{label}: the app logged keys {presses(log)[keys_before:]}")
    return problems + [f"{label}: {change}" for change in sampler.changes]


@implementation("focus.headless_existing_focus_or_refusal")
def headless_existing_focus_or_refusal(ctx: LegContext) -> None:
    tier = ctx.tier
    with _bystander_desktop(ctx) as (read, holder, point), key_log(tier, "proof") as (app, log):
        target = toplevel(tier, app)
        address, window, workspace = target["address"], app.windows[0], read()["workspace"]
        move(tier, address, workspace)
        restore_desktop(tier, holder, point)
        problems = _refused_headless(ctx, app, log, "inactive", "window_not_focused", None)
        problems += keeps_focus(tier, read())
        move(tier, address, HIDDEN)
        problems += _refused_headless(ctx, app, log, "hidden", "hidden_special_workspace", HIDDEN)
        if any(special == HIDDEN for _, special in shown(tier).values()):
            problems.append(f"the hidden special workspace was shown: {shown(tier)}")
        move(tier, address, workspace)
        give_focus(tier, address)
        problems += _witnesses(tier, app, target, "focused")
        with Sampler({"desktop": read}) as sampler:
            problems += existing_focus_problems(ctx.product(["focus-window", "--window-id", window]))
            pressed = ctx.product(["press", "x", "--window-id", window])
        if not pressed.ok:
            problems.append(f"headless press on the focused window: {pressed.raw[:400]}")
        if not raw.poll(lambda: entry_text(tier, app) == "x", 4.0):
            problems.append(f"the focused entry reads {entry_text(tier, app)!r}, not 'x'")
        problems += [f"focused: {change}" for change in sampler.changes]
        move(tier, address, HIDDEN)
        faked = active(tier)
        problems += [f"hidden while focused (j/activewindow {faked.get('address')}): {problem}" for problem in
                     _refused_headless(ctx, app, log, "hidden while focused", "hidden_special_workspace", HIDDEN)]
        require(not problems, "; ".join(problems))


def _withheld_press(ctx: LegContext, app, log, target: dict, timeout_ms: int | None) -> list[str]:
    """A headed press on a window whose frame never reports ACTIVE sends the activation and no key."""
    tier = ctx.tier
    label = "headed press" + (f" --timeout-ms {timeout_ms}" if timeout_ms else "")
    argv = ["press", "x", "--window-id", app.windows[0]] + (["--timeout-ms", str(timeout_ms)] if timeout_ms else [])
    envelope = ctx.product(argv)
    problems = activation_report_problems(label, envelope, "frame_not_active")
    if timeout_ms:
        problems += deadline_problems(label, envelope, timeout_ms)
    if active(tier).get("address") != target["address"]:
        problems.append(f"{label}: j/activewindow names {active(tier).get('address')}, so no activation shows")
    if frame_active(tier, app):
        problems.append(f"{label}: the raw frame reports ACTIVE, so the witnesses agree")
    if presses(log) or entry_text(tier, app):
        problems.append(f"{label}: keys reached the app: {presses(log)}, entry {entry_text(tier, app)!r}")
    return problems


def _buttons(log) -> list[dict]:
    return [item for item in records(log) if item.get("event") == "button"]


@implementation("focus.contradictory_or_missing_proof_refused")
def contradictory_or_missing_proof_refused(ctx: LegContext) -> None:
    tier = ctx.tier
    flag = tier.runtime_dir / "inactive-frame.withhold"
    flag.unlink(missing_ok=True)
    with _bystander_desktop(ctx) as (read, holder, point), inactive_frame(tier, flag) as (app, log):
        target = toplevel(tier, app)
        move(tier, target["address"], read()["workspace"])
        restore_desktop(tier, holder, point)
        ref = entry_ref(ctx, app)
        flag.touch()
        try:
            problems = _withheld_press(ctx, app, log, target, None)
            restore_desktop(tier, holder, point)
            problems += _withheld_press(ctx, app, log, target, DEADLINE_MS)
            restore_desktop(tier, holder, point)
            click = ctx.product(["click", ref])
            problems += activation_report_problems("headed ref click", click, "frame_not_active")
            if _buttons(log):
                problems.append(f"headed ref click: the app logged buttons {_buttons(log)}")
        finally:
            flag.unlink(missing_ok=True)
        restore_desktop(tier, holder, point)
        pressed = ctx.product(["press", "x", "--window-id", app.windows[0]])
        if not pressed.ok:
            problems.append(f"headed press with agreeing proof: {pressed.raw[:400]}")
        if not raw.poll(lambda: entry_text(tier, app) == "x", 4.0):
            problems.append(f"with agreeing proof the entry reads {entry_text(tier, app)!r}, not 'x'")
        problems += _witnesses(tier, app, target, "agreeing proof")
        clicked = ctx.product(["click", ref])
        problems += boundary_problems("headed ref click with agreeing proof", clicked, ("ACTION_FAILED",),
                                      physical=True, cause="native_wayland_client")
        if _buttons(log):
            problems.append(f"headed ref click with agreeing proof: the app logged buttons {_buttons(log)}")
        require(not problems, "; ".join(problems))
