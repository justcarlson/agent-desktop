from __future__ import annotations

from typing import Callable

from runner.session import Tier


class RawNode:
    role: int
    name: str
    states: frozenset[str]

    def children(self) -> list["RawNode"]:
        raise NotImplementedError("RawNode.children")

    def find(self, *, role: int | None = None, name: str | None = None) -> list["RawNode"]:
        raise NotImplementedError("RawNode.find")

    def extents(self, window_coords: bool = False) -> tuple[int, int, int, int]:
        raise NotImplementedError("RawNode.extents")

    def text(self) -> str | None:
        raise NotImplementedError("RawNode.text")

    def value(self) -> float | None:
        raise NotImplementedError("RawNode.value")

    def do_action(self, name: str) -> bool:
        raise NotImplementedError("RawNode.do_action")


def app_root(tier: Tier, owner_name: str) -> RawNode:
    raise NotImplementedError("app_root")


def poll(predicate: Callable[[], object], deadline_s: float, *, floor_s: float = 0.0):
    raise NotImplementedError("poll")
