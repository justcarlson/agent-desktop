from __future__ import annotations

import os
import re
import signal
import subprocess
from dataclasses import dataclass
from pathlib import Path

from oracle import atspi_raw as raw, proc, x11
from oracle.gsettings import tool
from runner.session import Tier


@dataclass
class OwnedApp:
    name: str
    pid: int
    start_time: int
    owner_names: list[str]
    windows: list[str]
    display: str


COMMANDS = {
    "Calculator": "gnome-calculator", "Files": "nautilus", "Terminal": "gnome-terminal",
    "Text Editor": "gnome-text-editor", "Settings": "gnome-control-center",
}
ALIASES = {
    "Calculator": ("calculator",), "Files": ("nautilus", "files"),
    "Terminal": ("terminal",), "Text Editor": ("text-editor", "text editor"),
    "Settings": ("control-center", "settings"),
}
_TIERS = {}
_SPAWNED = {}


def window_id(node) -> str:
    if not re.fullmatch(r":\d+\.\d+", node.bus):
        raise ValueError("window must have a unique accessibility bus owner")
    if not re.fullmatch(r"/(?:[A-Za-z0-9_]+/)*[A-Za-z0-9_]+", node.path):
        raise ValueError("invalid accessibility object path")
    return "w-" + node.bus[1:] + "-" + node.path[1:].replace("/", "-")


def attributable(tier, pid, start, env) -> bool:
    return (pid > 1 and start > 0 and
            env.get("XDG_RUNTIME_DIR") == str(tier.runtime_dir))


def _owners(tier: Tier) -> list[OwnedApp]:
    result = []
    for node in raw.registry(tier):
        pid = raw.owner_pid(node)
        try:
            start, _, _, env = proc.facts(pid)
        except (FileNotFoundError, ProcessLookupError):
            continue
        if not attributable(tier, pid, start, env):
            continue
        frames = [child for child in node.children() if child.role in (16, 23, 69)]
        result.append(OwnedApp(node.name, pid, start, [node.bus],
                               [window_id(frame) for frame in frames], env.get("DISPLAY", "")))
    return result


def owners_named(tier: Tier, name: str) -> list[OwnedApp]:
    needles = ALIASES.get(name, (name.casefold(),))
    result = {}
    for app in _owners(tier):
        if any(needle in app.name.casefold() for needle in needles):
            key = app.pid, app.start_time
            if key in result:
                result[key].owner_names.extend(app.owner_names)
                result[key].windows.extend(app.windows)
            else:
                result[key] = app
    return list(result.values())


def _display_verified(tier, app) -> bool:
    if not tier.display:
        return True
    if app.display != tier.display:
        raise AssertionError(f"{app.name} mapped on {app.display}, expected {tier.display}")
    mapped = []
    for xid in x11.stacking(tier):
        text = tool(tier, ["xprop", "-id", xid, "_NET_WM_PID"])
        match = re.search(r"=\s*(\d+)", text)
        if match and int(match[1]) == app.pid:
            x11.geometry(tier, xid)
            mapped.append(xid)
    return len(mapped) >= len(app.windows)


def launch(tier: Tier, app: str, args: tuple[str, ...] = (), *, settle_s: float = 5.0) -> OwnedApp:
    before = {(item.pid, item.start_time): set(item.windows) for item in _owners(tier)}
    env = dict(tier.env)
    if app.endswith("qt_window.py"):
        command = [str(Path.home() / ".local/ad-tools/qtvenv/bin/python"), app]
        env.update(QT_ACCESSIBILITY="1", QT_LINUX_ACCESSIBILITY_ALWAYS_ON="1", QT_QPA_PLATFORM="xcb")
    else:
        command = [COMMANDS.get(app, app)]
    if "code" in Path(command[0]).name:
        env["ACCESSIBILITY_ENABLED"] = "1"
    log = tier.runtime_dir / "app-launch.log"
    with log.open("ab") as stream:
        child = subprocess.Popen(command + list(args), env=env, stdout=stream, stderr=stream,
                                 start_new_session=True)
    _SPAWNED[child.pid] = child

    def locate():
        candidates = [item for item in _owners(tier) if item.windows and
                      set(item.windows) - before.get((item.pid, item.start_time), set())]
        if len(candidates) > 1:
            raise AssertionError(f"launch attributed multiple owners: {candidates}")
        if candidates and _display_verified(tier, candidates[0]):
            return candidates[0]
        return None

    found = raw.poll(locate, settle_s)
    if not found:
        raise AssertionError(f"no new private window for {app}; launcher exit={child.poll()}; log={log}")
    _TIERS[found.pid, found.start_time] = tier
    return found


def stop(app: OwnedApp) -> None:
    tier = _TIERS.get((app.pid, app.start_time))
    if tier is None:
        raise ValueError("refusing to stop an unattributed process")

    def alive():
        try:
            stat = (Path("/proc") / str(app.pid) / "stat").read_text()
            if stat[stat.rindex(")") + 2:].split()[0] == "Z":
                return False
            start, group, _, env = proc.facts(app.pid)
        except (FileNotFoundError, ProcessLookupError):
            return False
        if start != app.start_time:
            return False
        if not attributable(tier, app.pid, start, env):
            raise AssertionError("owned process left its runtime directory")
        return group

    group = alive()
    if group:
        signal_owner = os.killpg if group == app.pid else os.kill
        signal_owner(app.pid, signal.SIGTERM)
        if raw.poll(lambda: not alive(), 2.0) is not True:
            signal_owner(app.pid, signal.SIGKILL)
        if not raw.poll(lambda: not alive(), 2.0):
            raise AssertionError(f"owned process survived: {app.pid}")
    for pid, child in list(_SPAWNED.items()):
        if child.poll() is not None:
            child.wait()
            del _SPAWNED[pid]


def launch_qt(tier: Tier) -> OwnedApp:
    path = Path(__file__).resolve().parents[1] / "apps/qt_window.py"
    log = tier.runtime_dir / "qt-window.log"
    if log.exists():
        tool(tier, ["trash", str(log)])
    return launch(tier, str(path), (str(tier.runtime_dir / "qt-window.log"),))


def launch_vscode(tier: Tier) -> OwnedApp:
    executable = str(Path.home() / ".local/ad-tools/vscode/app/code")
    args = ("--no-sandbox", "--password-store=basic", "--user-data-dir",
            str(tier.runtime_dir / "vscode-profile"), "--extensions-dir",
            str(tier.runtime_dir / "vscode-extensions"), "--disable-extensions", "--disable-gpu",
            "--ozone-platform=x11", "--new-window")
    return launch(tier, executable, args, settle_s=35)
