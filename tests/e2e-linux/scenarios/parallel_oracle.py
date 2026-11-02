"""The timeline oracle for the four-agent legs: pure checks over observed lease and effect times.

A span is one agent process: when it opened the interaction lock (enqueue),
when /proc/locks first showed it holding the lock (acquire) and no longer
holding it (release), the monotonic times its app logged an effect, and the
lease contention count its envelope reported. A hold is any lease holder,
an agent or the runner's gate. A dispatch interval is the agent's lease
hold, from acquire to release, and holds must never overlap.

The timestamped app logs take part in the verdict:

1. Each effect an app logs is at or after the start of its own agent's
   hold. An earlier effect is input delivered outside the lease.
2. An app's handler may run after its agent releases: a GTK4 button emits
   `clicked` at the end of its 250 ms activate timeout and a Qt button
   through `animateClick`. That lag, from release to the effect, is
   reported and must stay within `LAG_BOUND_MS`.
3. Each app logs its effects in the order of its own agent's holds: every
   effect of one hold comes before the next hold of that agent starts.
   Effects of different apps are not ordered against each other, because a
   lagging toolkit's effect can land inside a later agent's hold.

The negative controls run the same oracle over mutated copies: an injected
overlap of two holds, an unreported wait, and an effect moved inside
another agent's hold before its own. An oracle that stops detecting any of
them fails the leg.
"""

from __future__ import annotations

import dataclasses
import math
from dataclasses import dataclass

LEASE_PREFIXES = ("AGENT_DESKTOP_INTERACTION_LEASE", "AGENT_DESKTOP_SESSION_LEASE")
AGENT_ENV = "AGENT_DESKTOP_AGENT_ID"
DELIVERED = ("delivered_verified", "delivered_unverified", "delivery_uncertain")
LAG_BOUND_MS = 1000.0


@dataclass(frozen=True)
class Span:
    agent: str
    enqueue: float | None
    acquire: float | None
    release: float | None
    effects: tuple[float, ...] = ()
    contention: int | None = None


@dataclass(frozen=True)
class Hold:
    holder: str
    start: float
    end: float


def dispatch_interval(span: Span) -> tuple[float, float] | None:
    if span.acquire is None or span.release is None:
        return None
    return span.acquire, span.release


def effect_lags_ms(spans: list[Span]) -> dict[str, float]:
    """Per agent, the longest time its app's effect landed after the agent released the lease."""
    lags: dict[str, float] = {}
    for span in spans:
        late = [effect - span.release for effect in span.effects
                if span.release is not None and effect > span.release]
        if late:
            lags[span.agent] = max(lags.get(span.agent, 0.0), max(late) * 1000.0)
    return lags


def overlap_problems(spans: list[Span]) -> list[str]:
    problems = [f"{span.agent}: no observed lease acquire and release" for span in spans
                if dispatch_interval(span) is None]
    timed = sorted((dispatch_interval(span), span.agent) for span in spans if dispatch_interval(span))
    for index, (first, first_agent) in enumerate(timed):
        for second, second_agent in timed[index + 1:]:
            if second[0] < first[1]:
                problems.append(f"{first_agent} and {second_agent} dispatch intervals overlap: "
                                f"{_interval(first)} and {_interval(second)}")
    return problems


def effect_problems(spans: list[Span], expected: int = 1) -> list[str]:
    problems = [f"{span.agent}: {len(span.effects)} effects, not {expected}" for span in spans
                if len(span.effects) != expected]
    return problems + early_effect_problems(spans)


def early_effect_problems(spans: list[Span]) -> list[str]:
    """An effect logged before its own agent's hold started was delivered outside the lease."""
    return [f"{span.agent}: an effect at {min(span.effects):.4f} came before its acquire at {span.acquire:.4f}"
            for span in spans if span.acquire is not None and span.effects and min(span.effects) < span.acquire]


def lag_problems(spans: list[Span], bound_ms: float = LAG_BOUND_MS) -> list[str]:
    """An effect that lands longer than `bound_ms` after its agent released the lease."""
    return [f"{span.agent}: an effect landed {(effect - span.release) * 1000.0:.0f} ms after its release, "
            f"past the {bound_ms:.0f} ms bound"
            for span in spans if span.release is not None
            for effect in span.effects if (effect - span.release) * 1000.0 > bound_ms]


def effect_order_problems(spans: list[Span]) -> list[str]:
    """Per app, every effect of one hold comes before that agent's next hold starts."""
    problems = []
    for agent in sorted({span.agent for span in spans}):
        held = sorted((span for span in spans if span.agent == agent and span.acquire is not None),
                      key=lambda span: span.acquire)
        for first, second in zip(held, held[1:]):
            if first.effects and (max(first.effects) >= second.acquire
                                  or (second.effects and max(first.effects) > min(second.effects))):
                problems.append(f"{agent}: an effect of the hold at {first.acquire:.4f} landed at "
                                f"{max(first.effects):.4f}, after the next hold began at {second.acquire:.4f}")
    return problems


def holds_of(spans: list[Span], gate: list[Hold]) -> list[Hold]:
    agents = [Hold(span.agent, span.acquire, span.release) for span in spans
              if span.acquire is not None and span.release is not None]
    return sorted([*gate, *agents], key=lambda hold: (hold.start, hold.end))


def queueing_problems(spans: list[Span], holds: list[Hold]) -> list[str]:
    problems = []
    for span in spans:
        if span.enqueue is None or span.acquire is None:
            continue
        behind = [hold.holder for hold in holds
                  if hold.holder != span.agent and hold.start < span.acquire and hold.end > span.enqueue]
        if span.contention is None:
            problems.append(f"{span.agent}: the envelope reports no lease contention count")
        elif behind and span.contention < 1:
            problems.append(f"{span.agent}: waited {wait_ms(span):.1f} ms behind {behind} but reports "
                            f"lease contention {span.contention}")
        elif not behind and span.contention > 0:
            problems.append(f"{span.agent}: reports lease contention {span.contention} but no other holder "
                            f"held the lease during its wait")
    return problems


def acquire_order(spans: list[Span]) -> str:
    return "".join(span.agent for span in sorted(spans, key=lambda span: span.acquire or 0.0)
                   if span.acquire is not None)


def order_problems(spans: list[Span], expected) -> list[str]:
    wanted = "".join(expected)
    found = acquire_order(spans)
    return [] if found == wanted else [f"the lease went to {found!r}, not {wanted!r}"]


def wait_ms(span: Span) -> float | None:
    if span.enqueue is None or span.acquire is None:
        return None
    return (span.acquire - span.enqueue) * 1000.0


def percentile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    return ordered[max(1, math.ceil(fraction * len(ordered))) - 1]


def timeline_problems(spans: list[Span], gate: list[Hold], order) -> list[str]:
    return (overlap_problems(spans) + effect_problems(spans) + lag_problems(spans)
            + queueing_problems(spans, holds_of(spans, gate)) + order_problems(spans, order))


def with_injected_overlap(spans: list[Span]) -> list[Span]:
    """A copy where the second agent to acquire did so halfway through the first agent's hold."""
    held = sorted((span for span in spans if dispatch_interval(span)), key=lambda span: span.acquire)
    if len(held) < 2:
        raise ValueError("an overlap needs two observed dispatches")
    first, second = held[0], held[1]
    moved = dataclasses.replace(second, acquire=(first.acquire + dispatch_interval(first)[1]) / 2)
    return [moved if span is second else span for span in spans]


def with_unreported_wait(spans: list[Span]) -> list[Span]:
    """A copy where every envelope reports no lease contention."""
    return [dataclasses.replace(span, contention=0) if span.contention is not None else span for span in spans]


def with_effect_in_foreign_hold(spans: list[Span]) -> list[Span]:
    """A copy where the second agent to acquire logged its effect halfway through the first agent's hold."""
    held = sorted((span for span in spans if dispatch_interval(span)), key=lambda span: span.acquire)
    if len(held) < 2:
        raise ValueError("a foreign hold needs two observed dispatches")
    first, second = held[0], held[1]
    moved = dataclasses.replace(second, effects=(round((first.acquire + first.release) / 2, 6),))
    return [moved if span is second else span for span in spans]


def control_problems(spans: list[Span], gate: list[Hold], order) -> list[str]:
    problems = []
    if not any("overlap" in item for item in timeline_problems(with_injected_overlap(spans), gate, order)):
        problems.append("an injected overlap of two dispatches passed the oracle")
    if not any("contention" in item for item in timeline_problems(with_unreported_wait(spans), gate, order)):
        problems.append("an unreported lease wait passed the oracle")
    moved = with_effect_in_foreign_hold(spans)
    agent = sorted((span for span in spans if dispatch_interval(span)), key=lambda span: span.acquire)[1].agent
    if not any(item.startswith(f"{agent}: an effect at") for item in timeline_problems(moved, gate, order)):
        problems.append("an effect inside another agent's hold passed the oracle")
    return problems


def lock_holders(text: str, inode: int) -> set[int]:
    """The pids /proc/locks shows holding a flock on `inode`; blocked waiters are skipped."""
    found = set()
    for line in text.splitlines():
        fields = line.split()
        if len(fields) < 6 or "->" in fields or fields[1] != "FLOCK":
            continue
        if int(fields[5].rsplit(":", 1)[1]) == inode:
            found.add(int(fields[4]))
    return found


def lease_events(samples) -> dict[int, dict]:
    """Per pid, the first sample that saw it queued, holding, and no longer holding.

    `samples` is a time-ordered list of (time, holders, queued) where queued
    holds every pid with the lock file open. A pid that held the lease more
    than once also gets `holds`.
    """
    found: dict[int, dict] = {}
    counts: dict[int, int] = {}
    holding: set[int] = set()
    for moment, holders, queued in samples:
        for pid in set(queued) | set(holders):
            found.setdefault(pid, {"enqueue": moment})
        for pid in set(holders) - holding:
            counts[pid] = counts.get(pid, 0) + 1
            found[pid].setdefault("acquire", moment)
            if counts[pid] > 1:
                found[pid]["holds"] = counts[pid]
        for pid in holding - set(holders):
            found[pid].setdefault("release", moment)
        holding = set(holders)
    return found


def hold_intervals(samples, names: dict[int, str]) -> list[Hold]:
    """Every lease hold the samples show, named by `names` or by pid; a hold still open never ends."""
    found: list[Hold] = []
    since: dict[int, float] = {}
    for moment, holders, _queued in samples:
        for pid in set(holders) - set(since):
            since[pid] = moment
        for pid in set(since) - set(holders):
            found.append(Hold(names.get(pid, f"pid {pid}"), since.pop(pid), moment))
    found += [Hold(names.get(pid, f"pid {pid}"), start, math.inf) for pid, start in since.items()]
    return sorted(found, key=lambda hold: (hold.start, hold.end))


def lock_timeout_problems(envelope, kind: str) -> list[str]:
    if envelope.ok:
        return [f"the contender succeeded while the lease was held: {envelope.raw[:400]}"]
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.exit_code != 1 or error.get("code") != "TIMEOUT":
        problems.append(f"the contender exited {envelope.exit_code} with {error.get('code')}, not TIMEOUT and 1")
    if details.get("kind") != kind:
        problems.append(f"the timeout kind is {details.get('kind')!r}, not {kind!r}")
    if not isinstance(details.get("contention_count"), int) or details["contention_count"] < 1:
        problems.append(f"the timeout counts contention {details.get('contention_count')!r}")
    if (error.get("disposition") or {}).get("delivery") in DELIVERED:
        problems.append(f"the timeout claims delivery: {error.get('disposition')}")
    return problems


def ref_refusal_problems(envelope) -> list[str]:
    """A headed ref click on an XWayland target refuses before any event: ACTION_FAILED with the
    `receiver_unproven` cause, no physical delivery started and a not-delivered disposition."""
    if envelope is None:
        return ["the headed ref click printed no envelope"]
    if envelope.ok:
        return [f"the headed ref click succeeded: {envelope.raw[:400]}"]
    error = envelope.error or {}
    details = error.get("details") or {}
    found = (envelope.exit_code, error.get("code"), details.get("kind"), details.get("cause"),
             details.get("physical_delivery_started"), (error.get("disposition") or {}).get("delivery"))
    expected = (1, "ACTION_FAILED", "receiver_evidence", "receiver_unproven", False, "not_delivered")
    if found != expected:
        return [f"the headed ref click refused as {found}, not {expected}"]
    return []


def partial_retry_problems(envelope) -> list[str]:
    """A cancelled partial delivery may print nothing, but never success or a safe retry."""
    if envelope is None:
        return []
    if envelope.ok:
        return [f"the cancelled partial delivery reported success: {envelope.raw[:400]}"]
    if ((envelope.error or {}).get("disposition") or {}).get("retry") == "safe":
        return [f"the cancelled partial delivery reports a safe retry: {envelope.raw[:400]}"]
    return []


def reported_contention(envelope) -> int | None:
    if envelope is None:
        return None
    auto_wait = ((envelope.data or {}).get("details") or {}).get("auto_wait") or {}
    found = auto_wait.get("lease_contention_count")
    return found if isinstance(found, int) else None


def child_env(source: dict[str, str], agent: str) -> dict[str, str]:
    """An agent child's environment: no inherited lease token or descriptor, and its own agent id."""
    env = {key: value for key, value in source.items() if not key.startswith(LEASE_PREFIXES)}
    env[AGENT_ENV] = agent
    return env


def _interval(pair: tuple[float, float]) -> str:
    return f"[{pair[0]:.4f}, {pair[1]:.4f}]"
