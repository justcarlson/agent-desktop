from __future__ import annotations

from runner.session import Tier


def foreign_set(tier: Tier, fmt: str, data: bytes) -> int:
    raise NotImplementedError("foreign_set")


def foreign_get(tier: Tier, fmt: str) -> bytes | None:
    raise NotImplementedError("foreign_get")
