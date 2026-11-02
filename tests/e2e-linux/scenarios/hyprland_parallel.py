"""Four agent processes in one multi-agent session on the attached `hyprland` tier.

The runner releases its interaction lease for a lease phase, and each agent
child, started with a distinct agent id and no inherited lease token or
descriptor, takes the real per-user lease itself. A gate descriptor holds
the lease so the children queue; a sampler reads `/proc/locks` and each
child's descriptor table about every millisecond, and each app logs its
effects with monotonic times. The timeline oracle checks that dispatch
intervals never overlap, that every envelope's lease contention matches the
observed wait, and, as the negative control, that an injected overlap and an
unreported wait both fail it. The command's own `ok` is never the evidence.
"""

from __future__ import annotations

import time

from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios import parallel_agents as agents
from scenarios import parallel_oracle as oracle
from scenarios import parallel_zoo as zoo
from scenarios.hyprland_parallel_orders import (ORDERS, Schedule, prepared, ref_click_refused,
                                                 refused_without_agent)

ROUNDS = 6


def _click(ref: str) -> list[str]:
    return ["click", ref, "--timeout-ms", agents.AGENT_TIMEOUT_MS]


def _round(tier, gate, sampler, launched, refs, session: str, index: int):
    gate.take()
    since = time.monotonic()
    group = [agents.spawn(tier, sampler, name, _click(refs[name]), session=session) for name in launched]
    with agents.reaped(group):
        agents.queued(sampler, group, f"round {index}: all four agents queue")
        gate.give()
        for agent in group:
            agent.finish()

    def landed(name: str) -> tuple[float, ...]:
        return agents.effect_times(launched[name].log, zoo.BUTTON, zoo.ACTIVATE, since)

    agents.settle(lambda: all(landed(agent.name) for agent in group))
    events = sampler.events()
    spans = [agents.span_of(agent, events, landed(agent.name)) for agent in group]
    gate_holds = [hold for hold in sampler.holds(agents.holder_names(group)) if hold.holder == "runner"]
    order = oracle.acquire_order(spans)
    problems = [problem for agent in group for problem in agents.outcome_problems(agent)]
    problems += oracle.timeline_problems(spans, gate_holds, order)
    problems += oracle.control_problems(spans, gate_holds, order)
    return spans, [f"round {index}: {problem}" for problem in problems]


def _strays(launched, spans) -> list[str]:
    allowed: dict[str, list[float]] = {}
    for span in spans:
        allowed.setdefault(span.agent, []).extend(span.effects)
    return [problem for item in launched.values() for problem in zoo.stray_events(item, allowed)]


def wait_summary(spans) -> str:
    waits = [wait for wait in (oracle.wait_ms(span) for span in spans) if wait is not None]
    p50, p95 = oracle.percentile(waits, 0.5), oracle.percentile(waits, 0.95)
    require(p50 is not None and p95 is not None, "no agent run had an observed lease wait")
    return f"lease wait over {len(waits)} agent runs: p50 {p50:.1f} ms, p95 {p95:.1f} ms"


def lag_summary(spans) -> str:
    lags = {name: f"{lag:.0f} ms" for name, lag in sorted(oracle.effect_lags_ms(spans).items())}
    return f"longest app effect after its agent released the lease, per agent: {lags or 'none'}"


@implementation("parallel.four_agents_serialized_dispatch")
def four_agents_serialized_dispatch(ctx: LegContext) -> None:
    tier = ctx.tier
    before = zoo.desktop_of(tier)
    ctx.before("hyprland.desktop", before)
    zoo.common_reads(ctx, "before")
    spans, problems = [], []
    with zoo.zoos(tier, "ABCD") as launched, zoo.multi_session(ctx) as session:
        refs = {name: agents.zoo_ref(ctx, session, item.app, zoo.BUTTON, None) for name, item in launched.items()}
        ctx.before("applog.strays", [])
        with agents.released_lease(), agents.LockSampler() as sampler:
            gate = agents.Gate()
            try:
                for index in range(ROUNDS):
                    found, failed = _round(tier, gate, sampler, launched, refs, session, index)
                    spans += found
                    problems += failed
            finally:
                gate.close()
        problems += oracle.effect_order_problems(spans)
        ctx.after("applog.strays", _strays(launched, spans))
        problems += _strays(launched, spans)
        after = zoo.desktop_of(tier)
        ctx.after("hyprland.desktop", after)
        problems += [] if after == before else [f"headless agents changed the desktop: {before} to {after}"]
        zoo.common_reads(ctx, "after")
        ctx.restored("applog.strays", _strays(launched, spans))
    ctx.finding(wait_summary(spans))
    ctx.finding(lag_summary(spans))
    zoo.common_reads(ctx, "restored")
    ctx.restored("hyprland.desktop", zoo.desktop_of(tier))
    require(not problems, "; ".join(problems))


@implementation("parallel.four_agents_three_interleavings")
def four_agents_three_interleavings(ctx: LegContext) -> None:
    tier = ctx.tier
    ctx.before("hyprland.desktop", zoo.desktop_of(tier))
    zoo.common_reads(ctx, "before")
    problems, spans = [], []
    with zoo.zoos(tier, "ABD") as launched, zoo.multi_session(ctx) as session:
        with zoo.shown_desktop(ctx, launched, "ABD") as (holder, point):
            setup = prepared(ctx, launched, session, holder, point)
            ctx.before("applog.strays", [])
            problems += refused_without_agent(ctx, setup)
            with agents.released_lease(), agents.LockSampler() as sampler:
                problems += ref_click_refused(setup, sampler)
                gate = agents.Gate()
                try:
                    for label, run in ORDERS:
                        schedule = Schedule(setup, gate, sampler, label)
                        problems += schedule.run(run)
                        spans += schedule.spans
                finally:
                    gate.close()
            problems += oracle.effect_order_problems(spans)
            problems += setup.final_problems()
            zoo.note_layers(ctx, problems)
            ctx.after("applog.strays", _strays(launched, spans))
            problems += _strays(launched, spans)
            ctx.after("hyprland.desktop", zoo.desktop_of(tier))
            zoo.common_reads(ctx, "after")
            ctx.restored("applog.strays", _strays(launched, spans))
        ctx.finding(wait_summary(spans))
        ctx.finding(lag_summary(spans))
    zoo.common_reads(ctx, "restored")
    ctx.restored("hyprland.desktop", zoo.desktop_of(tier))
    require(not problems, "; ".join(problems))
