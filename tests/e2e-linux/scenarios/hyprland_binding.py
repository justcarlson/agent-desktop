"""Which Hyprland instance the product binds, read against `hyprctl`.

Each expectation comes from `hyprctl -j monitors` against one named instance,
never from the product's own answer. A refused binding is checked as a refusal
and also by the desktops it must leave alone.
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

from oracle import hyprland
from runner import product, second_instance, socket_fixture
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, desktop, main_instance, shared

BUDGET_MS = 2000.0
DISPLAYS = ["list-displays"]
PLATFORM = "PLATFORM_NOT_SUPPORTED"
MISMATCH = "hyprland_mismatch"
GONE_SIGNATURE = "ad_e2e_no_such_instance"
DEBUG_LIMIT = 262_144


def bare_env(tier, **values: str | None) -> dict[str, str]:
    """An SSH-like environment: no display, bus or compositor variables."""
    env = {"PATH": tier.env.get("PATH", os.defpath), "HOME": tier.env.get("HOME", str(Path.home())),
           "AGENT_DESKTOP_HOME": str(tier.state_root)}
    env.update({key: value for key, value in values.items() if value is not None})
    return env


def pair_env(tier, signature: str | None, wayland: str | None, runtime: Path | None = None) -> dict[str, str]:
    return bare_env(tier, XDG_RUNTIME_DIR=str(runtime or shared(tier)),
                    HYPRLAND_INSTANCE_SIGNATURE=signature, WAYLAND_DISPLAY=wayland)


def monitor_names(monitors: list[dict]) -> list[str]:
    return sorted(monitor["name"] for monitor in monitors if not monitor.get("disabled"))


def compositor_names(runtime: Path, signature: str) -> list[str]:
    return monitor_names(json.loads(hyprland.control(runtime, signature, ["-j", "monitors"])))


def display_problems(label: str, envelope, expected: list[str]) -> list[str]:
    """How `list-displays` differs from the outputs one compositor reports."""
    if not envelope.ok or not isinstance(envelope.data, list):
        return [f"{label}: list-displays failed: {envelope.raw[:400]}"]
    if not expected:
        return [f"{label}: the compositor reports no output"]
    got = sorted(str(item.get("id")) for item in envelope.data)
    return [] if got == expected else [f"{label}: list-displays names {got}, the compositor {expected}"]


def monitors_all_record(runtime: Path, signature: str) -> object:
    """One instance's `hyprctl -j monitors all`, parsed when it is JSON, else the raw answer or the failure."""
    try:
        text = hyprland.control(runtime, signature, ["-j", "monitors", "all"])
    except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
        return {"error": str(error)}
    try:
        return json.loads(text)
    except ValueError:
        return {"raw": text}


def debug_record(label: str, env: dict[str, str], returncode: int | None, stderr: str) -> dict:
    """The stderr of one `list-displays` rerun with WAYLAND_DEBUG=1, or a note that it printed nothing."""
    record = {"label": label, "env": {"WAYLAND_DISPLAY": env.get("WAYLAND_DISPLAY"),
                                      "HYPRLAND_INSTANCE_SIGNATURE": env.get("HYPRLAND_INSTANCE_SIGNATURE"),
                                      "WAYLAND_DEBUG": env.get("WAYLAND_DEBUG")},
              "exit_code": returncode, "stderr_bytes": len(stderr.encode())}
    if not stderr.strip():
        record["stderr_empty"] = True
        return record
    record["stderr"] = stderr[:DEBUG_LIMIT]
    record["stderr_truncated"] = len(stderr) > DEBUG_LIMIT
    return record


def wayland_debug_env(env: dict[str, str]) -> dict[str, str]:
    return {**env, "LANG": env.get("LANG", "C.UTF-8"), "WAYLAND_DEBUG": "1"}


def rerun_with_wayland_debug(label: str, env: dict[str, str]) -> dict:
    debug_env = wayland_debug_env(env)
    binary = os.environ.get(product.BINARY_ENV) or str(product.DEFAULT_BINARY)
    try:
        result = subprocess.run([binary, *DISPLAYS], env=debug_env, capture_output=True, text=True,
                                timeout=30, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"label": label, "error": str(error)}
    return debug_record(label, debug_env, result.returncode, result.stderr)


def record_second_failure(ctx: LegContext, runtime: Path, signature: str, label: str,
                          env: dict[str, str]) -> None:
    """Evidence for a refused second-instance listing: its monitors and the Wayland protocol log."""
    ctx.after("hyprland.second_monitors_all", {"label": label, "monitors": monitors_all_record(runtime, signature)})
    ctx.after("hyprland.second_wayland_debug", rerun_with_wayland_debug(label, env))


def refusal_problems(label: str, envelope, cause: str) -> list[str]:
    """The refused-binding contract: code, cause, exit 1, not delivered, within the budget."""
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != PLATFORM:
        problems.append(f"{label}: expected {PLATFORM}, got ok={envelope.ok} code={error.get('code')!r}")
    if details.get("display_unavailable") != cause:
        problems.append(f"{label}: display_unavailable is {details.get('display_unavailable')!r}, not {cause!r}")
    if envelope.exit_code != 1:
        problems.append(f"{label}: a structured error must exit 1, got {envelope.exit_code}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: a refused binding must be reported not_delivered")
    if envelope.elapsed_ms > BUDGET_MS:
        problems.append(f"{label}: refused after {envelope.elapsed_ms:.0f} ms, over {BUDGET_MS:.0f} ms")
    return problems


def dead_signatures(work: Path) -> dict[str, str]:
    """Named instances that are not live: absent, a plain file, and a socket nobody serves."""
    plain, dead = socket_fixture.stale_instances(work)
    return {"an absent instance": GONE_SIGNATURE, "a stale plain socket file": plain,
            "a bound socket nobody serves": dead}


def generic_runtimes(work: Path) -> dict[str, Path]:
    """Runtime directories in which no Hyprland instance answers."""
    empty = Path(tempfile.mkdtemp(prefix="nh.", dir=work))
    empty.chmod(0o700)
    return {"only stale instances": socket_fixture.private_runtime(work), "no hypr directory": empty}


def _observe(ctx: LegContext, stage: str, instances: dict) -> None:
    record = getattr(ctx, stage)
    for channel, instance in instances.items():
        record(channel, desktop(ctx.tier, instance))


@implementation("session.hyprland_live_instance_binds")
def hyprland_live_instance_binds(ctx: LegContext) -> None:
    runtime = shared(ctx.tier)
    main = main_instance(ctx.tier)
    second_instance.stop(ctx.tier)
    main_names = compositor_names(runtime, main["signature"])
    socket_fixture.stale_instances(ctx.tier.runtime_dir)
    _, hangup = socket_fixture.instance(ctx.tier.runtime_dir, socket_fixture.HANGUP)
    _observe(ctx, "before", {"hyprland.desktop": main})
    try:
        problems = display_problems("discovery beside stale and hung-up instances",
                                    ctx.product(DISPLAYS, env=bare_env(ctx.tier, XDG_RUNTIME_DIR=str(runtime)),
                                                headed=False), main_names)
    finally:
        hangup.close()
    try:
        _second_instance_binds(ctx, runtime, main, main_names, problems)
    finally:
        second_instance.stop(ctx.tier)


def _second_instance_binds(ctx: LegContext, runtime: Path, main: dict, main_names: list[str],
                           problems: list[str]) -> None:
    try:
        nested = second_instance.ensure_sized(ctx.tier)
    except second_instance.OutputsNotReady as error:
        ctx.after("hyprland.second_not_ready", error.diagnostics)
        raise
    _observe(ctx, "before", {"hyprland.second_desktop": nested})
    instances = {"hyprland.desktop": main, "hyprland.second_desktop": nested}
    nested_names = compositor_names(runtime, nested["signature"])
    require(not set(main_names) & set(nested_names),
            f"the two instances share an output name: {main_names} {nested_names}")
    for label, env, expected, signature in (
        ("session pair", pair_env(ctx.tier, main["signature"], main["wayland"]), main_names, None),
        ("session signature only", pair_env(ctx.tier, main["signature"], None), main_names, None),
        ("second pair", pair_env(ctx.tier, nested["signature"], nested["wayland"]), nested_names,
         nested["signature"]),
        ("second signature only", pair_env(ctx.tier, nested["signature"], None), nested_names,
         nested["signature"]),
    ):
        envelope = ctx.product(DISPLAYS, env=env, headed=False)
        problems += display_problems(label, envelope, expected)
        if signature is not None and not envelope.ok:
            record_second_failure(ctx, runtime, signature, label, env)
    _observe(ctx, "after", instances)
    require(not problems, "; ".join(problems))
    _observe(ctx, "restored", instances)


@implementation("session.hyprland_dead_named_instance_refused")
def hyprland_dead_named_instance_refused(ctx: LegContext) -> None:
    runtime = shared(ctx.tier)
    main = main_instance(ctx.tier)
    second_instance.stop(ctx.tier)
    main_names = compositor_names(runtime, main["signature"])
    signatures = dead_signatures(ctx.tier.runtime_dir)
    reads = {"hyprland.desktop": lambda: desktop(ctx.tier, main),
             "hyprland.monitors": lambda: compositor_names(runtime, main["signature"])}
    _observe(ctx, "before", {"hyprland.desktop": main})
    problems = display_problems("discovery would bind the session instance",
                                ctx.product(DISPLAYS, env=pair_env(ctx.tier, None, None), headed=False),
                                main_names)
    with Sampler(reads) as sampler:
        for name, signature in signatures.items():
            for display_label, wayland in (("no display named", None), ("the session display", main["wayland"])):
                label = f"{name}, {display_label}"
                envelope = ctx.product(DISPLAYS, env=pair_env(ctx.tier, signature, wayland), headed=False)
                problems += refusal_problems(label, envelope, MISMATCH)
                if signature not in str(((envelope.error or {}).get("details") or {}).get("detail", "")):
                    problems.append(f"{label}: details.detail does not name {signature}: {envelope.raw[:300]}")
    problems += sampler.changes
    problems += display_problems("the session instance still binds",
                                 ctx.product(DISPLAYS, env=pair_env(ctx.tier, main["signature"], main["wayland"]),
                                             headed=False), main_names)
    _observe(ctx, "after", {"hyprland.desktop": main})
    require(not problems, "; ".join(problems))
    _observe(ctx, "restored", {"hyprland.desktop": main})


@implementation("session.hyprland_absent_keeps_generic_route")
def hyprland_absent_keeps_generic_route(ctx: LegContext) -> None:
    runtime = shared(ctx.tier)
    main = main_instance(ctx.tier)
    main_names = compositor_names(runtime, main["signature"])
    socket = str(hyprland.wayland_path(runtime, main["wayland"]))
    _observe(ctx, "before", {"hyprland.desktop": main})
    bound = ctx.product(DISPLAYS, env=pair_env(ctx.tier, main["signature"], main["wayland"]), headed=False)
    problems = display_problems("bound session", bound, main_names)
    for label, private in generic_runtimes(ctx.tier.runtime_dir).items():
        envs = {f"{label}, no signature": pair_env(ctx.tier, None, socket, private)}
        if (private / "hypr").is_dir():
            envs[f"{label}, a stale signature whose display serves no instance here"] = pair_env(
                ctx.tier, "ad_e2e_stale_dead", socket, private)
        for case, env in envs.items():
            generic = ctx.product(DISPLAYS, env=env, headed=False)
            problems += display_problems(case, generic, main_names)
            if generic.ok and bound.ok and generic.data != bound.data:
                problems.append(f"{case}: the generic route answers {generic.data}, the bound route {bound.data}")
    _observe(ctx, "after", {"hyprland.desktop": main})
    require(not problems, "; ".join(problems))
    _observe(ctx, "restored", {"hyprland.desktop": main})
