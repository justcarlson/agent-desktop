from __future__ import annotations

from runner.session import Tier


def active_window(tier: Tier) -> str | None:
    raise NotImplementedError("active_window")


def stacking(tier: Tier) -> list[str]:
    raise NotImplementedError("stacking")


def geometry(tier: Tier, window: str) -> tuple[int, int, int, int]:
    raise NotImplementedError("geometry")


def client_origin(tier: Tier, window: str) -> tuple[int, int]:
    raise NotImplementedError("client_origin")


def pointer(tier: Tier) -> tuple[int, int]:
    raise NotImplementedError("pointer")


def set_above(tier: Tier, window: str, on: bool) -> None:
    raise NotImplementedError("set_above")


def current_desktop(tier: Tier) -> int | None:
    raise NotImplementedError("current_desktop")


def overview_active(tier: Tier) -> bool:
    raise NotImplementedError("overview_active")
