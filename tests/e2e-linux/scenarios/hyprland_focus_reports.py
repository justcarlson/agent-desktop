"""The report a headed `focus-window` gives on the attached `hyprland` tier, held to the compositor.

`data.activation` says whether an activation was sent, which observers proved the focus and every
side effect it caused. Each focus is bracketed by two reads of `j/monitors` and `j/cursorpos`, and
the workspace switches, special workspaces closed and revealed and the cursor move between them are
the side effects the report must name, no more and no fewer. `j/activewindow` must name the target
on a workspace a monitor shows. A Calculator frame must go from no raw AT-SPI `ACTIVE` to `ACTIVE`
and report `compositor_and_accessibility`; a foot terminal has no tree and must report
`compositor_only`. Each leg gives focus, the cursor and the workspaces back to the bystander.
"""

from __future__ import annotations

import json

from oracle import atspi_raw as raw
from runner import apps
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems
from scenarios.hyprland_focus_controls import (active, client, clients_of, cursor, listed_ids, move, query,
                                               restore_desktop, shown, spare_workspaces, terminal,
                                               workspaces_in_use)

FOCUS_SETTLE_S = 3.0
PROOF_BOTH = "compositor_and_accessibility"
PROOF_COMPOSITOR = "compositor_only"
VIEW_KINDS = ("workspace_switched", "special_workspace_closed", "special_workspace_revealed")
SPECIALS = ("special:ad-e2e-proof-a", "special:ad-e2e-proof-b")


def scene(tier) -> dict:
    """What a focus may change beyond focus itself: each monitor's workspaces and the cursor."""
    return {"monitors": query(tier, "monitors"), "cursor": cursor(tier)}


def _point(read: dict) -> dict:
    return {"x": float(read["x"]), "y": float(read["y"])}


def _special(item: dict) -> str:
    return (item.get("specialWorkspace") or {}).get("name") or ""


def expected_effects(before: dict, after: dict) -> list[dict]:
    """The side effects two compositor reads show, monitor by monitor, then the cursor move."""
    effects: list[dict] = []
    was = {item["name"]: item for item in before["monitors"]}
    for now in after["monitors"]:
        old = was.get(now["name"])
        if old is None:
            continue
        name, regular = now["name"], now["activeWorkspace"]["name"]
        if old["activeWorkspace"]["name"] != regular:
            effects.append({"kind": "workspace_switched", "monitor": name,
                            "from": old["activeWorkspace"]["name"], "to": regular})
        if _special(old) == _special(now):
            continue
        if _special(old):
            effects.append({"kind": "special_workspace_closed", "monitor": name, "workspace": _special(old)})
        if _special(now):
            effects.append({"kind": "special_workspace_revealed", "monitor": name, "workspace": _special(now)})
    if before.get("cursor") and after.get("cursor") and _point(before["cursor"]) != _point(after["cursor"]):
        effects.append({"kind": "cursor_moved", "from": _point(before["cursor"]), "to": _point(after["cursor"])})
    return effects


def landed(effects: list[dict]) -> list[tuple[str, str]]:
    """Each workspace effect by its kind and the workspace it lands on, in report order."""
    return [(item["kind"], item.get("to") or item.get("workspace")) for item in effects
            if item.get("kind") in VIEW_KINDS]


def _canonical(effects: list[dict]) -> list[str]:
    def normal(item: dict) -> dict:
        return {key: _point(value) if key in ("from", "to") and isinstance(value, dict) else value
                for key, value in item.items()}
    return sorted(json.dumps(normal(item), sort_keys=True) for item in effects)


def activation_problems(label: str, envelope, proof: str, activated: bool, effects: list[dict]) -> list[str]:
    """The report names the proof, whether it activated, and exactly the effects the compositor showed."""
    if not envelope.ok:
        return [f"{label}: focus-window failed: {envelope.raw[:400]}"]
    activation = (envelope.data or {}).get("activation") or {}
    problems = []
    if activation.get("activated") is not activated:
        problems.append(f"{label}: activation.activated is {activation.get('activated')!r}, not {activated}")
    if activation.get("proof") != proof:
        problems.append(f"{label}: activation.proof is {activation.get('proof')!r}, not {proof!r}")
    reported = activation.get("side_effects") or []
    if _canonical(reported) != _canonical(effects):
        problems.append(f"{label}: side_effects {reported} are not the compositor's {effects}")
    return problems


def frame_problems(label: str, before: frozenset, after: frozenset) -> list[str]:
    """The raw AT-SPI frame was not ACTIVE before the focus and is after it."""
    problems = []
    if "ACTIVE" in before:
        problems.append(f"{label}: the frame was ACTIVE before focus-window, so its state proves nothing")
    if "ACTIVE" not in after:
        problems.append(f"{label}: the frame is not ACTIVE after focus-window: {sorted(after)}")
    return problems


def frame_states(tier, app, window_id: str) -> frozenset:
    """The raw AT-SPI states of the app's frame with this window id."""
    for frame in raw.app_root(tier, app.owner_names[0]).children():
        if apps.window_id(frame) == window_id:
            return frame.states
    raise AssertionError(f"{app.name} has no frame {window_id}")


def focus_view(tier, address: str) -> dict:
    """The active toplevel, the target's pid and workspace and what each monitor shows."""
    held = active(tier)
    toplevel = client(tier, address) or {}
    return {"active": held.get("address"), "active_pid": held.get("pid"), "pid": toplevel.get("pid"),
            "workspace": (toplevel.get("workspace") or {}).get("name"), "shown": shown(tier)}


def held_problems(label: str, view: dict, address: str, workspace: str) -> list[str]:
    """The target is the active toplevel, on its workspace, and a monitor shows that workspace uncovered."""
    problems = []
    if view.get("active") != address or view.get("active_pid") != view.get("pid"):
        problems.append(f"{label}: the compositor's active window is {view.get('active')}, not {address}")
    if view.get("workspace") != workspace:
        problems.append(f"{label}: the toplevel is on {view.get('workspace')!r}, not {workspace!r}")
    shows = view.get("shown") or {}
    if not any(special == workspace or (regular == workspace and not special) for regular, special in shows.values()):
        problems.append(f"{label}: no monitor shows {workspace!r} uncovered: {shows}")
    return problems


def special_open_problems(label: str, envelope, workspace: str) -> list[str]:
    """A window under an open special workspace is refused by that workspace's name, with nothing sent."""
    problems = boundary_problems(label, envelope, ("ACTION_FAILED",), physical=True, cause="special_workspace_open")
    named = ((envelope.error or {}).get("details") or {}).get("workspace")
    if named != workspace:
        problems.append(f"{label}: the refusal names {named!r}, not the open {workspace!r}")
    return problems


def focus_step(ctx: LegContext, label: str, target: tuple[str, str, str], proof: str,
               activated: bool = True) -> tuple[list[str], list[dict]]:
    """One headed focus-window on (window id, address, workspace), judged by the compositor. Returns the
    problems and the side effects the compositor showed."""
    window_id, address, workspace = target
    before = scene(ctx.tier)
    envelope = ctx.product(["focus-window", "--window-id", window_id])
    raw.poll(lambda: held_problems(label, focus_view(ctx.tier, address), address, workspace) == [], FOCUS_SETTLE_S)
    effects = expected_effects(before, scene(ctx.tier))
    problems = held_problems(label, focus_view(ctx.tier, address), address, workspace)
    return problems + activation_problems(label, envelope, proof, activated, effects), effects


def _view_problems(label: str, effects: list[dict], expected: list[tuple[str, str]]) -> list[str]:
    seen = landed(effects)
    return [] if sorted(seen) == sorted(expected) else [f"{label}: the compositor showed {seen}, not {expected}"]


def _ids(ctx: LegContext, apps_by_label: dict) -> dict[str, str]:
    listing = ctx.product(["list-windows"])
    found = {}
    for label, app in apps_by_label.items():
        ids = listed_ids(listing, app.pid)
        require(len(ids) == 1, f"list-windows gives the {label} terminal {ids}")
        found[label] = ids[0]
    return found


def _leg(ctx: LegContext, work) -> None:
    """Record the desktop around `work`, which returns its problems, and give it back after."""
    tier, main = ctx.tier, main_instance(ctx.tier)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", desktop(tier, main))
    try:
        problems = work(ctx, holder, point)
        ctx.after("hyprland.desktop", desktop(tier, main))
        require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", desktop(tier, main))


def _activation(ctx: LegContext, holder: dict, point: dict) -> list[str]:
    tier = ctx.tier
    with managed(ctx, "Calculator") as calculator, terminal(tier, "ad-e2e-proof-far") as (foot_app, foot):
        toplevels = clients_of(tier, calculator.pid)
        require(len(toplevels) == 1, f"Calculator has {len(toplevels)} toplevels")
        foot_id = _ids(ctx, {"far": foot_app})["far"]
        far = spare_workspaces(workspaces_in_use(tier), 2)
        calc = (calculator.windows[0], toplevels[0]["address"], far[0])
        term = (foot_id, foot["address"], far[1])
        for _, address, workspace in (calc, term):
            move(tier, address, workspace)
        restore_desktop(tier, holder, point)
        unfocused = frame_states(tier, calculator, calc[0])
        problems, effects = focus_step(ctx, "calculator", calc, PROOF_BOTH)
        raw.poll(lambda: "ACTIVE" in frame_states(tier, calculator, calc[0]), FOCUS_SETTLE_S)
        problems += frame_problems("calculator", unfocused, frame_states(tier, calculator, calc[0]))
        problems += _view_problems("calculator", effects, [("workspace_switched", far[0])])
        more, effects = focus_step(ctx, "terminal", term, PROOF_COMPOSITOR)
        return problems + more + _view_problems("terminal", effects, [("workspace_switched", far[1])])


def _in_view(ctx: LegContext, holder: dict, point: dict) -> list[str]:
    tier = ctx.tier
    here = (client(tier, holder["address"]).get("workspace") or {}).get("name")
    with terminal(tier, "ad-e2e-proof-near") as (foot_app, foot):
        near = (_ids(ctx, {"near": foot_app})["near"], foot["address"], here)
        move(tier, foot["address"], here)
        restore_desktop(tier, holder, point)
        require(focus_view(tier, foot["address"])["active"] != foot["address"], "the in-view terminal held focus first")
        problems, effects = focus_step(ctx, "in-view terminal", near, PROOF_COMPOSITOR)
        problems += _view_problems("in-view terminal", effects, [])
        again, effects = focus_step(ctx, "focused terminal", near, PROOF_COMPOSITOR, activated=False)
        problems += again + ([f"focused terminal: the compositor changed {effects}"] if effects else [])
        return problems


def _special_swap(ctx: LegContext, holder: dict, point: dict) -> list[str]:
    tier, main = ctx.tier, main_instance(ctx.tier)
    first, second = SPECIALS
    here = (client(tier, holder["address"]).get("workspace") or {}).get("name")
    with terminal(tier, "ad-e2e-proof-a") as (app_a, foot_a), terminal(tier, "ad-e2e-proof-b") as (app_b, foot_b), \
            terminal(tier, "ad-e2e-proof-under") as (app_c, foot_c):
        ids = _ids(ctx, {"a": app_a, "b": app_b, "under": app_c})
        a, b = (ids["a"], foot_a["address"], first), (ids["b"], foot_b["address"], second)
        for (_, address, workspace) in (a, b, (ids["under"], foot_c["address"], here)):
            move(tier, address, workspace)
        restore_desktop(tier, holder, point)
        opened = [_special(item) for item in scene(tier)["monitors"] if _special(item)]
        require(not opened, f"special workspaces are open before the leg: {opened}")
        problems = []
        for label, target, expected in (
                ("reveal a", a, [("special_workspace_revealed", first)]),
                ("leave a for b", b, [("special_workspace_closed", first), ("special_workspace_revealed", second)]),
                ("return to a", a, [("special_workspace_closed", second), ("special_workspace_revealed", first)])):
            found, effects = focus_step(ctx, label, target, PROOF_COMPOSITOR)
            problems += found + _view_problems(label, effects, expected)
        with Sampler({"desktop": lambda: desktop(tier, main)}) as sampler:
            envelope = ctx.product(["focus-window", "--window-id", ids["under"]])
            problems += special_open_problems("terminal under the open special", envelope, first)
        return problems + sampler.changes


@implementation("focus.hyprland_activation_proof_reported")
def activation_proof_reported(ctx: LegContext) -> None:
    _leg(ctx, _activation)


@implementation("focus.hyprland_in_view_no_invented_switch")
def in_view_no_invented_switch(ctx: LegContext) -> None:
    _leg(ctx, _in_view)


@implementation("focus.hyprland_special_reveal_and_close_reported")
def special_reveal_and_close_reported(ctx: LegContext) -> None:
    _leg(ctx, _special_swap)
