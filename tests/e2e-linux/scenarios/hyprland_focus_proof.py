"""Focus proof before keys and clicks on the attached `hyprland` tier.

A headless command never activates a window: it proves that the window
already holds focus or it refuses. A headed command activates the window and
then needs every focus witness to agree before any key or click: the
compositor's active window, the toplevel's activated state, a workspace in
view and the frame's ACTIVE state. Each leg reads those witnesses itself,
`j/activewindow` from the compositor and ACTIVE from the raw accessibility
tree, and reads delivery from the app's own log, never from the command's
`ok`. A key refused at the proof reports `not_delivered`, because no key
started, and names the activation it sent and any cursor it moved. A headed
click on a native Wayland window refuses before any activation, so the click
half of the proof runs on the same app under XWayland. Focus, cursor and
workspaces go back to the bystander.
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
from scenarios.hyprland_pointer_pad import XWAYLAND, backend_problem, x_display

APP_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "inactive_frame_log.py"
APP_CLASS = "ad-inactive-frame"
APP_NAMES = ("inactive frame", APP_CLASS)
HIDDEN = "special:ad-e2e-focus-hidden"
SETTLE_S = 10.0
NATIVE = "wayland"
DEADLINE_MS = 5000
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
    if "the activation was sent" not in (error.get("message") or ""):
        problems.append(f"{label}: the message {error.get('message')!r} does not say the activation was sent")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: disposition {error.get('disposition')} claims the input may have landed")
    return problems


def side_effect_problems(label: str, envelope, before: dict, after: dict) -> list[str]:
    """A cursor the activation moved shows in the report's side effects."""
    if (before.get("x"), before.get("y")) == (after.get("x"), after.get("y")):
        return []
    effects = (((envelope.error or {}).get("details") or {}).get("side_effects")) or []
    if any(item.get("kind") == "cursor_moved" for item in effects):
        return []
    return [f"{label}: the cursor moved from {before} to {after} and the report names no cursor_moved"]


def unactivated_problems(label: str, envelope, cause: str) -> list[str]:
    """A click refused before the focus proof names its cause and sends neither activation nor input."""
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != "ACTION_FAILED" or details.get("cause") != cause:
        problems.append(f"{label}: expected ACTION_FAILED with cause {cause}, got {envelope.raw[:400]}")
    if details.get("activation_sent"):
        problems.append(f"{label}: an activation was sent before the refusal")
    if details.get("physical_delivery_started") is not False:
        problems.append(f"{label}: physical_delivery_started is {details.get('physical_delivery_started')!r}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: disposition {error.get('disposition')}")
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


def _owner(tier, names: tuple[str, ...]):
    found = [app for app in apps._owners(tier, names=names) if len(app.windows) == 1]
    return found[0] if len(found) == 1 else None


@contextlib.contextmanager
def spawned(tier, argv: list[str], names: tuple[str, ...], executable: str, extra: dict[str, str],
            toplevels: tuple[str, int] | None = None):
    """A one-window Python app run by a private interpreter named `executable`, registered to the run."""
    require(not apps._owners(tier, names=names), f"{executable} from an earlier leg still runs")
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    env.update(extra)
    python = shutil.which("python3", path=env.get("PATH"))
    require(python, "no python3 on the session PATH")
    interpreter = private_interpreter(tier.runtime_dir, Path(python).resolve(), executable)
    attached_session.spawn(tier, [str(interpreter), *argv], env, tier.runtime_dir / f"{executable}.log",
                           toplevels=toplevels)
    with raw.bounded_until(time.monotonic() + SETTLE_S):
        owner = raw.poll(lambda: _owner(tier, names), SETTLE_S)
    require(owner, f"{executable} never registered its window")
    owned = _register(tier, owner)
    try:
        yield owned
    finally:
        _release(tier, owned)


@contextlib.contextmanager
def inactive_frame(tier, flag: Path, backend: str = NATIVE):
    """The inactive frame app on native Wayland or on XWayland. Yields the app and its log path."""
    extra = {}
    if backend == XWAYLAND:
        display = raw.poll(lambda: x_display(main_instance(tier)), SETTLE_S)
        require(display, "no XWayland display serves the attached compositor")
        extra["DISPLAY"] = display
    log = tier.runtime_dir / f"inactive-frame-{backend}.jsonl"
    log.write_text("")
    toplevels = (APP_CLASS, 1) if backend == NATIVE else None
    with spawned(tier, [str(APP_SCRIPT), str(log), str(flag), backend], APP_NAMES, APP_CLASS, extra,
                 toplevels) as owned:
        yield owned, log


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


def _withheld_press(ctx: LegContext, app, log, target: dict) -> list[str]:
    """A headed press on a window whose frame never reports ACTIVE sends the activation and no key,
    and answers within the press deadline."""
    tier, label = ctx.tier, "headed press"
    before = cursor(tier)
    envelope = ctx.product(["press", "x", "--window-id", app.windows[0]])
    problems = activation_report_problems(label, envelope, "frame_not_active")
    problems += deadline_problems(label, envelope, DEADLINE_MS)
    problems += side_effect_problems(label, envelope, before, cursor(tier))
    if active(tier).get("address") != target["address"]:
        problems.append(f"{label}: j/activewindow names {active(tier).get('address')}, so no activation shows")
    if frame_active(tier, app):
        problems.append(f"{label}: the raw frame reports ACTIVE, so the witnesses agree")
    if presses(log) or entry_text(tier, app):
        problems.append(f"{label}: keys reached the app: {presses(log)}, entry {entry_text(tier, app)!r}")
    return problems


def _buttons(log) -> list[dict]:
    return [item for item in records(log) if item.get("event") == "button"]


def _native_click(ctx: LegContext, app, log, ref: str, label: str) -> list[str]:
    """A headed ref click on the native Wayland window refuses before any activation and logs no button."""
    holder = active(ctx.tier).get("address")
    problems = unactivated_problems(label, ctx.product(["click", ref]), "native_wayland_client")
    if active(ctx.tier).get("address") != holder:
        problems.append(f"{label}: j/activewindow moved from {holder} to {active(ctx.tier).get('address')}")
    if _buttons(log):
        problems.append(f"{label}: the app logged buttons {_buttons(log)}")
    return problems


def _native_keys(ctx: LegContext, flag: Path, desk) -> list[str]:
    """Withheld ACTIVE refuses the key; agreeing witnesses let the same key through."""
    tier, (read, holder, point) = ctx.tier, desk
    with inactive_frame(tier, flag) as (app, log):
        target = toplevel(tier, app)
        move(tier, target["address"], read()["workspace"])
        restore_desktop(tier, holder, point)
        ref = entry_ref(ctx, app)
        flag.touch()
        try:
            problems = _withheld_press(ctx, app, log, target)
            restore_desktop(tier, holder, point)
            problems += _native_click(ctx, app, log, ref, "headed ref click")
        finally:
            flag.unlink(missing_ok=True)
        restore_desktop(tier, holder, point)
        pressed = ctx.product(["press", "x", "--window-id", app.windows[0]])
        if not pressed.ok:
            problems.append(f"headed press with agreeing proof: {pressed.raw[:400]}")
        if not raw.poll(lambda: entry_text(tier, app) == "x", 4.0):
            problems.append(f"with agreeing proof the entry reads {entry_text(tier, app)!r}, not 'x'")
        problems += _witnesses(tier, app, target, "agreeing proof")
        restore_desktop(tier, holder, point)
        problems += _native_click(ctx, app, log, ref, "headed ref click with agreeing proof")
    return problems


def _xwayland_clicks(ctx: LegContext, flag: Path, desk) -> list[str]:
    """Withheld ACTIVE refuses the click after the activation; agreeing witnesses let it through."""
    tier, (read, holder, point) = ctx.tier, desk
    with inactive_frame(tier, flag, XWAYLAND) as (app, log):
        target = toplevel(tier, app)
        problems = [f"xwayland: {problem}" for problem in [backend_problem(target, XWAYLAND)] if problem]
        move(tier, target["address"], read()["workspace"])
        restore_desktop(tier, holder, point)
        ref = entry_ref(ctx, app)
        flag.touch()
        try:
            label, before = "xwayland headed ref click", cursor(tier)
            click = ctx.product(["click", ref])
            problems += activation_report_problems(label, click, "frame_not_active")
            problems += side_effect_problems(label, click, before, cursor(tier))
            if frame_active(tier, app):
                problems.append(f"{label}: the raw frame reports ACTIVE, so the witnesses agree")
            if _buttons(log):
                problems.append(f"{label}: the app logged buttons {_buttons(log)}")
        finally:
            flag.unlink(missing_ok=True)
        restore_desktop(tier, holder, point)
        clicked = ctx.product(["click", ref])
        if not clicked.ok:
            problems.append(f"xwayland headed ref click with agreeing proof: {clicked.raw[:400]}")
        if not raw.poll(lambda: _buttons(log), 4.0):
            problems.append("xwayland headed ref click with agreeing proof: the app logged no button")
        problems += _witnesses(tier, app, target, "xwayland agreeing proof")
    return problems


@implementation("focus.contradictory_or_missing_proof_refused")
def contradictory_or_missing_proof_refused(ctx: LegContext) -> None:
    flag = ctx.tier.runtime_dir / "inactive-frame.withhold"
    flag.unlink(missing_ok=True)
    with _bystander_desktop(ctx) as desk:
        problems = _native_keys(ctx, flag, desk)
        problems += _xwayland_clicks(ctx, flag, desk)
        require(not problems, "; ".join(problems))
