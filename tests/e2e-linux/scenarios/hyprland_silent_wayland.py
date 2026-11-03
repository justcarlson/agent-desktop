"""A silent Wayland socket: the bound compositor's own, and one another process serves.

The product reads a Wayland socket only after the kernel's peer credentials
name the bound compositor, so a listener in the runner is refused as a
mismatch before any byte. `apps/held_wayland_socket.py` therefore listens
first and then execs a nested Hyprland, which keeps the pid the kernel
recorded, while a holder child keeps the listener and never accepts or never
replies. Discovery reads that instance's sockets in name order, so the held
socket's name sorts before every `wayland-N`. A runner-owned listener of each
behavior is checked too: named beside the session signature it must refuse as
a mismatch, and discovery must skip it. Every case runs while the session
desktop is polled, and it must not change.
"""

from __future__ import annotations

import contextlib
import os
import time
from pathlib import Path

from oracle import hyprland
from runner import attached, attached_session, second_instance, session, socket_fixture
from runner.helper import require
from runner.verdict import LegContext
from scenarios.hyprland_binding import (DISPLAYS, MISMATCH, compositor_names, display_problems, pair_env,
                                        refusal_problems)
from scenarios.hyprland_binding_refusals import QUIET_BEHAVIORS, silent_problems
from scenarios.hyprland_common import Sampler, desktop, main_instance, shared

HELD_APP = Path(__file__).resolve().parents[1] / "apps" / "held_wayland_socket.py"
SOCKET_PREFIX = "wayland-0-ad-e2e-"
LAUNCH_S = 20.0
RELEASE_S = 5.0


def reads_first(socket: Path, display: str) -> bool:
    """True when discovery, which reads a runtime directory's sockets in name order, reads `socket` before
    `display`."""
    return socket.name < display


def _poll(read, seconds: float):
    deadline = time.monotonic() + seconds
    while not (value := read()) and time.monotonic() < deadline:
        time.sleep(0.1)
    return value


def _socket_path(tier, kind: str, behavior: str) -> Path:
    return shared(tier) / f"{SOCKET_PREFIX}{kind}-{behavior}-{os.getpid()}"


def foreign_listener(tier, behavior: str) -> tuple[Path, socket_fixture.Listener]:
    """A runner-owned listener with `behavior` beside the compositors' Wayland sockets."""
    path = _socket_path(tier, "foreign", behavior)
    return path, socket_fixture.Listener(path, behavior)


def held_instance(tier, behavior: str) -> tuple[dict, Path]:
    """A nested Hyprland whose pid the kernel names as the peer of a held socket with `behavior`."""
    record = attached.load(tier.runtime_dir)
    runtime = Path(record["shared_runtime"])
    lua = hyprland.uses_lua(runtime, record["signature"])
    config = tier.runtime_dir / ("held-instance.lua" if lua else "held-instance.conf")
    config.write_text(second_instance.config_text(lua))
    held = _socket_path(tier, "held", behavior)
    before = set(hyprland.instances(runtime))
    argv = ["python3", "-I", str(HELD_APP), str(held), behavior, "--", "Hyprland", "-c", str(config)]
    attached_session.spawn(tier, argv, second_instance.launch_env(tier), tier.runtime_dir / f"held-{behavior}.log",
                           second_instance.OUTER_WORKSPACE)
    try:
        instance = _poll(lambda: second_instance.find_new(attached.load(tier.runtime_dir), before), LAUNCH_S)
        if instance is None:
            raise session.TierUnavailable("hyprland: the held instance did not answer on its IPC socket")
        attached.record_instance(tier.runtime_dir, instance)
        instance["wayland"] = _poll(lambda: second_instance.wayland_of(runtime, instance["pid"]), LAUNCH_S)
        if instance["wayland"] is None:
            raise session.TierUnavailable("hyprland: the held instance serves no Wayland socket")
        attached.record_instance(tier.runtime_dir, instance)
        if hyprland.peer_pid(held) != instance["pid"]:
            raise session.TierUnavailable(f"hyprland: the held socket is not served by pid {instance['pid']}")
        if not reads_first(held, instance["wayland"]):
            raise session.TierUnavailable(f"hyprland: discovery reads {instance['wayland']} before {held.name}")
    except BaseException:
        release(tier, held)
        raise
    return instance, held


def release(tier, held: Path) -> None:
    """Stop the held instance; its holder removes the socket when the instance's pid ends."""
    second_instance.stop(tier)
    if not _poll(lambda: not os.path.lexists(held), RELEASE_S):
        with contextlib.suppress(FileNotFoundError):
            held.unlink()
        raise AssertionError(f"the holder left {held} behind")


def mismatch_problems(label: str, envelope, socket: Path) -> list[str]:
    """`hyprland_mismatch` that names the socket another process serves."""
    problems = refusal_problems(label, envelope, MISMATCH)
    detail = str(((envelope.error or {}).get("details") or {}).get("detail", ""))
    return problems + ([] if str(socket) in detail else [f"{label}: details.detail does not name {socket}: {detail!r}"])


def _sampled(ctx: LegContext, phase: str, main: dict, work) -> None:
    runtime, channel = shared(ctx.tier), f"hyprland.desktop.{phase}"
    ctx.before(channel, desktop(ctx.tier, main))
    reads = {"desktop": lambda: desktop(ctx.tier, main),
             "monitors": lambda: compositor_names(runtime, main["signature"])}
    with Sampler(reads) as sampler:
        problems = work()
    problems += sampler.changes
    ctx.after(channel, desktop(ctx.tier, main))
    require(not problems, "; ".join(problems))
    ctx.restored(channel, desktop(ctx.tier, main))


def _foreign_cases(ctx: LegContext, main: dict, main_names: list[str], behavior: str, socket: Path) -> list[str]:
    runtime = shared(ctx.tier)
    named = ctx.product(DISPLAYS, env=pair_env(ctx.tier, main["signature"], socket.name, runtime), headed=False)
    found = ctx.product(DISPLAYS, env=pair_env(ctx.tier, main["signature"], None, runtime), headed=False)
    return (mismatch_problems(f"a {behavior} foreign socket named beside the session signature", named, socket)
            + display_problems(f"discovery by the session signature beside a {behavior} foreign socket", found,
                               main_names))


def _held_cases(ctx: LegContext, instance: dict, behavior: str, held: Path) -> list[str]:
    runtime = shared(ctx.tier)
    named = ctx.product(DISPLAYS, env=pair_env(ctx.tier, instance["signature"], held.name, runtime), headed=False)
    found = ctx.product(DISPLAYS, env=pair_env(ctx.tier, instance["signature"], None, runtime), headed=False)
    return (silent_problems(f"a {behavior} held socket named beside its instance signature", named, held)
            + silent_problems(f"discovery by the signature of an instance holding a {behavior} socket", found, held))


def hyprland_silent_wayland_refused(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    second_instance.stop(ctx.tier)
    runtime = shared(ctx.tier)
    main_names = compositor_names(runtime, main["signature"])
    for behavior in QUIET_BEHAVIORS:
        socket, listener = foreign_listener(ctx.tier, behavior)
        try:
            _sampled(ctx, f"foreign_{behavior}", main,
                     lambda: _foreign_cases(ctx, main, main_names, behavior, socket))
        finally:
            listener.close()
    for behavior in QUIET_BEHAVIORS:
        instance, held = held_instance(ctx.tier, behavior)
        try:
            _sampled(ctx, f"held_{behavior}", main, lambda: _held_cases(ctx, instance, behavior, held))
        finally:
            release(ctx.tier, held)
    healthy = ctx.product(DISPLAYS, env=pair_env(ctx.tier, main["signature"], main["wayland"], runtime), headed=False)
    problems = display_problems("the session pair once the sockets are gone", healthy, main_names)
    require(not problems, "; ".join(problems))
