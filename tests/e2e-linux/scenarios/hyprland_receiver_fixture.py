"""The receiver fixtures of probes/linux/FINDINGS.md L47 and L45 on the nested instance's XWayland
display, for the Hyprland ref pointer legs.

The fixture is `hyprland_hit_probe.py fixture <toolkit> <case>`: a window whose "Probe Target" button
sits under an in-window "Probe Shield" overlay (`overlay`) or under a popup of the same client
(`popup`). It writes one JSON line per event to its log; a `clicked` line is a button the app saw
clicked, the oracle for whether a pointer action reached the app.
"""

from __future__ import annotations

import contextlib
import json
from pathlib import Path

from oracle import atspi_raw as raw
from oracle import proc
from runner import attached_session
from runner.apps import OwnedApp
from runner.helper import require
from scenarios.hyprland_close import _register, _release
from scenarios.hyprland_pointer_pad import XWAYLAND, backend_problem, instance_query, pad_env

HIT_PROBE = Path(__file__).resolve().parents[1] / "hyprland_hit_probe.py"
FIXTURE_SETTLE_S = 15.0
POPUP_SETTLE_S = 5.0
TARGET = ("button", "Probe Target")
TITLE = "AD Hit Probe"


def fixture_events(log: Path) -> list[dict]:
    """The fixture's JSON lines; its toolkit's stderr shares the log and is left out."""
    found = []
    for line in log.read_text().splitlines():
        if line.startswith("{"):
            with contextlib.suppress(json.JSONDecodeError):
                found.append(json.loads(line))
    return found


def clicked(found: list[dict]) -> list[str]:
    return [item.get("widget") for item in found if item.get("ev") == "clicked"]


def fixture_client(tier, instance: dict, pid: int) -> dict | None:
    """The fixture's main toplevel. A popup of the same client may be listed beside it."""
    found = [item for item in instance_query(tier, instance, "clients")
             if item.get("pid") == pid and item.get("title") == TITLE]
    return found[0] if len(found) == 1 and found[0].get("mapped") else None


def _mapped_pid(log: Path) -> int | None:
    pids = [item.get("pid") for item in fixture_events(log) if item.get("ev") == "mapped"]
    return pids[0] if pids else None


@contextlib.contextmanager
def receiver_fixture(tier, instance: dict, toolkit: str, case: str):
    """The fixture on the instance's XWayland display. Yields the app, its toplevel and its log once
    `j/clients` shows the toplevel with `xwayland: true` and, for `popup`, the popup is shown."""
    env = dict(pad_env(tier, instance, XWAYLAND), QT_QPA_PLATFORM="xcb", QT_ACCESSIBILITY="1",
               QT_LINUX_ACCESSIBILITY_ALWAYS_ON="1")
    label = f"receiver-{toolkit}-{case}"
    log = tier.runtime_dir / f"{label}.log"
    log.write_text("")
    attached_session.spawn(tier, ["python3", "-I", str(HIT_PROBE), "fixture", toolkit, case], env, log)
    pid = raw.poll(lambda: _mapped_pid(log), FIXTURE_SETTLE_S)
    require(pid, f"the {label} fixture never mapped: {log.read_text()[-400:]}")
    start = proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]
    owned = _register(tier, OwnedApp(label, pid, start, [], [], ""))
    try:
        client = raw.poll(lambda: fixture_client(tier, instance, pid), 5.0)
        require(client, f"the {label} fixture has no mapped toplevel on {instance['signature']}")
        problem = backend_problem(client, XWAYLAND)
        require(problem is None, problem or "")
        if case == "popup":
            shown = raw.poll(lambda: any(item.get("ev") == "popup-shown" for item in fixture_events(log)),
                             POPUP_SETTLE_S)
            require(shown, f"the {label} popup never showed")
        yield owned, client, log
    finally:
        _release(tier, owned)
