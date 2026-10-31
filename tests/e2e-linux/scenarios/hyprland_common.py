"""Shared reads and verdicts for the legs on the attached `hyprland` tier."""

from __future__ import annotations

import os
import threading
from pathlib import Path

from oracle import hyprland
from runner import attached

BUDGET_MS = 2000.0
DEFAULT_RUNTIME = Path(f"/run/user/{os.getuid()}")


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


def expected_displays(monitors: list[dict]) -> list[tuple[str, bool]]:
    """What `list-displays` must report for a `j/monitors` reply: names, lowest id primary."""
    ordered = sorted(monitors, key=lambda item: item["id"])
    return [(item["name"], index == 0) for index, item in enumerate(ordered)]


def monitors(tier, instance: dict) -> list[tuple[str, bool]]:
    return expected_displays(hyprland.query(shared(tier), instance["signature"], "monitors",
                                            pid=instance["pid"]))


def reported_displays(envelope) -> list[tuple[str, bool]]:
    if not envelope.ok or not isinstance(envelope.data, list):
        raise AssertionError(f"list-displays failed: {envelope.raw}")
    return [(item.get("id"), item.get("is_primary") is True) for item in envelope.data]


def bare_env(tier, **values: str) -> dict[str, str]:
    """An SSH-like environment: no display, bus or compositor variables."""
    env = {"PATH": tier.env.get("PATH", os.defpath), "HOME": tier.env.get("HOME", str(Path.home())),
           "AGENT_DESKTOP_HOME": str(tier.state_root)}
    env.update({key: value for key, value in values.items() if value is not None})
    return env


def pair_env(tier, signature: str | None, wayland: str | None, runtime: Path | None = None) -> dict[str, str]:
    return bare_env(tier, XDG_RUNTIME_DIR=str(runtime or shared(tier)),
                    HYPRLAND_INSTANCE_SIGNATURE=signature, WAYLAND_DISPLAY=wayland)


def refusal_problems(label: str, envelope, code: str, cause: str, *, physical: bool = False) -> list[str]:
    """The refusal contract: code, cause, exit 1, not delivered, and no physical dispatch."""
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") != code:
        problems.append(f"{label}: expected {code}, got ok={envelope.ok} code={error.get('code')!r}")
    if details.get("display_unavailable") != cause:
        problems.append(f"{label}: display_unavailable is {details.get('display_unavailable')!r}, not {cause!r}")
    if envelope.exit_code != 1:
        problems.append(f"{label}: a structured error must exit 1, got {envelope.exit_code}")
    if (error.get("disposition") or {}).get("delivery") != "not_delivered":
        problems.append(f"{label}: a refused request must be reported not_delivered")
    if physical and details.get("physical_delivery_started") is not False:
        problems.append(f"{label}: an input refusal must report physical_delivery_started false")
    return problems


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
