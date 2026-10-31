"""A second, runner-owned headless Hyprland for the multi-instance binding legs.

The instance runs as a client of the session's compositor, alone on its own
silent special workspace (`OUTER_WORKSPACE`), inside the run's slice. Outer
tiling never shares that workspace with the apps a leg opens, so their windows
cannot resize the instance's window and move its cursor. It has its own IPC socket, its own
Wayland socket, a headless output named `OUTPUT` and a sentinel window, so a
leg can tell the two instances apart and observe both. The attached record
names it, so teardown and a cancelled run stop it and remove its paths.

A leg that reads the instance's outputs starts it with `ensure_sized`. The
instance commits a mode only when GBM on its render node can allocate a buffer,
which a VM without virgl cannot. `OUTPUT` gets the first mode it advertises,
every other output is disabled, and the leg goes on only once
`hyprctl -j monitors all` shows `OUTPUT` alone with a size. Otherwise it raises `OutputsNotReady` with the instance's log, its window in the
outer session and its launch environment.
"""

from __future__ import annotations

import json
import os
import re
import signal
import sys
import time
from pathlib import Path

from oracle import hyprland, proc as process
from runner import attached, attached_session, session

OUTPUT = "AGENT-DESKTOP-1"
SENTINEL = "ad-e2e-sentinel"
OUTER_WORKSPACE = "ad_e2e_nested"
MODE = "1280x800@60"
WAYLAND_SOCKET = re.compile(r"wayland-\d+")
LOG_LIMIT = 262_144
DEBUG_ENV = {"WAYLAND_DEBUG": "client"}
SIZED_LOGS = True
LAUNCH_KEYS = ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "WAYLAND_DEBUG", "XDG_RUNTIME_DIR")
DROPPED = ("HYPRLAND_INSTANCE_SIGNATURE", "AT_SPI_BUS_ADDRESS", "AGENT_DESKTOP_HOME")
LUA_DEBUG = " debug = { disable_logs = false, enable_stdout_logs = true },"
HYPRLANG_DEBUG = "debug {\n    disable_logs = false\n    enable_stdout_logs = true\n}\n"


def config_text(lua: bool, logs: bool = False) -> str:
    """The second instance's config; `logs` turns on its file and stdout logs."""
    if lua:
        return (
            f'hl.monitor({{ output = "", mode = "{MODE}", position = "0x0", scale = 1 }})\n'
            "hl.config({ misc = { disable_hyprland_logo = true, disable_splash_rendering = true },"
            f" animations = {{ enabled = false }},{LUA_DEBUG if logs else ''}"
            " ecosystem = { no_update_news = true, no_donation_nag = true } })\n"
        )
    return (
        f"monitor = , {MODE}, 0x0, 1\n"
        "misc {\n    disable_hyprland_logo = true\n    disable_splash_rendering = true\n}\n"
        "animations {\n    enabled = false\n}\n"
        "ecosystem {\n    no_update_news = true\n    no_donation_nag = true\n}\n"
        + (HYPRLANG_DEBUG if logs else "")
    )


class OutputsNotReady(RuntimeError):
    """The second instance's outputs never reported a size: a fixture error, not a product result."""

    def __init__(self, message: str):
        super().__init__(message)
        self.diagnostics: dict = {}


def output_command(lua: bool) -> list[str]:
    if lua:
        return ["eval", f'hl.monitor({{ output = "{OUTPUT}", mode = "{MODE}", position = "1280x0", scale = 1 }})']
    return ["keyword", "monitor", f"{OUTPUT},{MODE},1280x0,1"]


def advertised_mode(monitor: dict) -> str | None:
    """The first mode an output advertises, in monitor-rule form (`1920x1080@60.00`)."""
    modes = monitor.get("availableModes") or []
    return modes[0].removesuffix("Hz") if modes else None


def mode_command(name: str, mode: str | None, lua: bool) -> list[str]:
    """Give an output a mode it advertises at the origin, or disable one that advertises none."""
    if lua:
        spec = "disabled = true" if mode is None else f'mode = "{mode}", position = "0x0", scale = 1'
        return ["eval", f'hl.monitor({{ output = "{name}", {spec} }})']
    return ["keyword", "monitor", f"{name},disable" if mode is None else f"{name},{mode},0x0,1"]


def mode_commands(monitors: list[dict], lua: bool) -> list[list[str]]:
    """`OUTPUT` gets its first advertised mode; every other output is disabled, so only `OUTPUT` is read."""
    target = next((monitor for monitor in monitors if monitor.get("name") == OUTPUT), None)
    mode = advertised_mode(target) if target is not None else None
    if mode is None:
        raise OutputsNotReady(f"hyprland: the second instance's {OUTPUT} advertises no mode: "
                              f"{json.dumps(monitors)}")
    others = [monitor["name"] for monitor in monitors
              if monitor.get("name") != OUTPUT and not monitor.get("disabled")]
    return [mode_command(OUTPUT, mode, lua)] + [mode_command(name, None, lua) for name in others]


def readiness_problems(monitors: list[dict]) -> list[str]:
    """Why the enabled outputs are not yet the sized `OUTPUT` alone."""
    enabled = [monitor for monitor in monitors if not monitor.get("disabled")]
    names = [monitor.get("name") for monitor in enabled]
    problems = [] if names == [OUTPUT] else [f"enabled outputs are {names}, not [{OUTPUT!r}]"]
    problems += [f"{monitor.get('name')} is {monitor.get('width')}x{monitor.get('height')}"
                 for monitor in enabled if not (monitor.get("width", 0) > 0 and monitor.get("height", 0) > 0)]
    return problems


def _monitors_all(shared: Path, signature: str) -> list[dict]:
    return json.loads(hyprland.control(shared, signature, ["-j", "monitors", "all"]))


def _size_outputs(shared: Path, signature: str, lua: bool, seconds: float) -> None:
    for command in mode_commands(_monitors_all(shared, signature), lua):
        answer = hyprland.control(shared, signature, command)
        if answer != "ok":
            raise OutputsNotReady(f"hyprland: the second instance refused {command}: {answer}")
    if _poll(lambda: not readiness_problems(_monitors_all(shared, signature)), seconds):
        return
    monitors = _monitors_all(shared, signature)
    raise OutputsNotReady(f"hyprland: the second instance's outputs did not get a size within {seconds:.0f} s "
                          f"({'; '.join(readiness_problems(monitors))}): {json.dumps(monitors)}")


def _start_of(pid: int, proc_root: Path) -> int | None:
    try:
        return process.stat_fields((Path(proc_root) / str(pid) / "stat").read_text())[0]
    except (OSError, ValueError, IndexError):
        return None


def current(record: dict, proc_root: Path = Path("/proc")) -> dict | None:
    """The recorded second instance while that same process still runs."""
    for instance in record["instances"]:
        if _start_of(instance["pid"], proc_root) == instance["start"]:
            return instance
    return None


def find_new(record: dict, before: set[str], proc_root: Path = Path("/proc")) -> dict | None:
    """An instance that appeared after `before` and whose IPC peer is in the run's slice."""
    shared = Path(record["shared_runtime"])
    for signature in sorted(set(hyprland.instances(shared)) - before):
        try:
            pid = hyprland.peer_pid(hyprland.socket_path(shared, signature))
        except OSError:
            continue
        start = _start_of(pid, proc_root)
        if start is not None and attached.in_slice(pid, record["slice"], proc_root):
            return {"signature": signature, "pid": pid, "start": start, "wayland": None}
    return None


def wayland_of(shared: Path, pid: int) -> str | None:
    """The Wayland socket name in the shared runtime directory that `pid` serves."""
    for entry in sorted(Path(shared).iterdir()):
        if not WAYLAND_SOCKET.fullmatch(entry.name):
            continue
        try:
            if hyprland.peer_pid(entry, 0.5) == pid:
                return entry.name
        except OSError:
            continue
    return None


def _poll(read, seconds: float):
    deadline = time.monotonic() + seconds
    while True:
        value = read()
        if value or time.monotonic() >= deadline:
            return value
        time.sleep(0.1)


def _env(tier) -> dict[str, str]:
    return {key: value for key, value in tier.env.items() if key not in DROPPED}


def sentinel_pid(clients: list[dict]) -> int | None:
    """The sentinel's pid, matched on the title it started with: its shell may retitle it."""
    for client in clients:
        if client.get("initialTitle") == SENTINEL and client.get("pid", 0) > 1:
            return client["pid"]
    return None


def _sentinel(tier, instance: dict, seconds: float) -> list[int]:
    shared = Path(attached.load(tier.runtime_dir)["shared_runtime"])
    env = dict(_env(tier), WAYLAND_DISPLAY=instance["wayland"], GDK_BACKEND="wayland")
    attached_session.spawn(tier, ["foot", "--title", SENTINEL], env, tier.runtime_dir / "sentinel.log")

    def mapped():
        try:
            return sentinel_pid(hyprland.query(shared, instance["signature"], "clients", pid=instance["pid"]))
        except (OSError, RuntimeError, ValueError):
            return None

    pid = _poll(mapped, seconds)
    start = _start_of(pid, Path("/proc")) if pid else None
    if start is None:
        raise session.TierUnavailable("hyprland: the second instance's sentinel did not map")
    return [pid, start]


def launch_env(tier, sized: bool = False) -> dict[str, str]:
    """The second instance's launch environment; a sized start may log its Wayland protocol."""
    return {**_env(tier), **(DEBUG_ENV if sized and SIZED_LOGS else {})}


def ensure(tier, seconds: float = 20.0, sized: bool = False) -> dict:
    """The live second instance, started now if the run has none; see `launch_env` for `sized`."""
    record = attached.load(tier.runtime_dir)
    found = current(record)
    if found is not None and found.get("sentinel"):
        return found
    shared, main = Path(record["shared_runtime"]), record["signature"]
    lua = hyprland.uses_lua(shared, main)
    config = tier.runtime_dir / ("second-instance.lua" if lua else "second-instance.conf")
    config.write_text(config_text(lua, sized and SIZED_LOGS))
    before = set(hyprland.instances(shared))
    attached_session.spawn(tier, ["Hyprland", "-c", str(config)], launch_env(tier, sized),
                           tier.runtime_dir / "second-instance.log", OUTER_WORKSPACE)
    instance = _poll(lambda: find_new(attached.load(tier.runtime_dir), before), seconds)
    if instance is None:
        raise session.TierUnavailable("hyprland: the second instance did not answer on its IPC socket")
    attached.record_instance(tier.runtime_dir, instance)
    instance["wayland"] = _poll(lambda: wayland_of(shared, instance["pid"]), seconds)
    if instance["wayland"] is None:
        raise session.TierUnavailable("hyprland: the second instance serves no Wayland socket")
    attached.record_instance(tier.runtime_dir, instance)
    hyprland.control(shared, instance["signature"], ["output", "create", "headless", OUTPUT])
    hyprland.control(shared, instance["signature"], output_command(lua))
    instance["sentinel"] = _sentinel(tier, instance, seconds)
    attached.record_instance(tier.runtime_dir, instance)
    return instance


def launch_keys(environ: dict[str, str]) -> dict[str, str]:
    """The variables that choose and steer Hyprland's backends."""
    return {key: value for key, value in sorted(environ.items())
            if key in LAUNCH_KEYS or key.startswith(("AQ_", "HYPRLAND_", "GBM_"))}


def _environ_of(pid: int, proc_root: Path) -> dict[str, str] | str:
    try:
        raw = (Path(proc_root) / str(pid) / "environ").read_bytes()
    except OSError as error:
        return f"unreadable: {error}"
    pairs = (entry.decode(errors="replace").partition("=") for entry in raw.split(b"\0") if entry)
    return {key: value for key, _, value in pairs}


def tail(path: Path, limit: int = LOG_LIMIT) -> dict:
    """The end of a log, with its full size, or why it could not be read."""
    try:
        data = Path(path).read_bytes()
    except OSError as error:
        return {"path": str(path), "error": str(error)}
    return {"path": str(path), "bytes": len(data), "truncated": len(data) > limit,
            "text": data[-limit:].decode(errors="replace")}


def outer_client(clients: list[dict], pid: int) -> dict:
    """The nested instance's own window in the outer session, matched on its pid."""
    for client in clients:
        if client.get("pid") == pid:
            return client
    return {"missing": True, "client_pids": sorted({client.get("pid") for client in clients})}


WINDOW_KEYS = ("at", "size", "workspace", "floating", "pinned", "fullscreen", "monitor")


def outer_window(tier, instance: dict) -> dict:
    """Where the outer session holds the nested instance's window: geometry, workspace and floating state."""
    record = attached.load(tier.runtime_dir)
    try:
        client = outer_client(hyprland.query(Path(record["shared_runtime"]), record["signature"], "clients"),
                              instance["pid"])
    except (OSError, RuntimeError, ValueError) as error:
        return {"error": str(error)}
    return {key: client[key] for key in WINDOW_KEYS + ("missing", "client_pids") if key in client}


def print_outer_window(tier, instance: dict, stage: str) -> None:
    """Evidence only: the nested window's outer placement, to stderr, which the tier log keeps on a pass."""
    print(f"evidence: outer-window {stage} {json.dumps(outer_window(tier, instance), sort_keys=True)}",
          file=sys.stderr, flush=True)


def diagnostics(tier, instance: dict, sized: bool = False, proc_root: Path = Path("/proc")) -> dict:
    """What the second instance saw, read before teardown removes its directory."""
    record = attached.load(tier.runtime_dir)
    shared = Path(record["shared_runtime"])
    try:
        client = outer_client(hyprland.query(shared, record["signature"], "clients"), instance["pid"])
    except (OSError, RuntimeError, ValueError) as error:
        client = {"error": str(error)}
    environ = _environ_of(instance["pid"], proc_root)
    return {
        "hyprland_log": tail(hyprland.instance_dir(shared, instance["signature"]) / "hyprland.log"),
        "launch_log": tail(tier.runtime_dir / "second-instance.log"),
        "outer_client": client,
        "launch_env": {"given": launch_keys(launch_env(tier, sized)),
                       "process": environ if isinstance(environ, str) else launch_keys(environ)},
    }


def ensure_sized(tier, seconds: float = 20.0) -> dict:
    """The live second instance with `OUTPUT` alone sized, for a leg that reads its outputs."""
    instance = ensure(tier, seconds, sized=True)
    shared = Path(attached.load(tier.runtime_dir)["shared_runtime"])
    try:
        _size_outputs(shared, instance["signature"],
                      hyprland.uses_lua(shared, instance["signature"]), seconds)
    except OutputsNotReady as error:
        error.diagnostics = diagnostics(tier, instance, sized=True)
        raise
    return instance


def stop(tier, proc_root: Path = Path("/proc"), seconds: float = 5.0) -> None:
    """Stop the recorded second instance and its sentinel, then remove its paths."""
    record = attached.load(tier.runtime_dir)
    instance = current(record, proc_root)
    if instance is not None:
        targets = [tuple(instance["sentinel"])] if instance.get("sentinel") else []
        targets.append((instance["pid"], instance["start"]))
        for signum in (signal.SIGTERM, signal.SIGKILL):
            for pid, start in targets:
                session._kill(pid, signum, start, tier.runtime_dir, proc_root)
            if _poll(lambda: all(_start_of(pid, proc_root) != start for pid, start in targets), seconds):
                break
        if any(_start_of(pid, proc_root) == start for pid, start in targets):
            raise AssertionError(f"the second instance survived: {instance['signature']}")
    for entry in record["instances"]:
        if _start_of(entry["pid"], proc_root) != entry["start"]:
            for path in attached_session.instance_paths(record, entry):
                attached_session.remove_path(path)
    if instance is not None and os.path.lexists(hyprland.instance_dir(Path(record["shared_runtime"]),
                                                                      instance["signature"])):
        raise AssertionError("the second instance left its directory behind")
