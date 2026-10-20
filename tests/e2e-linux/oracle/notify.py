from __future__ import annotations

from pathlib import Path

from runner.session import Tier


def send(
    tier: Tier,
    app: str,
    summary: str,
    body: str = "",
    actions: tuple[tuple[str, str], ...] = (),
) -> int:
    raise NotImplementedError("send")


def monitor(tier: Tier, path: Path) -> int:
    raise NotImplementedError("monitor")
