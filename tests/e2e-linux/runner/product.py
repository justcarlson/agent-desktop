from __future__ import annotations

from dataclasses import dataclass

from runner.session import Tier


@dataclass
class Envelope:
    ok: bool
    data: dict | None
    error: dict | None
    raw: str
    elapsed_ms: float
    argv: list[str]


def run_product(
    tier: Tier,
    argv: list[str],
    *,
    headed: bool = False,
    timeout_s: float = 30.0,
    window_id: str | None = None,
) -> Envelope:
    raise NotImplementedError("run_product")
