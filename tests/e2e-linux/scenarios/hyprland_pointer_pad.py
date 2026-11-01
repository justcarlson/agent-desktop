"""The pointer pad on the attached `hyprland` tier: a GTK3 window the run owns that logs each
pointer event it receives, started on the session instance or on the nested second instance.

The nested instance has no shell layers and no part of the user's desktop, so every pointer
event the legs deliver lands there, inside the pad. The pad's own log is the oracle for what
arrived; the compositor's `j/clients` and `j/cursorpos` say where.
"""

from __future__ import annotations

import contextlib
import json
import time
from pathlib import Path

from oracle import atspi_raw as raw
from oracle import hyprland
from runner import apps, attached_session
from runner.helper import nodes, require
from scenarios.hyprland_close import _register, _release
from scenarios.hyprland_common import shared

PAD_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "pointer_pad.py"
PAD_CLASS = "ad-pointer-pad"
PAD_NAMES = ("ad pointer pad", PAD_CLASS)
PAD_SETTLE_S = 10.0
EVENT_SETTLE_S = 3.0
CLICK_GAP_S = 0.8
COVER_CLASS = "ad-pointer-cover"


def instance_env(tier, instance: dict) -> dict[str, str]:
    """The product's environment bound to `instance`."""
    return dict(tier.env, WAYLAND_DISPLAY=instance["wayland"], HYPRLAND_INSTANCE_SIGNATURE=instance["signature"])


def instance_query(tier, instance: dict, name: str):
    return hyprland.query(shared(tier), instance["signature"], name, pid=instance["pid"])


def instance_control(tier, instance: dict, argv: list[str]) -> None:
    output = hyprland.control(shared(tier), instance["signature"], argv)
    if output != "ok":
        raise RuntimeError(f"the compositor refused a test control {argv}: {output}")


def pad_client(tier, instance: dict, pid: int) -> dict | None:
    found = [item for item in instance_query(tier, instance, "clients") if item.get("pid") == pid]
    return found[0] if len(found) == 1 and found[0].get("mapped") else None


def cursor_at(tier, instance: dict) -> tuple[float, float]:
    point = instance_query(tier, instance, "cursorpos")
    return float(point["x"]), float(point["y"])


def events(log: Path) -> list[dict]:
    return [json.loads(line) for line in log.read_text().splitlines() if line.strip()]


def presses(found: list[dict], widget: str) -> list[int]:
    """The GTK press counts one widget logged, in order."""
    return [item["n"] for item in found if item.get("ev") == "press" and item.get("widget") == widget]


def scrolls(found: list[dict], widget: str) -> list[str]:
    """The wheel directions one widget logged, in order; a smooth event with no direction is left out."""
    return [item["direction"] for item in found if item.get("ev") == "scroll" and item.get("widget") == widget
            and item.get("direction") != "smooth"]


def since(found: list[dict], count: int) -> list[dict]:
    """The events logged after the first `count`."""
    return found[count:]


def releases(found: list[dict], widget: str) -> list[dict]:
    return [item for item in found if item.get("ev") == "release" and item.get("widget") == widget]


def motions_after(found: list[dict], moment: float) -> list[dict]:
    return [item for item in found if item.get("ev") == "motion" and item["t"] > moment]


def apart_from_last_click(log: Path, gap: float = CLICK_GAP_S) -> None:
    """Wait until `gap` seconds have passed since the pad's last event, so the
    next press does not join the last click sequence."""
    found = events(log)
    if found:
        left = found[-1]["t"] + gap - time.monotonic()
        if left > 0:
            time.sleep(left)


def holds(point: tuple[float, float], box: dict) -> bool:
    """Whether a client or layer box (`at`/`size`, or `x`/`y`/`w`/`h`) holds a global point."""
    if "at" in box:
        (x, y), (width, height) = box["at"], box["size"]
    else:
        x, y, width, height = box["x"], box["y"], box["w"], box["h"]
    return x <= point[0] < x + width and y <= point[1] < y + height


def covering_layers(layers: dict, point: tuple[float, float]) -> list[str]:
    """The namespaces of the top and overlay layers (level 2 and 3) that hold a global point."""
    found = []
    for monitor in layers.values():
        for level, entries in (monitor.get("levels") or {}).items():
            if int(level) >= 2:
                found += [entry.get("namespace", "") for entry in entries if holds(point, entry)]
    return found


def window_point(point: tuple[float, float], client: dict) -> tuple[float, float]:
    """A global layout point in the client's window-local logical pixels."""
    return point[0] - client["at"][0], point[1] - client["at"][1]


def lands_at(event: dict, expected: tuple[float, float], tolerance: float = 1.0) -> bool:
    return abs(event["wx"] - expected[0]) <= tolerance and abs(event["wy"] - expected[1]) <= tolerance


def global_point(client: dict, fraction: tuple[float, float]) -> tuple[int, int]:
    """A point inside the client, at `fraction` of its width and height."""
    (x, y), (width, height) = client["at"], client["size"]
    return int(x + width * fraction[0]), int(y + height * fraction[1])


def xy(point: tuple[float, float]) -> str:
    return f"{int(point[0])},{int(point[1])}"


def settled(log: Path, check, seconds: float = EVENT_SETTLE_S):
    """The pad's log once `check` accepts it, or its last state when the time runs out."""
    deadline = time.monotonic() + seconds
    while True:
        found = events(log)
        if check(found) or time.monotonic() >= deadline:
            return found
        time.sleep(0.05)


def find_ref(ctx, env: dict[str, str], window: str, role: str, name: str) -> str:
    """The one qualified ref for `role`/`name` in the pad window, observed through `env`."""
    envelope = ctx.product(["find", "--exact", "--limit", "0", "--role", role, "--name", name],
                           env=env, window_id=window, headed=False)
    require(envelope.ok, f"find {role}/{name}: {envelope.raw[:400]}")
    found = {item["ref_id"] for item in nodes(envelope.data) if isinstance(item.get("ref_id"), str)}
    require(len(found) == 1, f"expected one ref for {role}/{name}, found {found}")
    return found.pop()


def pad_ref(ctx, env: dict[str, str], owned) -> str:
    """The pad button's qualified ref, observed through `env`."""
    envelope = ctx.product(["list-windows"], env=env, headed=False)
    require(envelope.ok and isinstance(envelope.data, list), f"list-windows failed: {envelope.raw[:400]}")
    windows = [item.get("id") for item in envelope.data if item.get("pid") == owned.pid]
    require(len(windows) == 1, f"the pad is listed {len(windows)} times")
    window = windows[0]
    return find_ref(ctx, env, window, "button", "Pad Button")


def pads_left(tier) -> list[int]:
    """The pids of pointer pads still running, once any closing pad is gone; empty between legs."""
    raw.poll(lambda: not apps._owners(tier, names=PAD_NAMES), PAD_SETTLE_S)
    return sorted(app.pid for app in apps._owners(tier, names=PAD_NAMES))


def _pad_owner(tier):
    found = [app for app in apps._owners(tier, names=PAD_NAMES) if app.windows]
    return found[0] if len(found) == 1 else None


@contextlib.contextmanager
def pointer_pad(tier, instance: dict, label: str):
    """The pad on `instance`. Yields the app, its toplevel and its event log."""
    require(not apps._owners(tier, names=PAD_NAMES), "a pointer pad from an earlier leg still runs")
    env = dict(tier.env, WAYLAND_DISPLAY=instance["wayland"], GDK_BACKEND="wayland")
    env.pop("AT_SPI_BUS_ADDRESS", None)
    env.pop("HYPRLAND_INSTANCE_SIGNATURE", None)
    log = tier.runtime_dir / f"pointer-pad-{label}.events"
    log.write_text("")
    attached_session.spawn(tier, ["python3", str(PAD_SCRIPT), str(log)], env,
                           tier.runtime_dir / f"pointer-pad-{label}.log")
    with raw.bounded_until(time.monotonic() + PAD_SETTLE_S):
        owner = raw.poll(lambda: _pad_owner(tier), PAD_SETTLE_S)
    require(owner, "the pointer pad never registered its window")
    owned = _register(tier, owner)
    try:
        client = raw.poll(lambda: pad_client(tier, instance, owned.pid), 5.0)
        require(client, f"the pointer pad has no mapped toplevel on {instance['signature']}")
        yield owned, client, log
    finally:
        _release(tier, owned)
