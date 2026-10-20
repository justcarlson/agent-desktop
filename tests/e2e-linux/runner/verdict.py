from __future__ import annotations

from dataclasses import dataclass

from runner.product import Envelope
from runner.session import Tier


@dataclass(frozen=True)
class OwnedSet:
    members: frozenset[tuple[int, int]]


class LegContext:
    def __init__(self, tier: Tier, mode: str) -> None:
        self.tier = tier
        self.mode = mode

    def before(self, channel: str, value: object) -> None:
        raise NotImplementedError("LegContext.before")

    def after(self, channel: str, value: object) -> None:
        raise NotImplementedError("LegContext.after")

    def restored(self, channel: str, value: object) -> None:
        raise NotImplementedError("LegContext.restored")

    def product(self, argv: list[str], **kw) -> Envelope:
        raise NotImplementedError("LegContext.product")

    def owned(self) -> OwnedSet:
        raise NotImplementedError("LegContext.owned")

    def finding(self, text: str) -> None:
        raise NotImplementedError("LegContext.finding")
