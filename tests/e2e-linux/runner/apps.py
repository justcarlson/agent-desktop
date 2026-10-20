from __future__ import annotations

from dataclasses import dataclass

from runner.session import Tier


@dataclass
class OwnedApp:
    name: str
    pid: int
    start_time: int
    owner_names: list[str]
    windows: list[str]
    display: str


def launch(tier: Tier, app: str, args: tuple[str, ...] = (), *, settle_s: float = 5.0) -> OwnedApp:
    raise NotImplementedError("launch")


def owners_named(tier: Tier, name: str) -> list[OwnedApp]:
    raise NotImplementedError("owners_named")


def stop(app: OwnedApp) -> None:
    raise NotImplementedError("stop")


def launch_qt(tier: Tier) -> OwnedApp:
    raise NotImplementedError("launch_qt")


def launch_vscode(tier: Tier) -> OwnedApp:
    raise NotImplementedError("launch_vscode")
