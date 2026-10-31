"""A second, runner-owned headless Hyprland for the multi-instance binding legs.

The instance runs as a client of the session's compositor, on a silent
special workspace, inside the run's slice. It has its own IPC socket, its own
Wayland socket, a headless output named `OUTPUT` and a sentinel window, so a
leg can tell the two instances apart and observe both. The attached record
names it, so teardown and a cancelled run stop it and remove its paths.
"""

from __future__ import annotations

import os
import re
import signal
import time
from pathlib import Path

from oracle import hyprland, proc as process
from runner import attached, attached_session, session

OUTPUT = "AGENT-DESKTOP-1"
SENTINEL = "ad-e2e-sentinel"
MODE = "1280x800@60"
WAYLAND_SOCKET = re.compile(r"wayland-\d+")
DROPPED = ("HYPRLAND_INSTANCE_SIGNATURE", "AT_SPI_BUS_ADDRESS", "AGENT_DESKTOP_HOME")
LUA_CONFIG = (
    f'hl.monitor({{ output = "", mode = "{MODE}", position = "0x0", scale = 1 }})\n'
    "hl.config({ misc = { disable_hyprland_logo = true, disable_splash_rendering = true },"
    " animations = { enabled = false },"
    " ecosystem = { no_update_news = true, no_donation_nag = true } })\n"
)
HYPRLANG_CONFIG = (
    f"monitor = , {MODE}, 0x0, 1\n"
    "misc {\n    disable_hyprland_logo = true\n    disable_splash_rendering = true\n}\n"
    "animations {\n    enabled = false\n}\n"
    "ecosystem {\n    no_update_news = true\n    no_donation_nag = true\n}\n"
)


def config_text(lua: bool) -> str:
    return LUA_CONFIG if lua else HYPRLANG_CONFIG


def output_command(lua: bool) -> list[str]:
    if lua:
        return ["eval", f'hl.monitor({{ output = "{OUTPUT}", mode = "{MODE}", position = "1280x0", scale = 1 }})']
    return ["keyword", "monitor", f"{OUTPUT},{MODE},1280x0,1"]


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


def ensure(tier, seconds: float = 20.0) -> dict:
    """The live second instance, started now if the run has none."""
    record = attached.load(tier.runtime_dir)
    found = current(record)
    if found is not None and found.get("sentinel"):
        return found
    shared, main = Path(record["shared_runtime"]), record["signature"]
    lua = hyprland.uses_lua(shared, main)
    config = tier.runtime_dir / ("second-instance.lua" if lua else "second-instance.conf")
    config.write_text(config_text(lua))
    before = set(hyprland.instances(shared))
    attached_session.spawn(tier, ["Hyprland", "-c", str(config)], _env(tier),
                           tier.runtime_dir / "second-instance.log")
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
