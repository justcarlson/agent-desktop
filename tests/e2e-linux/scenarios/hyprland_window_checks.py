"""Verdicts for the Hyprland window legs.

A leg runs one product command, then reads the target back from the compositor through
`read`, which returns the target's `placement`. The command's own `ok` and disposition must
agree with what the compositor shows; neither alone passes. `env` binds the product to a
nested instance when the leg works there.
"""

from __future__ import annotations

from typing import Callable

from oracle import atspi_raw as raw, hyprland
from scenarios.hyprland_desktop import boundary_problems
from scenarios.hyprland_focus_controls import lua_call

MINIMIZED = "special:agent-desktop-minimized-"
SETTLE_S = 3.0
TOLERANCE = 2
VERIFIED = "delivered_verified"
OPERATIONS = (("move-window", "--x", "40", "--y", "60"), ("resize-window", "--width", "500", "--height", "400"),
              ("minimize",), ("maximize",), ("restore",))

Read = Callable[[], dict]


def point(x: int, y: int) -> list[str]:
    """`--x` and `--y` in the `=` form, the one a negative value parses in."""
    return [f"--x={x}", f"--y={y}"]


def placement(item: dict | None) -> dict:
    """The fields of a toplevel that a window operation may change."""
    item = item or {}
    return {"at": list(item.get("at") or ()), "size": list(item.get("size") or ()),
            "floating": item.get("floating"), "fullscreen": item.get("fullscreen"),
            "workspace": (item.get("workspace") or {}).get("name")}


def holds(now: dict, expected: dict) -> bool:
    """Every expected field is as asked; a position or size within the tolerance."""
    for key, value in expected.items():
        if key in ("at", "size"):
            seen = now.get(key) or []
            if len(seen) != len(value) or any(abs(a - b) > TOLERANCE for a, b in zip(seen, value)):
                return False
        elif now.get(key) != value:
            return False
    return True


def float_argv(address: str, lua: bool) -> list[str]:
    target = f"address:{address}"
    if lua:
        return ["dispatch", lua_call("hl.dsp.window.float", {"action": "enable", "window": target})]
    hyprland.lua_string(target)
    return ["dispatch", "setfloating", target]


def window_argv(verb: str, window_id: str, *extra: str) -> list[str]:
    return [verb, "--window-id", window_id, *extra]


def delivery(envelope) -> str | None:
    return ((envelope.data or {}).get("disposition") or {}).get("delivery")


def operated(ctx, label: str, argv: list[str], read: Read, expected: dict, *, env=None) -> list[str]:
    """Run one operation, then read the toplevel back."""
    envelope = ctx.product(argv, env=env)
    raw.poll(lambda: holds(read(), expected), SETTLE_S)
    now = read()
    problems = [] if holds(now, expected) else [f"{label}: j/clients shows {now}, not {expected}"]
    if not envelope.ok or delivery(envelope) != VERIFIED:
        problems.append(f"{label}: expected {VERIFIED}, got {envelope.raw[:400]}")
    return problems


def satisfied(ctx, label: str, argv: list[str], read: Read, *, env=None) -> list[str]:
    """An operation already satisfied answers not_delivered and changes nothing."""
    before = read()
    envelope = ctx.product(argv, env=env)
    problems = [] if envelope.ok and delivery(envelope) == "not_delivered" else [
        f"{label}: expected ok and not_delivered, got {envelope.raw[:400]}"]
    return problems + unchanged(label, read, before)


def refusal(ctx, label: str, argv: list[str], read: Read, cause: str, *, env=None) -> list[str]:
    """A refused operation sends nothing, so the target is as it was."""
    before = read()
    envelope = ctx.product(argv, env=env)
    problems = boundary_problems(label, envelope, ("ACTION_NOT_SUPPORTED",), physical=True, cause=cause)
    return problems + unchanged(label, read, before)


def missing(ctx, label: str, window_id: str, reads: dict[str, Read], *, env=None) -> list[str]:
    """Every operation on a window that is gone answers WINDOW_NOT_FOUND and falls back to no
    other window: each window in `reads` is as it was."""
    problems = []
    for operation in OPERATIONS:
        before = {name: read() for name, read in reads.items()}
        envelope = ctx.product(window_argv(operation[0], window_id, *operation[1:]), env=env)
        problems += boundary_problems(f"{label} {operation[0]}", envelope, ("WINDOW_NOT_FOUND",))
        for name, read in reads.items():
            problems += unchanged(f"{label} {operation[0]}, {name}", read, before[name])
    return problems


def unchanged(label: str, read: Read, before: dict) -> list[str]:
    after = read()
    return [] if after == before else [f"{label}: the target changed from {before} to {after}"]
