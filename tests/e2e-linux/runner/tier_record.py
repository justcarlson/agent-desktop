"""The attached tier's run record, read only where the fork installs that tier."""
from __future__ import annotations

import importlib.util
from pathlib import Path


def installed() -> bool:
    """Whether the fork-only attached tier is present; upstream ships the harness without it."""
    return importlib.util.find_spec("runner.attached") is not None


def load(runtime_dir: Path) -> dict | None:
    """The attached record for `runtime_dir`, or None for a plain tier or a harness without the attached tier."""
    if not installed():
        return None
    from runner import attached

    return attached.load(runtime_dir)
