"""Shared reads and verdicts for the legs on the attached `hyprland` tier."""

from __future__ import annotations

import threading
from pathlib import Path

from oracle import hyprland
from runner import attached


def record(tier) -> dict:
    found = attached.load(tier.runtime_dir)
    if found is None:
        raise AssertionError("the hyprland tier has no attached record")
    return found


def shared(tier) -> Path:
    return Path(record(tier)["shared_runtime"])


def main_instance(tier) -> dict:
    found = record(tier)
    return {"signature": found["signature"], "pid": found["compositor"][0], "wayland": tier.wayland_display}


def desktop(tier, instance: dict) -> dict:
    return hyprland.desktop(shared(tier), instance["signature"], pid=instance["pid"])


def bystander(tier) -> dict:
    found = record(tier).get("bystander")
    if not found:
        raise AssertionError("the session has no bystander window the runner did not launch")
    return found


class Sampler:
    """Reads desktops while a leg works, so a transient change is caught too."""

    def __init__(self, reads: dict, interval_s: float = 0.05):
        self.reads = reads
        self.baseline = {name: read() for name, read in reads.items()}
        self.changes: list[str] = []
        self._stop = threading.Event()
        self._interval = interval_s
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self) -> None:
        while not self._stop.wait(self._interval):
            self.sample()

    def sample(self) -> None:
        for name, read in self.reads.items():
            try:
                value = read()
            except (OSError, RuntimeError, ValueError) as error:
                self.changes.append(f"{name} could not be read: {error}")
                continue
            if value != self.baseline[name] and len(self.changes) < 20:
                self.changes.append(f"{name} changed to {value!r}")

    def __enter__(self):
        self._thread.start()
        return self

    def __exit__(self, *exc):
        self._stop.set()
        self._thread.join(5.0)
        self.sample()
        return False
