from __future__ import annotations

from runner.session import Tier


def get(tier: Tier, schema: str, key: str) -> str:
    raise NotImplementedError("get")


def set(tier: Tier, schema: str, key: str, value: str) -> None:
    raise NotImplementedError("set")
