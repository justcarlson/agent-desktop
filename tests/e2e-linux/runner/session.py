from __future__ import annotations

import contextlib
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Tier:
    name: str
    runtime_dir: Path
    display: str | None
    wayland_display: str | None
    bus_address: str
    a11y_bus_address: str
    state_root: Path
    keyfile: Path
    env: dict[str, str]


def start_tier(name: str, deadline: float) -> contextlib.AbstractContextManager[Tier]:
    raise NotImplementedError("start_tier")
