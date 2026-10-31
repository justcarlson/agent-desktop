"""R1: which Hyprland instance the product binds, and when it refuses to bind."""

from __future__ import annotations

from oracle.proc import members
from runner import second_instance, socket_fixture
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import (BUDGET_MS, DEFAULT_RUNTIME, bare_env, desktop, main_instance,
                                       monitors, pair_env, refusal_problems, reported_displays, shared)

LIST = ["list-displays"]
HEADED_INPUT = ["press", "shift"]
PLATFORM = "PLATFORM_NOT_SUPPORTED"


def _observe(ctx: LegContext, stage: str, instances: dict) -> None:
    record = getattr(ctx, stage)
    for channel, instance in instances.items():
        record(channel, desktop(ctx.tier, instance))
    record("proc.members", members(ctx.tier.runtime_dir))


def _displays(ctx: LegContext, env: dict, expected, label: str) -> list[str]:
    envelope = ctx.product(LIST, env=env, headed=False)
    try:
        reported = reported_displays(envelope)
    except AssertionError as error:
        return [f"{label}: {error}"]
    return [] if reported == expected else [f"{label}: reported {reported}, the compositor has {expected}"]


def _refused(ctx: LegContext, env: dict, code: str, cause: str, label: str) -> list[str]:
    problems = refusal_problems(label, ctx.product(LIST, env=env, headed=False), code, cause)
    if ctx.mode == "headed":
        problems += refusal_problems(f"{label} (headed input)", ctx.product(HEADED_INPUT, env=env),
                                     code, cause, physical=True)
    return problems


def _discovery_envs(ctx: LegContext) -> dict[str, dict]:
    runtime = shared(ctx.tier)
    envs = {"runtime directory only": bare_env(ctx.tier, XDG_RUNTIME_DIR=str(runtime))}
    if runtime == DEFAULT_RUNTIME:
        envs["empty environment"] = bare_env(ctx.tier)
    return envs


@implementation("session.hyprland_discovery_single_live_instance")
def discovery_single_live_instance(ctx: LegContext) -> None:
    second_instance.stop(ctx.tier)
    main = main_instance(ctx.tier)
    expected = monitors(ctx.tier, main)
    _observe(ctx, "before", {"hyprland.desktop": main})
    socket_fixture.stale_instances(ctx.tier.runtime_dir)
    _, hangup = socket_fixture.instance(ctx.tier.runtime_dir, socket_fixture.HANGUP)
    try:
        problems = []
        for label, env in _discovery_envs(ctx).items():
            problems += _displays(ctx, env, expected, label)
        _observe(ctx, "after", {"hyprland.desktop": main})
        require(not problems, "; ".join(problems))
    finally:
        hangup.close()
    _observe(ctx, "restored", {"hyprland.desktop": main})


@implementation("session.hyprland_explicit_binding_with_other_instances")
def explicit_binding_with_other_instances(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    nested = second_instance.ensure(ctx.tier)
    instances = {"hyprland.desktop": main, "hyprland.second_desktop": nested}
    main_outputs, nested_outputs = monitors(ctx.tier, main), monitors(ctx.tier, nested)
    require(not {name for name, _ in main_outputs} & {name for name, _ in nested_outputs},
            f"the two instances share an output name: {main_outputs} {nested_outputs}")
    _observe(ctx, "before", instances)
    problems = []
    for label, env, expected in (
        ("session pair", pair_env(ctx.tier, main["signature"], main["wayland"]), main_outputs),
        ("second pair", pair_env(ctx.tier, nested["signature"], nested["wayland"]), nested_outputs),
        ("second signature only", pair_env(ctx.tier, nested["signature"], None), nested_outputs),
        ("session signature only", pair_env(ctx.tier, main["signature"], None), main_outputs),
    ):
        problems += _displays(ctx, env, expected, label)
    _observe(ctx, "after", instances)
    require(not problems, "; ".join(problems))
    _observe(ctx, "restored", instances)


@implementation("session.hyprland_ambiguous_or_mismatched_binding_refused")
def ambiguous_or_mismatched_binding_refused(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    nested = second_instance.ensure(ctx.tier)
    instances = {"hyprland.desktop": main, "hyprland.second_desktop": nested}
    no_live = socket_fixture.private_runtime(ctx.tier.runtime_dir)
    _observe(ctx, "before", instances)
    problems = []
    for label, env, cause in (
        ("several live instances", pair_env(ctx.tier, None, None), "hyprland_ambiguous"),
        ("several, display served by one", pair_env(ctx.tier, None, main["wayland"]), "hyprland_ambiguous"),
        ("session signature, second display", pair_env(ctx.tier, main["signature"], nested["wayland"]),
         "hyprland_mismatch"),
        ("second signature, session display", pair_env(ctx.tier, nested["signature"], main["wayland"]),
         "hyprland_mismatch"),
        ("no live instance", bare_env(ctx.tier, XDG_RUNTIME_DIR=str(no_live)), "not_configured"),
    ):
        problems += _refused(ctx, env, PLATFORM, cause, label)
    several = ctx.product(LIST, env=pair_env(ctx.tier, None, None), headed=False)
    named = sorted(((several.error or {}).get("details") or {}).get("signatures") or [])
    if named != sorted((main["signature"], nested["signature"])):
        problems.append(f"the ambiguous refusal names {named}, not both live instances")
    problems += _displays(ctx, pair_env(ctx.tier, main["signature"], main["wayland"]),
                          monitors(ctx.tier, main), "validated pair beside a second instance")
    _observe(ctx, "after", instances)
    require(not problems, "; ".join(problems))
    _observe(ctx, "restored", instances)


@implementation("session.hyprland_silent_socket_deadline")
def silent_socket_deadline(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    instances = {"hyprland.desktop": main}
    work = ctx.tier.runtime_dir
    _, silent = socket_fixture.instance(work, socket_fixture.SILENT)
    listeners = [silent]
    try:
        _observe(ctx, "before", instances)
        problems = _silent(ctx, pair_env(ctx.tier, None, None), "silent IPC during discovery", silent.path)
        mute_signature, mute = socket_fixture.instance(work, socket_fixture.MUTE)
        answer_signature, answer = socket_fixture.instance(work, socket_fixture.ANSWER)
        silent_wayland = socket_fixture.Listener(work / "wayland-silent", socket_fixture.MUTE)
        listeners += [mute, answer, silent_wayland]
        for label, env, socket in (
            ("named instance accepts and never replies", pair_env(ctx.tier, mute_signature, None), mute.path),
            ("silent Wayland socket", pair_env(ctx.tier, answer_signature, str(silent_wayland.path)),
             silent_wayland.path),
        ):
            problems += _silent(ctx, env, label, socket)
        problems += _displays(ctx, pair_env(ctx.tier, main["signature"], main["wayland"]),
                              monitors(ctx.tier, main), "healthy pair beside silent endpoints")
        _observe(ctx, "after", instances)
        require(not problems, "; ".join(problems))
    finally:
        for listener in listeners:
            listener.close()
    _observe(ctx, "restored", instances)


def _silent(ctx: LegContext, env: dict, label: str, socket) -> list[str]:
    checks = [(label, ctx.product(LIST, env=env, headed=False), False)]
    if ctx.mode == "headed":
        checks.append((f"{label} (headed input)", ctx.product(HEADED_INPUT, env=env), True))
    problems = []
    for name, envelope, physical in checks:
        problems += refusal_problems(name, envelope, "TIMEOUT", "silent", physical=physical)
        details = (envelope.error or {}).get("details") or {}
        if details.get("retryable") is not True:
            problems.append(f"{name}: a silent endpoint must be retryable")
        if str(socket) not in str(details.get("detail", "")):
            problems.append(f"{name}: the refusal does not name {socket}")
        if envelope.elapsed_ms > BUDGET_MS:
            problems.append(f"{name}: refused after {envelope.elapsed_ms:.0f} ms, over {BUDGET_MS:.0f} ms")
    return problems
