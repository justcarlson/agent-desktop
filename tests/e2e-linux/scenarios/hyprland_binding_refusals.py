"""Bindings the product must refuse: a silent instance, several live instances, a swapped pair.

A silent instance is a fixture listener under the shared `hypr/` directory
that never accepts, or accepts and never replies. The several-instance and
swapped-pair cases use the runner's second Hyprland. Every refusal is checked
as a refusal, and every desktop it could reach is polled while it runs and must
not change.
"""

from __future__ import annotations

from pathlib import Path

from oracle import hyprland
from runner import second_instance, socket_fixture
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_binding import (BUDGET_MS, DISPLAYS, MISMATCH, bare_env, compositor_names,
                                        display_problems, pair_env, refusal_problems)
from scenarios.hyprland_common import Sampler, desktop, main_instance, shared

SILENT_CODE = "TIMEOUT"
SILENT_CAUSE = "silent"
AMBIGUOUS = "hyprland_ambiguous"
QUIET_BEHAVIORS = (socket_fixture.SILENT, socket_fixture.MUTE)


def silent_problems(label: str, envelope, socket: Path) -> list[str]:
    """The silent-endpoint contract: TIMEOUT, cause silent, retryable, the socket named, exit 1,
    not delivered, within the budget."""
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != SILENT_CODE:
        problems.append(f"{label}: expected {SILENT_CODE}, got ok={envelope.ok} code={error.get('code')!r}")
    if details.get("display_unavailable") != SILENT_CAUSE:
        problems.append(f"{label}: display_unavailable is {details.get('display_unavailable')!r}, "
                        f"not {SILENT_CAUSE!r}")
    if details.get("retryable") is not True:
        problems.append(f"{label}: a silent endpoint must be retryable, got {details.get('retryable')!r}")
    if details.get("detail") != str(socket):
        problems.append(f"{label}: details.detail is {details.get('detail')!r}, not the socket {socket}")
    if envelope.exit_code != 1:
        problems.append(f"{label}: a structured error must exit 1, got {envelope.exit_code}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: a refused binding must be reported not_delivered")
    if envelope.elapsed_ms > BUDGET_MS:
        problems.append(f"{label}: refused after {envelope.elapsed_ms:.0f} ms, over {BUDGET_MS:.0f} ms")
    return problems


def ambiguous_problems(label: str, envelope, signatures: list[str]) -> list[str]:
    """`hyprland_ambiguous` names exactly the live instances."""
    problems = refusal_problems(label, envelope, AMBIGUOUS)
    named = ((envelope.error or {}).get("details") or {}).get("signatures")
    if not isinstance(named, list) or sorted(named) != sorted(signatures):
        problems.append(f"{label}: details.signatures is {named!r}, the live instances are {sorted(signatures)}")
    return problems


def swapped_problems(label: str, envelope, signature: str, socket: Path) -> list[str]:
    """`hyprland_mismatch` that names the bound instance and the foreign Wayland socket."""
    problems = refusal_problems(label, envelope, MISMATCH)
    detail = str(((envelope.error or {}).get("details") or {}).get("detail", ""))
    problems += [f"{label}: details.detail does not name {name}: {detail!r}"
                 for name in (signature, str(socket)) if name not in detail]
    return problems


def _reads(tier, instances: dict) -> dict:
    runtime = shared(tier)
    reads = {}
    for name, instance in instances.items():
        reads[f"{name}.desktop"] = lambda instance=instance: desktop(tier, instance)
        reads[f"{name}.monitors"] = lambda instance=instance: compositor_names(runtime, instance["signature"])
    return reads


def _sampled(ctx: LegContext, instances: dict, work) -> None:
    """Record both desktops, run `work` under a sampler, then require no problem and no change."""
    for name, instance in instances.items():
        ctx.before(f"hyprland.{name}", desktop(ctx.tier, instance))
    with Sampler(_reads(ctx.tier, instances)) as sampler:
        problems = work()
    problems += sampler.changes
    for name, instance in instances.items():
        ctx.after(f"hyprland.{name}", desktop(ctx.tier, instance))
    require(not problems, "; ".join(problems))
    for name, instance in instances.items():
        ctx.restored(f"hyprland.{name}", desktop(ctx.tier, instance))


def _second(ctx: LegContext) -> dict:
    try:
        return second_instance.ensure_sized(ctx.tier)
    except second_instance.OutputsNotReady as error:
        ctx.after("hyprland.second_not_ready", error.diagnostics)
        raise


def _silent_cases(ctx: LegContext, main: dict, main_names: list[str]) -> list[str]:
    runtime = shared(ctx.tier)
    problems = []
    for behavior in QUIET_BEHAVIORS:
        signature, listener = socket_fixture.instance(ctx.tier.runtime_dir, behavior)
        try:
            socket = runtime / "hypr" / signature / ".socket.sock"
            for display_label, wayland in (("no display named", None), ("the session display", main["wayland"])):
                label = f"a {behavior} instance named, {display_label}"
                envelope = ctx.product(DISPLAYS, env=pair_env(ctx.tier, signature, wayland, runtime), headed=False)
                problems += silent_problems(label, envelope, socket)
            envelope = ctx.product(DISPLAYS, env=bare_env(ctx.tier, XDG_RUNTIME_DIR=str(runtime)), headed=False)
            problems += silent_problems(f"discovery beside a {behavior} instance", envelope, socket)
        finally:
            listener.close()
    healthy = ctx.product(DISPLAYS, env=pair_env(ctx.tier, main["signature"], main["wayland"], runtime), headed=False)
    return problems + display_problems("the session pair once the fixtures are gone", healthy, main_names)


@implementation("session.hyprland_silent_ipc_refused")
def hyprland_silent_ipc_refused(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    second_instance.stop(ctx.tier)
    main_names = compositor_names(shared(ctx.tier), main["signature"])
    _sampled(ctx, {"desktop": main}, lambda: _silent_cases(ctx, main, main_names))


def _ambiguous_cases(ctx: LegContext, main: dict, nested: dict) -> list[str]:
    runtime = shared(ctx.tier)
    live = [main["signature"], nested["signature"]]
    problems = []
    for label, wayland in (("discovery with only the runtime directory", None),
                           ("discovery with the session display", main["wayland"])):
        env = bare_env(ctx.tier, XDG_RUNTIME_DIR=str(runtime), WAYLAND_DISPLAY=wayland)
        problems += ambiguous_problems(label, ctx.product(DISPLAYS, env=env, headed=False), live)
    for label, instance in (("the session pair", main), ("the second pair", nested)):
        envelope = ctx.product(DISPLAYS, env=pair_env(ctx.tier, instance["signature"], instance["wayland"], runtime),
                               headed=False)
        problems += display_problems(f"{label} beside the other instance", envelope,
                                     compositor_names(runtime, instance["signature"]))
    return problems


@implementation("session.hyprland_several_instances_ambiguous")
def hyprland_several_instances_ambiguous(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    try:
        nested = _second(ctx)
        _sampled(ctx, {"desktop": main, "second_desktop": nested}, lambda: _ambiguous_cases(ctx, main, nested))
    finally:
        second_instance.stop(ctx.tier)


def _swapped_cases(ctx: LegContext, main: dict, nested: dict) -> list[str]:
    runtime = shared(ctx.tier)
    problems = []
    for label, owner, other in (("the session signature with the second display", main, nested),
                                ("the second signature with the session display", nested, main)):
        envelope = ctx.product(DISPLAYS, env=pair_env(ctx.tier, owner["signature"], other["wayland"], runtime),
                               headed=False)
        problems += swapped_problems(label, envelope, owner["signature"],
                                     hyprland.wayland_path(runtime, other["wayland"]))
    return problems


@implementation("session.hyprland_swapped_pair_mismatch")
def hyprland_swapped_pair_mismatch(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    try:
        nested = _second(ctx)
        _sampled(ctx, {"desktop": main, "second_desktop": nested}, lambda: _swapped_cases(ctx, main, nested))
    finally:
        second_instance.stop(ctx.tier)
