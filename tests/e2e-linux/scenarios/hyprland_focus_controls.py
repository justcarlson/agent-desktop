"""Compositor reads, test controls and test windows for the Hyprland focus legs.

Every read goes to the session compositor over its own IPC socket, as
`j/clients`, `j/activewindow`, `j/monitors`, `j/workspaces` and
`j/cursorpos`. Every control is one `hyprctl` call against that instance, in
the Lua form on a Lua config and the classic form otherwise. A window a leg
starts runs in the run's slice and is stopped by its recorded generation.
"""

from __future__ import annotations

import contextlib
import time
from pathlib import Path

from oracle import atspi_raw as raw, hyprland, proc
from runner import apps, attached_session
from runner.apps import OwnedApp
from runner.helper import require
from scenarios.hyprland_common import main_instance, shared

SPECIAL = "special:"
SPARE_WORKSPACES = tuple(str(number) for number in range(31, 40))
TWINS_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "focus_twins.py"
TWINS_NAMES = ("focus twins", "focus_twins.py")
SIZE_TOLERANCE = 1.0
TWINS_SETTLE_S = 10.0


def query(tier, name: str):
    main = main_instance(tier)
    return hyprland.query(shared(tier), main["signature"], name, pid=main["pid"])


def clients(tier) -> list[dict]:
    return query(tier, "clients")


def clients_of(tier, pid: int) -> list[dict]:
    return [item for item in clients(tier) if item.get("pid") == pid]


def client(tier, address: str) -> dict | None:
    found = [item for item in clients(tier) if item.get("address") == address]
    return found[0] if len(found) == 1 else None


def active(tier) -> dict:
    return query(tier, "activewindow")


def cursor(tier) -> dict:
    return query(tier, "cursorpos")


def shown(tier) -> dict[str, tuple[str, str]]:
    """Each monitor's regular workspace and the special workspace open over it, or an empty name."""
    return {item["name"]: (item["activeWorkspace"]["name"], (item.get("specialWorkspace") or {}).get("name", ""))
            for item in query(tier, "monitors")}


def spare_workspaces(names_in_use: set[str], count: int) -> list[str]:
    """Regular workspaces no window and no monitor uses, so a leg owns them alone."""
    free = [name for name in SPARE_WORKSPACES if name not in names_in_use]
    require(len(free) >= count, f"no {count} spare workspaces among {SPARE_WORKSPACES}")
    return free[:count]


def workspaces_in_use(tier) -> set[str]:
    return {item.get("name") for item in query(tier, "workspaces")}


def lua_call(name: str, fields: dict[str, object]) -> str:
    parts = []
    for key, value in fields.items():
        if value is False:
            text = "false"
        elif isinstance(value, int):
            text = str(value)
        else:
            text = hyprland.lua_string(str(value))
        parts.append(f"{key}={text}")
    return f"{name}({{{','.join(parts)}}})"


def focus_argv(address: str, lua: bool) -> list[str]:
    target = f"address:{address}"
    if lua:
        return ["dispatch", lua_call("hl.dsp.focus", {"window": target})]
    hyprland.lua_string(target)
    return ["dispatch", "focuswindow", target]


def move_argv(address: str, workspace: str, lua: bool) -> list[str]:
    target = f"address:{address}"
    if lua:
        return ["dispatch", lua_call("hl.dsp.window.move", {"workspace": workspace, "follow": False,
                                                            "window": target})]
    hyprland.lua_string(f"{workspace},{target}")
    return ["dispatch", "movetoworkspacesilent", f"{workspace},{target}"]


def toggle_argv(name: str, lua: bool) -> list[str]:
    bare = name.removeprefix(SPECIAL)
    if lua:
        return ["dispatch", f"hl.dsp.workspace.toggle_special({hyprland.lua_string(bare)})"]
    hyprland.lua_string(bare)
    return ["dispatch", "togglespecialworkspace", bare]


def cursor_argv(x: float, y: float, lua: bool) -> list[str]:
    if lua:
        return ["dispatch", lua_call("hl.dsp.cursor.move", {"x": int(x), "y": int(y)})]
    return ["dispatch", "movecursor", str(int(x)), str(int(y))]


def control(tier, build) -> None:
    main = main_instance(tier)
    runtime, signature = shared(tier), main["signature"]
    output = hyprland.control(runtime, signature, build(hyprland.uses_lua(runtime, signature)))
    if output != "ok":
        raise RuntimeError(f"the compositor refused a test control: {output}")


def move(tier, address: str, workspace: str) -> None:
    control(tier, lambda lua: move_argv(address, workspace, lua))
    require(raw.poll(lambda: ((client(tier, address) or {}).get("workspace") or {}).get("name") == workspace, 3.0),
            f"{address} did not move to {workspace}")


def restore_desktop(tier, holder: dict, point: dict) -> None:
    """Close every special workspace, give the bystander focus back and put the cursor back."""
    for name in {special for _, special in shown(tier).values() if special}:
        control(tier, lambda lua, name=name: toggle_argv(name, lua))
    require(raw.poll(lambda: not any(special for _, special in shown(tier).values()), 3.0),
            f"a special workspace stayed open: {shown(tier)}")
    control(tier, lambda lua: focus_argv(holder["address"], lua))
    require(raw.poll(lambda: active(tier).get("address") == holder["address"], 3.0),
            "the bystander did not take focus back")
    control(tier, lambda lua: cursor_argv(point["x"], point["y"], lua))
    require(raw.poll(lambda: cursor(tier) == point, 3.0), f"the cursor did not return to {point}")


def _register(tier, owned: OwnedApp) -> OwnedApp:
    require(proc.owned(tier, owned.pid, owned.start_time, {}), f"{owned.name} is not in the run's slice")
    apps._TIERS[owned.pid, owned.start_time] = tier
    return owned


def _stop(tier, owned: OwnedApp) -> None:
    apps.stop(owned)
    apps._TIERS.pop((owned.pid, owned.start_time), None)
    require(raw.poll(lambda: not clients_of(tier, owned.pid), 5.0), f"{owned.name} still has a toplevel")


def _new_client(tier, app_id: str, known: set[str]) -> dict | None:
    found = [item for item in clients(tier) if item.get("class") == app_id and item.get("address") not in known]
    require(len(found) <= 1, f"several new {app_id} clients appeared: {found}")
    return found[0] if found and found[0].get("mapped") else None


def terminal_argv(app_id: str) -> list[str]:
    """A foot toplevel whose command never writes a title, so its title holds still."""
    return ["foot", "--app-id", app_id, "--title", app_id, "sleep", "infinity"]


@contextlib.contextmanager
def terminal(tier, app_id: str):
    """A foot toplevel with no accessibility tree. Yields it and its compositor client."""
    hyprland.lua_string(app_id)
    known = {item.get("address") for item in clients(tier)}
    attached_session.spawn(tier, terminal_argv(app_id), dict(tier.env), tier.runtime_dir / "focus-terminal.log")
    found = raw.poll(lambda: _new_client(tier, app_id, known), 10.0)
    require(found, f"the {app_id} terminal never mapped")
    start = proc.stat_fields((Path("/proc") / str(found["pid"]) / "stat").read_text())[0]
    owned = _register(tier, OwnedApp(app_id, found["pid"], start, [], [], ""))
    try:
        yield owned, found
    finally:
        _stop(tier, owned)


def _twins_owner(tier) -> OwnedApp | None:
    found = [app for app in apps._owners(tier, names=TWINS_NAMES) if len(app.windows) == 2]
    return found[0] if len(found) == 1 else None


def _mapped_pair(tier, pid: int) -> list[dict] | None:
    own = clients_of(tier, pid)
    return own if len(own) == 2 and all(item.get("mapped") for item in own) else None


@contextlib.contextmanager
def focus_twins(tier):
    """Two same-titled native Wayland frames of one process. Yields the app and its two clients."""
    require(not apps._owners(tier, names=TWINS_NAMES), "focus twins from an earlier leg still run")
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    env["GTK_USE_PORTAL"] = "0"
    attached_session.spawn(tier, ["python3", str(TWINS_SCRIPT)], env, tier.runtime_dir / "focus-twins.log",
                           toplevels=("focus_twins.py", 2))
    with raw.bounded_until(time.monotonic() + TWINS_SETTLE_S):
        owner = raw.poll(lambda: _twins_owner(tier), TWINS_SETTLE_S)
    require(owner, "the focus twins never registered two frames")
    owned = _register(tier, owner)
    try:
        found = raw.poll(lambda: _mapped_pair(tier, owned.pid), 5.0)
        require(found, f"the focus twins have toplevels {clients_of(tier, owned.pid)}")
        xwayland = [item.get("address") for item in found if item.get("xwayland") is not False]
        require(not xwayland, f"focus twins run under Xwayland: {xwayland}")
        yield owned, found
    finally:
        _stop(tier, owned)


def frame_sizes(tier, app: OwnedApp) -> dict[str, tuple[int, int]]:
    """The AT-SPI size of each of the app's frames, by window id."""
    frames = raw.app_root(tier, app.owner_names[0]).children()
    return {apps.window_id(frame): tuple(frame.extents()[2:]) for frame in frames
            if apps.window_id(frame) in app.windows}


def within(first, second) -> bool:
    return all(abs(float(a) - float(b)) <= SIZE_TOLERANCE for a, b in zip(first, second))


def address_by_size(size, toplevels: list[dict]) -> str | None:
    """The one toplevel whose size is the frame's, or None when none or several are."""
    found = [item["address"] for item in toplevels if within(size, item.get("size") or ())]
    return found[0] if len(found) == 1 else None


def same_size(toplevels: list[dict], sizes: dict[str, tuple]) -> bool:
    """Both toplevels have one size, and both frames report it."""
    distinct = {tuple(item.get("size") or ()) for item in toplevels}
    if len(toplevels) != 2 or len(distinct) != 1 or len(sizes) != 2:
        return False
    size = next(iter(distinct))
    return all(within(frame, size) for frame in sizes.values())


def bound_by_size(toplevels: list[dict], sizes: dict[str, tuple]) -> dict[str, str] | None:
    """Each frame matched to its own toplevel by size, or None while any frame matches none or both."""
    if len(toplevels) != 2 or len(sizes) != 2:
        return None
    matched = {window: address_by_size(size, toplevels) for window, size in sizes.items()}
    if None in matched.values() or len(set(matched.values())) != 2:
        return None
    return matched


def listed_ids(envelope, pid: int) -> list[str]:
    """The ids list-windows gives a process; the leg uses them as input, never as evidence."""
    require(envelope.ok and isinstance(envelope.data, list), f"list-windows failed: {envelope.raw[:400]}")
    return [item.get("id") for item in envelope.data if item.get("pid") == pid]
