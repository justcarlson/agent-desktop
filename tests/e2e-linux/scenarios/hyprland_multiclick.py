"""Coordinate double and triple click on the attached `hyprland` tier, on XWayland windows of the
nested second instance.

On Hyprland a headed `double-click` or `triple-click` by ref on an XWayland window refuses as
`receiver_unproven` (L45-2), so the fallback is `mouse-click --xy X,Y --count 2|3`. This leg sends
it to the centre of "Probe Target" in the GTK 3, GTK 4 and Qt 6 `receiver_fixture`, whose own log is
the oracle: each toolkit logs one `clicked` per press and release, so a double click on the target
logs it twice and a triple click three times. The log names the widget, not the point or the
gesture count; the GTK 3 pointer pad's press count is checked by
`pointer.coordinates_have_no_receiver_proof`. The report still says `receiver_proof: false`,
because the compositor cannot show which surface took the events.

The registry entries for this leg belong to `runner/legs.py`; until they land there,
`register()` adds the implementation for a registry that names it.
"""

from __future__ import annotations

import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_focus_controls import cursor_argv
from scenarios.hyprland_hover import fixtures_running
from scenarios.hyprland_pointer import nested, no_receiver_proof
from scenarios.hyprland_pointer_pad import cursor_at, instance_control, instance_env, instance_query, xy
from scenarios.hyprland_pointer_refs import fixture_ref, refused_without_input
from scenarios.hyprland_receiver_fixture import TARGET, clicked, fixture_events, receiver_fixture

TOOLKITS = ("gtk3", "gtk4", "qt6")
COUNTS = (2, 3)
CLICK_GAP_S = 1.0
CLICK_SETTLE_S = 3.0
EXTRA_CLICK_SETTLE_S = 0.5
LEGS = {"pointer.coordinate_multi_click_xwayland": "coordinate_multi_click_xwayland"}


def fixture_click_problems(label: str, envelope, found: list[dict], count: int) -> list[str]:
    """`mouse-click --count <count>` reports `count` clicks as delivered_unverified without receiver
    proof, and the fixture logs exactly `count` clicks, all on the target."""
    name = f"{label} mouse-click --count {count}"
    if not envelope.ok:
        return [f"{name}: answered {envelope.raw[:300]}"]
    data = envelope.data or {}
    problems = no_receiver_proof(name, data)
    delivery = (data.get("disposition") or {}).get("delivery")
    if delivery != "delivered_unverified":
        problems.append(f"{name}: the report says delivery {delivery!r}, not delivered_unverified")
    if data.get("count") != count:
        problems.append(f"{name}: the report says count {data.get('count')!r}")
    logged = clicked(found)
    if logged != [TARGET[1]] * count:
        problems.append(f"{name}: the fixture logged {len(logged)} clicks {logged}, expected {count} on {TARGET[1]}")
    return problems


def target_centre(ctx, env: dict[str, str], ref: str, client: dict) -> tuple[float, float]:
    """The centre of the target's live bounds, which must lie inside the fixture's toplevel."""
    read = ctx.product(["get", ref, "--property", "bounds"], env=env, headed=False)
    bounds = (read.data or {}).get("value") if read.ok else None
    require(bounds, f"the target has no live bounds: {read.raw[:300]}")
    centre = bounds["x"] + bounds["width"] / 2, bounds["y"] + bounds["height"] / 2
    (x, y), (width, height) = client["at"], client["size"]
    require(x <= centre[0] <= x + width and y <= centre[1] <= y + height,
            f"the target centre {centre} is outside the toplevel at {client['at']} size {client['size']}")
    return centre


def apart_from_last_event(log: Path, gap: float = CLICK_GAP_S) -> None:
    """Wait until `gap` seconds have passed since the fixture's last event, so the next press does
    not join the last click sequence."""
    found = fixture_events(log)
    if found and isinstance(found[-1].get("t"), (int, float)):
        left = found[-1]["t"] + gap - time.monotonic()
        if left > 0:
            time.sleep(left)


def coordinate_clicks(ctx, env, log: Path, label: str, centre: tuple[float, float]) -> list[str]:
    problems = []
    for count in COUNTS:
        apart_from_last_event(log)
        start = len(fixture_events(log))
        envelope = ctx.product(["mouse-click", f"--xy={xy(centre)}", "--count", str(count)], env=env)
        raw.poll(lambda: len(clicked(fixture_events(log)[start:])) >= count, CLICK_SETTLE_S)
        raw.poll(lambda: len(clicked(fixture_events(log)[start:])) > count, EXTRA_CLICK_SETTLE_S)
        problems += fixture_click_problems(label, envelope, fixture_events(log)[start:], count)
    return problems


def toolkit_problems(ctx, env, instance: dict, toolkit: str, started: list) -> tuple[list[str], list[str]]:
    """The problems on one fixture and the widgets its log names as clicked."""
    with receiver_fixture(ctx.tier, instance, toolkit, "plain") as (owned, client, log):
        started.append(owned)
        target = fixture_ref(ctx, env, owned.pid)
        label = f"{toolkit} plain fixture"
        problems = refused_without_input(ctx, env, instance, label,
                                         [["double-click", target], ["triple-click", target]],
                                         lambda: len(clicked(fixture_events(log))))
        problems += coordinate_clicks(ctx, env, log, label, target_centre(ctx, env, target, client))
        return problems, clicked(fixture_events(log))


def coordinate_multi_click_xwayland(ctx: LegContext) -> None:
    """Headed double and triple click by ref refuse on each XWayland fixture with no click logged;
    `mouse-click --xy --count 2|3` then lands that many clicks on the target, without receiver proof.
    The fixtures are gone and the nested cursor is back where it was at the end."""
    tier = ctx.tier
    instance, lua = nested(tier)
    env = instance_env(tier, instance)
    before = instance_query(tier, instance, "cursorpos")
    ctx.before("hyprland.cursor", cursor_at(tier, instance))
    ctx.before("applog.fixture", fixtures_running([]))
    problems, logged, started = [], {}, []
    try:
        for toolkit in TOOLKITS:
            found, logged[toolkit] = toolkit_problems(ctx, env, instance, toolkit, started)
            problems += found
        ctx.after("hyprland.cursor", cursor_at(tier, instance))
        ctx.after("applog.fixture", logged)
    finally:
        instance_control(tier, instance, cursor_argv(before["x"], before["y"], lua))
        require(raw.poll(lambda: instance_query(tier, instance, "cursorpos") == before, 3.0),
                f"the nested cursor did not return to {before}")
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.cursor", cursor_at(tier, instance))
    ctx.restored("applog.fixture", fixtures_running(started))
    ctx.finding("each mouse-click --count report says receiver_proof: false")


def register(implementation) -> None:
    """Adds the leg's implementation through the registry's `implementation` decorator."""
    for leg_id, name in LEGS.items():
        implementation(leg_id)(globals()[name])
