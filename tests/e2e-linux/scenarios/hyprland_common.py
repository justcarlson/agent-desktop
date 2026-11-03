"""Shared reads and verdicts for the legs on the attached `hyprland` tier."""

from __future__ import annotations

import threading
from pathlib import Path

from oracle import hyprland

X_LOCKS = Path("/tmp")


def record(tier) -> dict:
    from runner import attached
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


def instance_env(tier, instance: dict) -> dict[str, str]:
    """The product's environment bound to `instance`."""
    return dict(tier.env, WAYLAND_DISPLAY=instance["wayland"], HYPRLAND_INSTANCE_SIGNATURE=instance["signature"])


def instance_query(tier, instance: dict, name: str):
    return hyprland.query(shared(tier), instance["signature"], name, pid=instance["pid"])


def instance_control(tier, instance: dict, argv: list[str]) -> None:
    output = hyprland.control(shared(tier), instance["signature"], argv)
    if output != "ok":
        raise RuntimeError(f"the compositor refused a test control {argv}: {output}")


def _parent_of(pid: int, root: Path = Path("/proc")) -> int | None:
    try:
        text = (root / str(pid) / "stat").read_text()
    except OSError:
        return None
    return int(text[text.rindex(")") + 2:].split()[1])


def x_display(instance: dict, locks: Path = X_LOCKS, root: Path = Path("/proc")) -> str:
    """The X display the instance's XWayland serves: the `.X<n>-lock` file that names the compositor
    or a child of it, or "" when none does."""
    for lock in sorted(locks.glob(".X*-lock")):
        number = lock.name[2:-len("-lock")]
        try:
            owner = int(lock.read_text().strip())
        except (OSError, ValueError):
            continue
        if number.isdigit() and instance["pid"] in (owner, _parent_of(owner, root)):
            return f":{number}"
    return ""
