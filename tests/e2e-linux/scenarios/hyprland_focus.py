"""`focus-window` on the attached `hyprland` tier.

A headed command activates the bound toplevel and reports how focus was
proven and what else changed. A headless one proves existing focus or
refuses. Every claim is read back from the compositor's active window,
monitors and cursor, and for an accessible window from its AT-SPI frame.
The runner's own apps start on the hidden special workspace `ad_e2e`; each
leg puts the bystander's focus, the workspaces and the cursor back.
"""

from __future__ import annotations

from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from oracle import atspi_raw as raw
from scenarios.hyprland_windows import (
    SPARE_WORKSPACE, SPECIAL, active, client, clients_of, close_specials, cursor, focus, frame_active,
    listed, move, regular_workspaces, restore_desktop, specials, terminal, toggle_special,
)
from runner.attached_session import WORKSPACE

LAUNCH_SPECIAL = SPECIAL + WORKSPACE
SECOND_SPECIAL = LAUNCH_SPECIAL + "_b"


def activation_problems(label: str, envelope, *, window_id: str, activated: bool, proof: str,
                        effects: list[dict] = (), cursor_before=None, cursor_after=None) -> list[str]:
    """The success contract: the window, whether it was activated, its proof and exact side effects."""
    if not envelope.ok:
        return [f"{label}: expected success, got {envelope.raw[:400]}"]
    data = envelope.data or {}
    focused, activation = data.get("focused") or {}, data.get("activation") or {}
    problems = []
    if focused.get("id") != window_id or focused.get("is_focused") is not True:
        problems.append(f"{label}: focused is {focused}, not {window_id} with is_focused true")
    if activation.get("activated") is not activated:
        problems.append(f"{label}: activated is {activation.get('activated')!r}, not {activated}")
    if activation.get("proof") != proof:
        problems.append(f"{label}: proof is {activation.get('proof')!r}, not {proof!r}")
    reported = list(activation.get("side_effects") or [])
    moved = [item for item in reported if item.get("kind") == "cursor_moved"]
    if [item for item in reported if item.get("kind") != "cursor_moved"] != list(effects):
        problems.append(f"{label}: side effects are {reported}, expected {list(effects)} and any cursor move")
    if not activated and reported:
        problems.append(f"{label}: nothing was activated, yet side effects were reported: {reported}")
    point = lambda value: {"x": float(value["x"]), "y": float(value["y"])}
    if cursor_before is not None and cursor_after is not None:
        expected = ([] if cursor_before == cursor_after else
                    [{"kind": "cursor_moved", "from": point(cursor_before), "to": point(cursor_after)}])
        if moved != expected:
            problems.append(f"{label}: cursor effect is {moved}, the compositor moved it "
                            f"{cursor_before} -> {cursor_after}")
    return problems


def refusal(label: str, envelope, code: str, cause: str, **details) -> list[str]:
    problems = boundary_problems(label, envelope, (code,), physical=True, cause=cause)
    found = ((envelope.error or {}).get("details") or {})
    for key, value in details.items():
        if found.get(key) != value:
            problems.append(f"{label}: details.{key} is {found.get(key)!r}, not {value!r}")
    return problems


def _monitor(tier) -> str:
    names = list(specials(tier))
    require(len(names) == 1, f"the focus legs expect one monitor, found {names}")
    return names[0]


def _spare(tier) -> str:
    current = regular_workspaces(tier)[_monitor(tier)]
    return "8" if current == SPARE_WORKSPACE else SPARE_WORKSPACE


def _window_of(ctx, pid: int) -> dict:
    found = listed(ctx.product(["list-windows"]), pid=pid)
    require(len(found) == 1, f"expected one listed window for pid {pid}, found {found}")
    return found[0]


def _proof(entry: dict) -> str:
    return "compositor_and_accessibility" if entry.get("accessible") else "compositor_only"


def _calculator(ctx, app) -> dict:
    found = clients_of(ctx.tier, app.pid)
    require(len(found) == 1, f"Calculator has {len(found)} toplevels")
    return found[0]


@implementation("focus.hyprland_headless_proves_or_refuses")
def headless_proves_or_refuses(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder = bystander(tier)
    with managed(ctx, "Calculator") as app, terminal(tier, "ad-e2e-headless") as (_, foot):
        move(tier, foot["address"], _spare(tier))
        ctx.before("hyprland.desktop", read())
        ctx.before("atspi.frame_active", frame_active(tier, app))
        own = _window_of(ctx, holder["pid"])
        foot_id = _window_of(ctx, foot["pid"])["id"]
        with Sampler({"desktop": read}) as sampler:
            problems = activation_problems("focused bystander", ctx.product(
                ["focus-window", "--window-id", own["id"]]), window_id=own["id"], activated=False,
                proof=_proof(own))
            problems += refusal("hidden special", ctx.product(
                ["focus-window", "--window-id", app.windows[0]]), "POLICY_DENIED",
                "hidden_special_workspace", workspace=LAUNCH_SPECIAL)
            problems += refusal("unfocused terminal", ctx.product(
                ["focus-window", "--window-id", foot_id]), "POLICY_DENIED", "window_not_focused")
        ctx.after("hyprland.desktop", read())
        ctx.after("atspi.frame_active", frame_active(tier, app))
        problems += sampler.changes + keeps_focus(tier, read())
        ctx.restored("atspi.frame_active", frame_active(tier, app))
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())


@implementation("focus.hyprland_headed_accessible_window")
def headed_accessible_window(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    baseline = read()
    ctx.before("hyprland.desktop", baseline)
    try:
        with managed(ctx, "Calculator") as app:
            ctx.before("atspi.frame_active", frame_active(tier, app))
            calculator = _calculator(ctx, app)
            monitor = _monitor(tier)
            focused = ctx.product(["focus-window", "--window-id", app.windows[0]])
            after = cursor(tier)
            problems = activation_problems(
                "hidden Calculator", focused, window_id=app.windows[0], activated=True,
                proof="compositor_and_accessibility", cursor_before=point, cursor_after=after,
                effects=[{"kind": "special_workspace_revealed", "monitor": monitor,
                          "workspace": LAUNCH_SPECIAL}])
            ctx.after("hyprland.desktop", read())
            ctx.after("atspi.frame_active", frame_active(tier, app))
            if active(tier).get("address") != calculator["address"]:
                problems.append(f"the compositor's active window is {active(tier)}, not Calculator")
            if specials(tier).get(monitor) != LAUNCH_SPECIAL:
                problems.append(f"{LAUNCH_SPECIAL} is not open: {specials(tier)}")
            if not frame_active(tier, app):
                problems.append("the Calculator frame does not report ACTIVE")
            with Sampler({"desktop": read}) as sampler:
                problems += activation_problems(
                    "focused Calculator", ctx.product(["focus-window", "--window-id", app.windows[0]]),
                    window_id=app.windows[0], activated=False, proof="compositor_and_accessibility")
            problems += sampler.changes
            restore_desktop(tier, holder, point)
            ctx.restored("atspi.frame_active", frame_active(tier, app))
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


@implementation("focus.hyprland_special_workspace_open_refused")
def special_workspace_open_refused(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with managed(ctx, "Calculator") as app, terminal(tier, "ad-e2e-covered") as (_, foot):
            ctx.before("atspi.frame_active", frame_active(tier, app))
            move(tier, foot["address"], _spare(tier))
            foot_id = _window_of(ctx, foot["pid"])["id"]
            toggle_special(tier, LAUNCH_SPECIAL)
            monitor = _monitor(tier)
            require(specials(tier).get(monitor) == LAUNCH_SPECIAL, f"{LAUNCH_SPECIAL} did not open")
            opened = read()
            with Sampler({"desktop": read}) as sampler:
                problems = refusal("covered terminal", ctx.product(["focus-window", "--window-id", foot_id]),
                                   "ACTION_FAILED", "special_workspace_open", workspace=LAUNCH_SPECIAL,
                                   monitor=monitor)
            ctx.after("hyprland.desktop", read())
            ctx.after("atspi.frame_active", frame_active(tier, app))
            problems += sampler.changes
            if read() != opened:
                problems.append("the refused focus-window changed the desktop")
            restore_desktop(tier, holder, point)
            ctx.restored("atspi.frame_active", frame_active(tier, app))
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


@implementation("focus.hyprland_headed_compositor_only_window")
def headed_compositor_only_window(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with terminal(tier, "ad-e2e-headed") as (_, foot):
            monitor, spare = _monitor(tier), _spare(tier)
            start_workspace = regular_workspaces(tier)[monitor]
            move(tier, foot["address"], spare)
            foot_id = _window_of(ctx, foot["pid"])["id"]
            ctx.before("hyprland.terminal_active", active(tier).get("address") == foot["address"])
            problems = _switch_workspace(ctx, foot, foot_id, monitor, (start_workspace, spare), point)
            ctx.after("hyprland.terminal_active", active(tier).get("address") == foot["address"])
            problems += _hidden_while_active(ctx, foot, foot_id, monitor)
            problems += _close_and_reveal(ctx, (foot, holder), foot_id, monitor)
            ctx.after("hyprland.desktop", read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.terminal_active", active(tier).get("address") == foot["address"])
    ctx.restored("hyprland.desktop", read())


def _switch_workspace(ctx, foot: dict, foot_id: str, monitor: str, workspaces: tuple[str, str],
                      point: dict) -> list[str]:
    tier = ctx.tier
    focused = ctx.product(["focus-window", "--window-id", foot_id])
    problems = activation_problems(
        "terminal on another workspace", focused, window_id=foot_id, activated=True,
        proof="compositor_only", cursor_before=point, cursor_after=cursor(tier),
        effects=[{"kind": "workspace_switched", "monitor": monitor,
                  "from": workspaces[0], "to": workspaces[1]}])
    if active(tier).get("address") != foot["address"]:
        problems.append(f"the compositor's active window is {active(tier)}, not the terminal")
    if regular_workspaces(tier).get(monitor) != workspaces[1]:
        problems.append(f"the monitor shows {regular_workspaces(tier)}, not workspace {workspaces[1]}")
    problems += activation_problems("focused terminal", ctx.product(["focus-window", "--window-id", foot_id]),
                                    window_id=foot_id, activated=False, proof="compositor_only")
    return problems


def hidden_activation_problems(label: str, envelope, *, stale: bool, monitor: str) -> list[str]:
    """A window moved to a hidden special workspace is never proven focused without activation.

    Hyprland 0.56.2 keeps it active (`stale`) and ignores its activation, which
    the product reports as `window_hidden` after sending it; otherwise the
    activation reveals the special workspace.
    """
    if envelope.ok:
        activation = (envelope.data or {}).get("activation") or {}
        reported = [item for item in activation.get("side_effects") or [] if item.get("kind") != "cursor_moved"]
        expected = [{"kind": "special_workspace_revealed", "monitor": monitor, "workspace": SECOND_SPECIAL}]
        if activation.get("activated") is not True or reported != expected:
            return [f"{label}: activation {activation}, expected activated true revealing {SECOND_SPECIAL}"]
        return []
    error = envelope.error or {}
    details = error.get("details") or {}
    expected = {"cause": "window_hidden", "activation_sent": True, "in_view": False,
                "compositor_active": True, "physical_delivery_started": False}
    problems = [f"{label}: {key} is {details.get(key)!r}, not {value!r}"
                for key, value in expected.items() if details.get(key) != value]
    if error.get("code") != "ACTION_FAILED":
        problems.append(f"{label}: code is {error.get('code')!r}, not ACTION_FAILED")
    if error.get("disposition") != {"delivery": "delivered_unverified", "retry": "unsafe"}:
        problems.append(f"{label}: disposition is {error.get('disposition')}")
    if not stale:
        problems.append(f"{label}: refused although the compositor no longer reported the window active")
    return problems


def _hidden_while_active(ctx, foot: dict, foot_id: str, monitor: str) -> list[str]:
    tier = ctx.tier
    move(tier, foot["address"], SECOND_SPECIAL)
    stale = active(tier).get("address") == foot["address"]
    problems = hidden_activation_problems("terminal moved to a hidden special workspace",
                                          ctx.product(["focus-window", "--window-id", foot_id]),
                                          stale=stale, monitor=monitor)
    close_specials(tier)
    return problems


def _close_and_reveal(ctx, windows: tuple[dict, dict], foot_id: str, monitor: str) -> list[str]:
    tier, (foot, holder) = ctx.tier, windows
    focus(tier, holder["address"])
    require(raw.poll(lambda: active(tier).get("address") == holder["address"], 3.0),
            "the bystander did not take focus")
    require(client(tier, foot["address"])["workspace"]["name"] == SECOND_SPECIAL,
            f"the terminal left {SECOND_SPECIAL}")
    toggle_special(tier, LAUNCH_SPECIAL)
    require(raw.poll(lambda: specials(tier).get(monitor) == LAUNCH_SPECIAL, 3.0),
            f"{LAUNCH_SPECIAL} did not open")
    before = cursor(tier)
    focused = ctx.product(["focus-window", "--window-id", foot_id])
    problems = activation_problems(
        "terminal on a second special workspace", focused, window_id=foot_id, activated=True,
        proof="compositor_only", cursor_before=before, cursor_after=cursor(tier),
        effects=[{"kind": "special_workspace_closed", "monitor": monitor, "workspace": LAUNCH_SPECIAL},
                 {"kind": "special_workspace_revealed", "monitor": monitor, "workspace": SECOND_SPECIAL}])
    if specials(tier).get(monitor) != SECOND_SPECIAL:
        problems.append(f"{SECOND_SPECIAL} is not the open special workspace: {specials(tier)}")
    if active(tier).get("address") != foot["address"]:
        problems.append(f"the compositor's active window is {active(tier)}, not the terminal")
    return problems
