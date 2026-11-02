"""The timeline oracle for the four-agent legs: pure checks over observed lease and effect times.

A span is one agent process: when it opened the interaction lock (enqueue),
when /proc/locks first showed it holding the lock (acquire) and no longer
holding it (release), the monotonic times its app logged an effect, and the
lease contention count its envelope reported. A hold is any lease holder,
an agent or the runner's gate. A dispatch interval is the agent's lease hold,
from acquire to release, and holds must never overlap. An app's own handler
may run after its agent releases: a GTK4 button emits `clicked` at the end
of its activate timeout and a Qt button through `animateClick`, so such an
effect must still land once and after its own agent's acquire, and its lag
past release is measured, not counted as an overlap. The negative
controls run the same oracle over mutated copies, so an oracle that stops
detecting an overlap or an unreported wait fails the leg.
"""

from __future__ import annotations

import dataclasses
import math
from dataclasses import dataclass

LEASE_PREFIXES = ("AGENT_DESKTOP_INTERACTION_LEASE", "AGENT_DESKTOP_SESSION_LEASE")
AGENT_ENV = "AGENT_DESKTOP_AGENT_ID"
DELIVERED = ("delivered_verified", "delivered_unverified", "delivery_uncertain")


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
    problems = []
    for span in spans:
        if len(span.effects) != expected:
            problems.append(f"{span.agent}: {len(span.effects)} effects, not {expected}")
        elif span.acquire is not None and any(effect < span.acquire for effect in span.effects):
            problems.append(f"{span.agent}: an effect at {min(span.effects):.4f} came before its acquire at "
                            f"{span.acquire:.4f}")
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
    return (overlap_problems(spans) + effect_problems(spans) + queueing_problems(spans, holds_of(spans, gate))
            + order_problems(spans, order))


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


def control_problems(spans: list[Span], gate: list[Hold], order) -> list[str]:
    problems = []
    if not any("overlap" in item for item in timeline_problems(with_injected_overlap(spans), gate, order)):
        problems.append("an injected overlap of two dispatches passed the oracle")
    if not any("contention" in item for item in timeline_problems(with_unreported_wait(spans), gate, order)):
        problems.append("an unreported lease wait passed the oracle")
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
