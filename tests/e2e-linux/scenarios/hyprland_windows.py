"""Compositor reads and controls for the Hyprland window legs.

Every read goes to the session compositor over its own IPC socket. Every
control is one `hyprctl` call against that one instance, in the Lua form on a
Lua config and the classic form otherwise. The terminal a leg starts runs in
the run's slice and is stopped by its exact recorded generation.
"""

from __future__ import annotations

import contextlib
from pathlib import Path

from oracle import atspi_raw as raw, hyprland, proc
from runner import apps, attached_session
from runner.apps import OwnedApp
from runner.helper import require
from scenarios.hyprland_common import main_instance, shared

SPECIAL = "special:"
SPARE_WORKSPACE = "7"
FOREIGN_INSTANCE = "w-hypr-1-1-"


def query(tier, name: str):
    main = main_instance(tier)
    return hyprland.query(shared(tier), main["signature"], name, pid=main["pid"])


def clients(tier) -> list[dict]:
    return query(tier, "clients")


def client(tier, address: str) -> dict:
    found = [item for item in clients(tier) if item.get("address") == address]
    require(len(found) == 1, f"expected one client at {address}, found {len(found)}")
    return found[0]


def clients_of(tier, pid: int) -> list[dict]:
    return [item for item in clients(tier) if item.get("pid") == pid]


def active(tier) -> dict:
    return query(tier, "activewindow")


def cursor(tier) -> dict:
    return query(tier, "cursorpos")


def specials(tier) -> dict[str, str]:
    """Each monitor's open special workspace, or an empty name."""
    return {item["name"]: (item.get("specialWorkspace") or {}).get("name", "")
            for item in query(tier, "monitors")}


def regular_workspaces(tier) -> dict[str, str]:
    return {item["name"]: item["activeWorkspace"]["name"] for item in query(tier, "monitors")}


def lua_call(name: str, fields: dict[str, object]) -> str:
    parts = []
    for key, value in fields.items():
        text = "false" if value is False else (str(value) if isinstance(value, int)
                                              else hyprland.lua_string(str(value)))
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
        return ["dispatch", lua_call("hl.dsp.window.move",
                                     {"workspace": workspace, "follow": False, "window": target})]
    hyprland.lua_string(f"{workspace},{target}")
    return ["dispatch", "movetoworkspacesilent", f"{workspace},{target}"]


def toggle_argv(name: str, lua: bool) -> list[str]:
    bare = name.removeprefix(SPECIAL)
    if lua:
        return ["dispatch", f"hl.dsp.workspace.toggle_special({hyprland.lua_string(bare)})"]
    hyprland.lua_string(bare)
    return ["dispatch", "togglespecialworkspace", bare]


def cursor_argv(x: int, y: int, lua: bool) -> list[str]:
    if lua:
        return ["dispatch", lua_call("hl.dsp.cursor.move", {"x": int(x), "y": int(y)})]
    return ["dispatch", "movecursor", str(int(x)), str(int(y))]


def control(tier, build) -> None:
    main = main_instance(tier)
    runtime, signature = shared(tier), main["signature"]
    output = hyprland.control(runtime, signature, build(hyprland.uses_lua(runtime, signature)))
    if output != "ok":
        raise RuntimeError(f"the compositor refused a test control: {output}")


def focus(tier, address: str) -> None:
    control(tier, lambda lua: focus_argv(address, lua))


def move(tier, address: str, workspace: str) -> None:
    control(tier, lambda lua: move_argv(address, workspace, lua))
    require(raw.poll(lambda: client(tier, address)["workspace"]["name"] == workspace, 3.0),
            f"{address} did not move to {workspace}")


def toggle_special(tier, name: str) -> None:
    control(tier, lambda lua: toggle_argv(name, lua))


def close_specials(tier) -> None:
    for name in {special for special in specials(tier).values() if special}:
        toggle_special(tier, name)
    require(raw.poll(lambda: not any(specials(tier).values()), 3.0),
            f"a special workspace stayed open: {specials(tier)}")


def restore_desktop(tier, holder: dict, point: dict) -> None:
    """Close every special workspace, give the bystander focus back and put the cursor back."""
    close_specials(tier)
    focus(tier, holder["address"])
    require(raw.poll(lambda: active(tier).get("address") == holder["address"], 3.0),
            "the bystander did not take focus back")
    control(tier, lambda lua: cursor_argv(point["x"], point["y"], lua))
    require(raw.poll(lambda: cursor(tier) == point, 3.0), f"the cursor did not return to {point}")


def _new_client(tier, app_id: str, known: set[str]) -> dict | None:
    found = [item for item in clients(tier)
             if item.get("class") == app_id and item.get("address") not in known]
    require(len(found) <= 1, f"several new {app_id} clients appeared: {found}")
    return found[0] if found and found[0].get("mapped") else None


def terminal_argv(app_id: str) -> list[str]:
    """A foot toplevel whose command never writes a title, so its title holds still while a leg reads it."""
    return ["foot", "--app-id", app_id, "--title", app_id, "sleep", "infinity"]


@contextlib.contextmanager
def terminal(tier, app_id: str):
    """A foot toplevel with no accessibility tree, started in the run's slice."""
    hyprland.lua_string(app_id)
    known = {item.get("address") for item in clients(tier)}
    attached_session.spawn(tier, terminal_argv(app_id), dict(tier.env),
                           tier.runtime_dir / "terminal-launch.log")
    found = raw.poll(lambda: _new_client(tier, app_id, known), 10.0)
    require(found, f"the {app_id} terminal never mapped")
    pid = found["pid"]
    start = proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]
    require(proc.owned(tier, pid, start, {}), f"the {app_id} terminal is not in the run's slice")
    owned = OwnedApp(app_id, pid, start, [], [], "")
    apps._TIERS[pid, start] = tier
    try:
        yield owned, found
    finally:
        apps.stop(owned)
        apps._TIERS.pop((pid, start), None)


def listed(envelope, *, pid: int | None = None, app_name: str | None = None) -> list[dict]:
    require(envelope.ok and isinstance(envelope.data, list), f"list-windows failed: {envelope.raw[:400]}")
    return [item for item in envelope.data
            if (pid is None or item.get("pid") == pid)
            and (app_name is None or item.get("app_name") == app_name)]


def compositor_prefix(tier) -> str:
    start = proc.stat_fields((Path("/proc") / str(main_instance(tier)["pid"]) / "stat").read_text())[0]
    return f"w-hypr-{main_instance(tier)['pid']}-{start}-"


def foreign_id(window_id: str) -> str:
    """The same toplevel identifier under a compositor instance that never ran."""
    return FOREIGN_INSTANCE + window_id.rsplit("-", 1)[-1]


def frame_sizes(tier, app) -> list[tuple[int, int]]:
    """The AT-SPI size of each of the app's frames."""
    frames = [child for child in raw.app_root(tier, app.owner_names[0]).children()
              if apps.window_id(child) in app.windows]
    return sorted(tuple(frame.extents()[2:]) for frame in frames)

