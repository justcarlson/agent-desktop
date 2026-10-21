from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

from runner.legs import ORACLE_NAMES
from runner.product import Envelope, run_product
from runner.session import Tier, _proc_members


@dataclass(frozen=True)
class OwnedSet:
    members: frozenset[tuple[int, int]]


def _session_members(tier: Tier) -> frozenset[tuple[int, int]]:
    return frozenset(_proc_members(tier.runtime_dir))


class LegContext:
    def __init__(
        self,
        tier: Tier,
        mode: str,
        leg=None,
        member_reader: Callable[[Tier], frozenset[tuple[int, int]]] | None = None,
    ) -> None:
        self.tier = tier
        self.mode = mode
        self.leg = leg
        self._member_reader = member_reader or _session_members
        self._baseline = self.owned()
        self._reads: dict[str, dict[str, list[object]]] = {}
        self._findings: list[str] = []
        self._products: list[Envelope] = []
        self.exception_traceback = ""

    def _record(self, stage: str, channel: str, value: object) -> None:
        family = channel.partition(".")[0]
        if family not in ORACLE_NAMES:
            raise ValueError(f"unknown oracle channel {channel}")
        if self.leg is not None and family not in self.leg.oracles:
            raise ValueError(
                f"leg {self.leg.id} read oracle channel {channel} it did not declare"
            )
        self._reads.setdefault(channel, {}).setdefault(stage, []).append(value)

    def before(self, channel: str, value: object) -> None:
        self._record("before", channel, value)

    def after(self, channel: str, value: object) -> None:
        self._record("after", channel, value)

    def restored(self, channel: str, value: object) -> None:
        self._record("restored", channel, value)

    def product(self, argv: list[str], **kw) -> Envelope:
        kw.setdefault("headed", self.mode == "headed")
        envelope = run_product(self.tier, argv, **kw)
        self._products.append(envelope)
        return envelope

    def owned(self) -> OwnedSet:
        return OwnedSet(members=self._member_reader(self.tier))

    def finding(self, text: str) -> None:
        self._findings.append(str(text))

    @property
    def findings(self) -> list[str]:
        return list(self._findings)

    @property
    def products(self) -> list[Envelope]:
        return list(self._products)

    @property
    def reads(self) -> dict[str, dict[str, list[object]]]:
        return {channel: {stage: list(values) for stage, values in entry.items()}
                for channel, entry in self._reads.items()}

    @property
    def baseline(self) -> OwnedSet:
        return self._baseline


def evaluate(ctx: LegContext) -> list[str]:
    failures = []
    reads = ctx._reads
    if not any(entry.get("before") for entry in reads.values()):
        failures.append("no oracle recorded a before read")
    if not any(entry.get("after") for entry in reads.values()):
        failures.append("no oracle recorded an after read")
    for channel in sorted(reads):
        entry = reads[channel]
        befores = entry.get("before", [])
        afters = entry.get("after", [])
        restores = entry.get("restored", [])
        if not befores:
            failures.append(f"oracle {channel} is missing its before read")
            continue
        if not afters:
            failures.append(f"oracle {channel} is missing its after read")
            continue
        changed = any(before != after for before, after in zip(befores, afters))
        if not changed:
            if restores and any(restored not in befores for restored in restores):
                failures.append(
                    f"oracle {channel} drifted: the restoration read disagrees with the before read"
                )
        elif not restores:
            failures.append(
                f"oracle {channel} changed during the leg but recorded no restoration read"
            )
        elif any(restored not in befores for restored in restores):
            failures.append(
                f"oracle {channel} changed during the leg and was not restored"
            )
    current = ctx.owned()
    extra = sorted(current.members - ctx._baseline.members)
    gone = sorted(ctx._baseline.members - current.members)
    if extra or gone:
        detail = []
        if extra:
            detail.append(f"joined {[pid for pid, _ in extra]}")
        if gone:
            detail.append(f"left {[pid for pid, _ in gone]}")
        failures.append(f"the owned-process set changed during the leg ({'; '.join(detail)})")
    return failures
