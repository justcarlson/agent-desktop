"""Hover legs on the attached `hyprland` tier, on the nested second instance.

Hover by ref has no receiver evidence on Hyprland, so it refuses with `hover_unsupported`. It must
refuse at once, before any pointer or modifier event, also for a GTK4 window under XWayland whose
popup covers the target (L45-3, `hyprland_receiver_fixture`): that popup is a second client of
the same pid, and a refusal that waited for the target to become actionable first ended in
`TIMEOUT`. `hover --xy` delivers; `j/cursorpos` is its oracle.

The registry entries for these legs belong to `runner/legs.py`; until they land there,
`register()` adds the implementations for a registry that names them.
"""

from __future__ import annotations

import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner import second_instance
from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_focus_controls import cursor_argv
from scenarios.hyprland_pointer import delivered, nested
from scenarios.hyprland_pointer_pad import (NATIVE, XWAYLAND, cursor_at, events, find_ref,
                                            global_point, instance_control, instance_env, instance_query, lands_at,
                                            pad_ref, pads_left, pointer_pad, settled, since, xy)
from scenarios.hyprland_receiver_fixture import TARGET, TITLE, fixture_events, receiver_fixture

REFUSAL_BUDGET_MS = 2000
QUIET_S = 0.5
LEGS = {"pointer.hover_by_ref_refused": "hover_by_ref_refused", "pointer.hover_coordinates": "hover_coordinates"}


def ref_refusal_problems(label: str, envelope) -> list[str]:
    """Why a hover by ref did not refuse at once with `hover_unsupported` before any delivery."""
    if envelope.ok:
        return [f"{label}: hover by ref delivered: {envelope.raw[:300]}"]
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if error.get("code") != "ACTION_NOT_SUPPORTED":
        problems.append(f"{label}: hover by ref answered {error.get('code')}, not ACTION_NOT_SUPPORTED: "
                        f"{envelope.raw[:300]}")
    if details.get("cause") != "hover_unsupported":
        problems.append(f"{label}: the refusal's cause is {details.get('cause')!r}, not hover_unsupported")
    if details.get("physical_delivery_started") is not False:
        problems.append(f"{label}: physical_delivery_started is {details.get('physical_delivery_started')!r}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: the refusal is not not_delivered: {error.get('disposition')}")
    if envelope.elapsed_ms > REFUSAL_BUDGET_MS:
        problems.append(f"{label}: the refusal took {envelope.elapsed_ms:.0f} ms, over {REFUSAL_BUDGET_MS} ms")
    return problems


def xy_hover_problems(envelope, cursor: tuple[float, float], aim: tuple[int, int]) -> list[str]:
    """Why `hover --xy` at `aim` did not deliver there, given `j/cursorpos` after it."""
    if not envelope.ok:
        return [f"hover --xy={xy(aim)} refused: {envelope.raw[:300]}"]
    data = envelope.data or {}
    problems = []
    if (data.get("disposition") or {}).get("delivery") != "delivered_unverified":
        problems.append(f"hover --xy={xy(aim)} reports {data.get('disposition')}, not delivered_unverified")
    if (data.get("x"), data.get("y")) != aim:
        problems.append(f"hover --xy={xy(aim)} reports the point {(data.get('x'), data.get('y'))}")
    if not lands_at({"wx": cursor[0], "wy": cursor[1]}, aim):
        problems.append(f"hover --xy={xy(aim)}: j/cursorpos reads {cursor}")
    return problems


def _popup_target(ctx, env: dict[str, str], owned) -> str:
    envelope = ctx.product(["list-windows"], env=env, headed=False)
    require(envelope.ok and isinstance(envelope.data, list), f"list-windows failed: {envelope.raw[:400]}")
    windows = [item.get("id") for item in envelope.data if item.get("pid") == owned.pid and item.get("title") == TITLE]
    require(len(windows) == 1, f"the receiver fixture window is listed {len(windows)} times")
    return find_ref(ctx, env, windows[0], *TARGET)


def _fixture_quiet(log: Path):
    """A check that the fixture logs nothing after this moment."""
    start = len(fixture_events(log))

    def quiet() -> list[str]:
        time.sleep(QUIET_S)
        found = fixture_events(log)[start:]
        return [f"gtk4 popup: the fixture logged {found[:5]}"] if found else []
    return quiet


def _refused_quietly(ctx, tier, instance, label: str, target: str, quiet) -> list[str]:
    """A hover by ref on `target` refuses at once, the cursor stays and `quiet()` sees no event."""
    env = instance_env(tier, instance)
    cursor = cursor_at(tier, instance)
    problems = ref_refusal_problems(label, ctx.product(["hover", target], env=env))
    after = cursor_at(tier, instance)
    if after != cursor:
        problems.append(f"{label}: the cursor moved from {cursor} to {after}")
    return problems + quiet()


def _pad_quiet(backend: str, log: Path):
    """A check that the pad logs nothing after this moment."""
    start = len(events(log))

    def quiet() -> list[str]:
        found = since(settled(log, lambda all_: len(all_) > start, QUIET_S), start)
        return [f"{backend} pad: the pad logged {found[:5]}"] if found else []
    return quiet


def fixtures_running(owned_apps) -> list[int]:
    """The pids of the fixtures this leg started that still run as the same process; empty once the
    leg has released them. A fixture's click count cannot be the oracle's restored read, because the
    clicks a leg sends stay in the log."""
    return sorted(app.pid for app in owned_apps
                  if second_instance._start_of(app.pid, Path("/proc")) == app.start_time)


def _cursor_on(tier, instance: dict, aim: tuple[int, int]) -> tuple[float, float] | None:
    at = cursor_at(tier, instance)
    return at if lands_at({"wx": at[0], "wy": at[1]}, aim) else None


def hover_by_ref_refused(ctx: LegContext) -> None:
    tier = ctx.tier
    instance, _ = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    ctx.before("hyprland.cursor", cursor_at(tier, instance))
    ctx.before("applog.fixture", fixtures_running([]))
    problems, pad_events = [], {}
    for backend in (XWAYLAND, NATIVE):
        with pointer_pad(tier, instance, f"hover-{backend}", backend) as (owned, _, log):
            button = pad_ref(ctx, env, owned)
            problems += _refused_quietly(ctx, tier, instance, f"{backend} pad", button, _pad_quiet(backend, log))
            pad_events[backend] = len(events(log))
    ctx.after("applog.pad", pad_events)
    with receiver_fixture(tier, instance, "gtk4", "popup") as (owned, _, log):
        target = _popup_target(ctx, env, owned)
        problems += _refused_quietly(ctx, tier, instance, "gtk4 popup", target, _fixture_quiet(log))
        ctx.after("applog.fixture", len(fixture_events(log)))
    ctx.after("hyprland.cursor", cursor_at(tier, instance))
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.cursor", cursor_at(tier, instance))
    ctx.restored("applog.pad", pads_left(tier))
    ctx.restored("applog.fixture", fixtures_running([owned]))


def hover_coordinates(ctx: LegContext) -> None:
    tier = ctx.tier
    instance, lua = nested(tier)
    env = instance_env(tier, instance)
    ctx.before("applog.pad", pads_left(tier))
    with pointer_pad(tier, instance, "hover-xy") as (_, client, log):
        before = instance_query(tier, instance, "cursorpos")
        ctx.before("hyprland.cursor", cursor_at(tier, instance))
        try:
            aim = global_point(client, (0.5, 0.75))
            envelope = ctx.product(["hover", f"--xy={xy(aim)}"], env=env)
            delivered(envelope)
            cursor = raw.poll(lambda: _cursor_on(tier, instance, aim), 3.0) or cursor_at(tier, instance)
            ctx.after("hyprland.cursor", cursor)
            ctx.after("applog.pad", len(events(log)))
            problems = xy_hover_problems(envelope, cursor, aim)
            require(not problems, "; ".join(problems))
        finally:
            instance_control(tier, instance, cursor_argv(before["x"], before["y"], lua))
            require(raw.poll(lambda: instance_query(tier, instance, "cursorpos") == before, 3.0),
                    f"the nested cursor did not return to {before}")
            ctx.restored("hyprland.cursor", cursor_at(tier, instance))
    ctx.restored("applog.pad", pads_left(tier))


def register(implementation) -> None:
    """Adds each hover leg's implementation through the registry's `implementation` decorator."""
    for leg_id, name in LEGS.items():
        implementation(leg_id)(globals()[name])
