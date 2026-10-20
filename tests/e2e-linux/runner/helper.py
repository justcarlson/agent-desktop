from __future__ import annotations

from typing import Callable


class Trigger:
    fired_at: float | None

    def join(self) -> None:
        raise NotImplementedError("Trigger.join")


def delayed(delay_s: float, action: Callable[[], None]) -> Trigger:
    raise NotImplementedError("delayed")
