"""Cancelling agent processes at the lease barriers on the attached `hyprland` tier.

Each stage stops one agent with SIGTERM at an observed barrier: queued
behind the gate, just after it takes the lease, and in the middle of a
paced `type` once its app has logged some of the text. A cancelled waiter
sends nothing; a cancelled holder lets go of the lease and leaves no key
held; a partial delivery never reports success or a safe retry. In every
stage the next queued agent then takes the lease and lands its effect once.
Each app's log and accessible text, `/proc/locks` and the compositor's
keyboards are the evidence.
"""

from __future__ import annotations

import signal
import time

from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios import parallel_agents as agents
from scenarios import parallel_oracle as oracle
from scenarios import parallel_zoo as zoo
from scenarios.hyprland_keys_common import keyboards
from scenarios.hyprland_parallel_faults import agent_argv, effects, session_refs, strays

PACED = "abcdefghijklmnopqrst"
PACED_MS = "150"
PARTIAL_AT = 3


def _cancelled(agent: agents.Agent) -> list[str]:
    agent.signal(signal.SIGTERM)
    agent.finish()
    if agent.exit_code != -signal.SIGTERM:
        return [f"{agent.name} exited {agent.exit_code}, not by SIGTERM: the barrier passed before the signal"]
    return []


def _next_proceeds(gate, sampler, launched, agent: agents.Agent, since: float, allowed) -> list[str]:
    agents.turn(gate, sampler, agent, [agent])
    agent.finish()
    problems = agents.outcome_problems(agent)
    agents.settle(lambda: effects(launched, agent.name, since))
    landed = effects(launched, agent.name, since)
    allowed.setdefault(agent.name, []).extend(landed)
    if len(landed) != 1:
        problems.append(f"the next agent {agent.name} landed {len(landed)} effects, not 1")
    return problems


def queued_stage(tier, gate, sampler, launched, refs, session: str, allowed) -> list[str]:
    """A waiter cancelled behind the gate never takes the lease and never reaches its app."""
    since = time.monotonic()
    waiter = agents.spawn(tier, sampler, "A", agent_argv("A", refs["A"]), session=session)
    after = agents.spawn(tier, sampler, "D", agent_argv("D", refs["D"]), session=session, headed=True)
    with agents.reaped([waiter, after]):
        agents.queued(sampler, [waiter, after], "the waiter and the next agent queue")
        problems = [f"queued: {item}" for item in _cancelled(waiter)]
        problems += [f"queued: {item}" for item in _next_proceeds(gate, sampler, launched, after, since, allowed)]
    if "acquire" in sampler.events().get(waiter.pid, {}):
        problems.append("queued: the cancelled waiter took the lease")
    if effects(launched, "A", since):
        problems.append("queued: the cancelled waiter reached its app")
    return problems


def acquired_stage(tier, gate, sampler, launched, refs, session: str, allowed) -> list[str]:
    """A holder cancelled as it takes the lease lets go of it, and the next agent proceeds."""
    since = time.monotonic()
    holder = agents.spawn(tier, sampler, "D", agent_argv("D", refs["D"]), session=session, headed=True)
    after = agents.spawn(tier, sampler, "A", agent_argv("A", refs["A"]), session=session)
    with agents.reaped([holder, after]):
        agents.queued(sampler, [holder, after], "the holder and the next agent queue")
        problems = []

        def cancel() -> None:
            problems.extend(f"acquired: {item}" for item in _cancelled(holder))

        agents.turn(gate, sampler, holder, [holder, after], during=cancel)
        problems += [f"acquired: {item}" for item in _next_proceeds(gate, sampler, launched, after, since, allowed)]
    landed = effects(launched, "D", since)
    allowed.setdefault("D", []).extend(landed)
    if len(landed) > 1:
        problems.append(f"acquired: the cancelled holder landed {len(landed)} clicks")
    return problems


def partial_stage(ctx: LegContext, gate, sampler, launched, refs, session: str, allowed) -> list[str]:
    """A paced `type` cancelled mid-text leaves a strict prefix, no held key and no safe retry."""
    tier = ctx.tier
    before = keyboards(tier)
    zoo.focus_entry(tier, launched["B"])
    since = time.monotonic()
    argv = [*agent_argv("B", refs["B"], text=PACED), "--interval-ms", PACED_MS]
    typist = agents.spawn(tier, sampler, "B", argv, session=session, headed=True)
    after = agents.spawn(tier, sampler, "A", agent_argv("A", refs["A"]), session=session)
    with agents.reaped([typist, after]):
        agents.queued(sampler, [typist, after], "the typist and the next agent queue")
        problems = []

        def cancel() -> None:
            agents.until(lambda: len(effects(launched, "B", since)) >= PARTIAL_AT or typist.process.poll() is not None,
                         f"the entry logs {PARTIAL_AT} changes")
            problems.extend(f"partial: {item}" for item in _cancelled(typist))

        agents.turn(gate, sampler, typist, [typist, after], during=cancel)
        problems += [f"partial: {item}" for item in oracle.partial_retry_problems(typist.envelope)]
        problems += [f"partial: {item}" for item in _next_proceeds(gate, sampler, launched, after, since, allowed)]
    allowed.setdefault("B", []).extend(effects(launched, "B", since))
    typed = zoo.entry_text(tier, launched["B"]) or ""
    if not typed or typed == PACED or not PACED.startswith(typed):
        problems.append(f"partial: the entry reads {typed!r}, not a strict prefix of the text")
    if keyboards(tier) != before:
        problems.append(f"partial: the keyboards changed: {before} to {keyboards(tier)}")
    ctx.finding(f"the cancelled paced type left {typed!r} in the entry")
    return problems


def final_state(launched) -> str:
    counts = {name: len(effects(launched, name, 0.0)) for name in launched}
    return f"final effects per app: {counts}"


@implementation("parallel.cancel_waiter_and_holder")
def cancel_waiter_and_holder(ctx: LegContext) -> None:
    tier = ctx.tier
    ctx.before("hyprland.desktop", zoo.desktop_of(tier))
    zoo.common_reads(ctx, "before")
    problems, allowed = [], {}
    with zoo.zoos(tier, "ABD") as launched, zoo.multi_session(ctx) as session:
        with zoo.shown_desktop(ctx, launched, "ABD") as (holder, point):
            refs = session_refs(ctx, launched, session)
            ctx.before("applog.strays", [])
            with agents.released_lease(), agents.LockSampler() as sampler:
                gate = agents.Gate()
                try:
                    gate.take()
                    problems += queued_stage(tier, gate, sampler, launched, refs, session, allowed)
                    problems += acquired_stage(tier, gate, sampler, launched, refs, session, allowed)
                    problems += partial_stage(ctx, gate, sampler, launched, refs, session, allowed)
                finally:
                    gate.close()
            zoo.note_layers(ctx, problems)
            ctx.finding(final_state(launched))
            found = strays(launched, allowed)
            ctx.after("applog.strays", found)
            problems += found
            zoo.restore_desktop(tier, holder, point)
            ctx.after("hyprland.desktop", zoo.desktop_of(tier))
            zoo.common_reads(ctx, "after")
            ctx.restored("applog.strays", strays(launched, allowed))
    zoo.common_reads(ctx, "restored")
    ctx.restored("hyprland.desktop", zoo.desktop_of(tier))
    require(not problems, "; ".join(problems))
