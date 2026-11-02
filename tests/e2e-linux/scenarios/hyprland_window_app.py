"""The window sizer on any Hyprland instance the run owns, and reads of its log.

The sizer (`apps/hyprland_window_sizer.py`) logs each size and toplevel state the compositor
gives it; the size is the oracle (L46-1). That log is the app's own view of a window operation, read beside `j/clients`.
"""

from __future__ import annotations

import contextlib
import json
import time
from pathlib import Path

from oracle import atspi_raw as raw, proc
from runner import attached_session
from runner.apps import OwnedApp
from runner.helper import require
from scenarios.hyprland_close import _register, _release
from scenarios.hyprland_pointer_pad import NATIVE, instance_query, pad_env
from scenarios.hyprland_window_checks import TOLERANCE, placement

SIZER_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "hyprland_window_sizer.py"
SIZER_CLASS = "ad-window-sizer"
SIZER_SETTLE_S = 10.0
LOG_SETTLE_S = 3.0


def events(log: Path) -> list[dict]:
    return [json.loads(line) for line in log.read_text().splitlines() if line.strip()]


def logged(log: Path, kind: str) -> dict | None:
    """The last line of `kind` in the sizer's log."""
    found = [item for item in events(log) if item.get("ev") == kind]
    return found[-1] if found else None


def size_matches(item: dict | None, size: list[int]) -> bool:
    return bool(item) and len(size) == 2 and all(
        abs(item[key] - want) <= TOLERANCE for key, want in zip(("w", "h"), size))


def size_problems(label: str, log: Path, size: list[int]) -> list[str]:
    """The sizer's last configured size is `size`, the size `j/clients` reports."""
    raw.poll(lambda: size_matches(logged(log, "configure"), size), LOG_SETTLE_S)
    seen = logged(log, "configure")
    return [] if size_matches(seen, size) else [f"{label}: the app's log shows size {seen}, j/clients {size}"]


def resized_problems(label: str, log: Path, before: list[int], size: list[int]) -> list[str]:
    """The sizer was given `size`, the size `j/clients` reports, and it is not `before`. The app's
    toplevel state is no oracle here: Hyprland sends it maximized when it tiles the window at map
    time and never changes it on a maximize or a restore (L46-1)."""
    if size_matches({"w": size[0], "h": size[1]}, before):
        return [f"{label}: j/clients keeps the size {size} it had before, {before}"]
    return size_problems(label, log, size)


def settled_count(log: Path, quiet_s: float = 0.5) -> int:
    """The number of lines in the sizer's log once no line arrived for `quiet_s`."""
    last = [len(events(log)), time.monotonic()]

    def still() -> bool:
        now = len(events(log))
        if now != last[0]:
            last[:] = [now, time.monotonic()]
            return False
        return time.monotonic() - last[1] >= quiet_s

    raw.poll(still, LOG_SETTLE_S)
    return last[0]


def quiet_problems(label: str, log: Path, count: int) -> list[str]:
    """No size or state reached the sizer since its log held `count` lines."""
    late = events(log)[count:]
    return [] if not late else [f"{label}: the app saw {late} although nothing was sent"]


def client_of(tier, instance: dict, address: str) -> dict | None:
    found = [item for item in instance_query(tier, instance, "clients") if item.get("address") == address]
    return found[0] if len(found) == 1 else None


def reader(tier, instance: dict, address: str):
    """Reads the toplevel's placement on `instance`."""
    return lambda: placement(client_of(tier, instance, address))


def _new_client(tier, instance: dict, known: set[str]) -> dict | None:
    found = [item for item in instance_query(tier, instance, "clients")
             if item.get("class") == SIZER_CLASS and item.get("address") not in known]
    require(len(found) <= 1, f"several new sizer clients appeared: {found}")
    return found[0] if found and found[0].get("mapped") else None


@contextlib.contextmanager
def sizer(tier, instance: dict, label: str):
    """The sizer on `instance`, native Wayland, in the run's slice. Yields the app, its toplevel
    and its log once `j/clients` shows the toplevel mapped."""
    known = {item.get("address") for item in instance_query(tier, instance, "clients")}
    log = tier.runtime_dir / f"window-sizer-{label}.events"
    log.write_text("")
    attached_session.spawn(tier, ["python3", str(SIZER_SCRIPT), str(log)], pad_env(tier, instance, NATIVE),
                           tier.runtime_dir / f"window-sizer-{label}.log")
    found = raw.poll(lambda: _new_client(tier, instance, known), SIZER_SETTLE_S)
    require(found, f"the window sizer never mapped on {instance['signature']}")
    start = proc.stat_fields((Path("/proc") / str(found["pid"]) / "stat").read_text())[0]
    owned = _register(tier, OwnedApp(SIZER_CLASS, found["pid"], start, [], [], ""))
    try:
        yield owned, found, log
    finally:
        _release(tier, owned)
        require(raw.poll(lambda: client_of(tier, instance, found["address"]) is None, 5.0),
                "the window sizer still has a toplevel")
