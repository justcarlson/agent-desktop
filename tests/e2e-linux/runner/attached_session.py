"""The `hyprland` tier: attach to the live Hyprland session the runner runs in.

The tier shares the session's compositor, buses, portals and runtime
directory. The runner owns a private work directory (its tier
`runtime_dir`, which carries the owner marker and the agent-desktop state
root), one transient systemd user slice for every process it starts, and the
paths it records under the shared `hypr/` directory. Teardown stops the slice,
signals only recorded generations, removes only recorded paths, and checks
that the session processes it found at start are still the same processes.
"""

from __future__ import annotations

import contextlib
import os
import re
import shlex
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from oracle import hyprland, proc as process
from runner import attached, attached_guard, session
from runner.session import OWNER_MARKER, TeardownError, Tier, TierUnavailable

NAME = "hyprland"
WORKSPACE = "ad_e2e"
RULE = "ad_e2e_place"
REQUIRED = ("XDG_RUNTIME_DIR", "HYPRLAND_INSTANCE_SIGNATURE", "WAYLAND_DISPLAY",
            "DBUS_SESSION_BUS_ADDRESS")
PASSED = ("HOME", "PATH", "LANG", "XDG_CURRENT_DESKTOP", "XDG_CONFIG_HOME", "XDG_DATA_HOME",
          "XDG_CACHE_HOME", "LD_LIBRARY_PATH")
NOTIFIER = "org.freedesktop.Notifications"
SAFE_PATH = re.compile(r"[A-Za-z0-9_./-]+")
WAYLAND_SOCKET = re.compile(r"wayland-\d+")


def required_env(source: dict[str, str]) -> dict[str, str]:
    missing = [key for key in REQUIRED if not source.get(key)]
    if missing:
        raise TierUnavailable(f"{NAME}: not inside a Hyprland session; unset: {', '.join(missing)}")
    return {key: source[key] for key in REQUIRED}


def _unix_path(address: str) -> Path | None:
    for part in address.split(";")[0].split(":", 1)[-1].split(","):
        if part.startswith("path="):
            return Path(part[5:])
    return None


def _generation(pid: int, role: str, proc_root: Path) -> list | None:
    try:
        start, _, comm = process.stat_fields((proc_root / str(pid) / "stat").read_text())
    except (OSError, ValueError, IndexError):
        return None
    return [pid, start, comm, role]


def _peer(path: Path | None) -> int | None:
    if path is None:
        return None
    try:
        return hyprland.peer_pid(path)
    except OSError:
        return None


def _bus_owner(env: dict[str, str], name: str) -> int | None:
    result = subprocess.run(
        ["gdbus", "call", "--session", "-d", "org.freedesktop.DBus", "-o", "/org/freedesktop/DBus",
         "-m", "org.freedesktop.DBus.GetConnectionUnixProcessID", name],
        env={"PATH": env["PATH"], "DBUS_SESSION_BUS_ADDRESS": env["DBUS_SESSION_BUS_ADDRESS"]},
        capture_output=True, text=True, timeout=5, check=False)
    match = re.search(r"uint32 (\d+)", result.stdout)
    return int(match[1]) if result.returncode == 0 and match else None


def _layer_owners(runtime: Path, signature: str, compositor: int) -> list[int]:
    owners = []
    for output in hyprland.query(runtime, signature, "layers", pid=compositor).values():
        for level in output.get("levels", {}).values():
            owners.extend(layer["pid"] for layer in level if layer.get("pid", 0) > 1)
    return owners


def survivors(tier: Tier, record: dict, proc_root: Path = Path("/proc")) -> list[list]:
    """Session processes that must outlive the run, found by role, not by name."""
    runtime, signature = Path(record["shared_runtime"]), record["signature"]
    compositor = record["compositor"][0]
    found = {compositor: "compositor"}
    for role, pid in (("session-bus", _peer(_unix_path(tier.bus_address))),
                      ("a11y-bus", _peer(_unix_path(tier.a11y_bus_address))),
                      ("notifier", _bus_owner(tier.env, NOTIFIER))):
        if pid:
            found.setdefault(pid, role)
    for pid in _layer_owners(runtime, signature, compositor):
        found.setdefault(pid, "layer-shell")
    bystander = record.get("bystander")
    if bystander:
        found.setdefault(bystander["pid"], "bystander")
    entries = (_generation(pid, role, proc_root) for pid, role in sorted(found.items()))
    return [entry for entry in entries
            if entry and not attached.is_member(record, entry[0], entry[1], proc_root)]


def _bystander(runtime: Path, signature: str, compositor: int, record: dict,
               proc_root: Path) -> dict | None:
    active = hyprland.query(runtime, signature, "activewindow", pid=compositor)
    pid = active.get("pid", 0)
    generation = _generation(pid, "bystander", proc_root) if pid > 1 else None
    if generation is None or attached.is_member(record, pid, generation[1], proc_root):
        return None
    return {"pid": pid, "start": generation[1], "address": active.get("address"),
            "class": active.get("class"), "title": active.get("title")}


def _tier_env(source: dict[str, str], values: dict[str, str], state_root: Path) -> dict[str, str]:
    env = {key: source[key] for key in PASSED if source.get(key)}
    env.setdefault("PATH", os.defpath)
    env.setdefault("LANG", "C.UTF-8")
    env.update(values)
    env.update(AGENT_DESKTOP_HOME=str(state_root), GDK_BACKEND="wayland",
               XDG_SESSION_TYPE="wayland", QT_QPA_PLATFORM="wayland")
    return env


def _compositor(values: dict[str, str]) -> int:
    runtime = Path(values["XDG_RUNTIME_DIR"])
    signature = values["HYPRLAND_INSTANCE_SIGNATURE"]
    ipc = _peer(hyprland.socket_path(runtime, signature))
    if ipc is None:
        raise TierUnavailable(f"{NAME}: instance {signature} does not answer on its IPC socket")
    display = _peer(hyprland.wayland_path(runtime, values["WAYLAND_DISPLAY"]))
    if display != ipc:
        raise TierUnavailable(f"{NAME}: {values['WAYLAND_DISPLAY']} is served by pid {display}, "
                              f"not by the compositor {ipc}")
    hyprland.query(runtime, signature, "version", pid=ipc)
    return ipc


def probe(tier: Tier) -> None:
    record = attached.load(tier.runtime_dir)
    if record is None:
        raise RuntimeError("the attached record is missing")
    if _compositor(tier.env) != record["compositor"][0]:
        raise RuntimeError("the compositor generation changed")


@contextlib.contextmanager
def start(deadline: float, source: dict[str, str] | None = None):
    source = dict(os.environ if source is None else source)
    attached_guard.require(source)
    values = required_env(source)
    monotonic_deadline = time.monotonic() + deadline
    compositor = _compositor(values)
    proc_root = Path("/proc")
    base = session.session_base(source)
    base.mkdir(mode=0o700, parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="rt.", dir=base))
    try:
        own = session._own_start_time()
        (work / OWNER_MARKER).write_text(f"{os.getpid()} {own}\n")
        state_root = work / "ad-home"
        state_root.mkdir(mode=0o700)
        suite = Path(source.get("AGENT_DESKTOP_E2E_SUITE") or tempfile.mkdtemp(prefix="ad-tier-"))
        suite.mkdir(parents=True, exist_ok=True)
        record = attached.new_record(
            slice_name=attached.slice_name(os.getpid(), own),
            shared_runtime=values["XDG_RUNTIME_DIR"],
            signature=values["HYPRLAND_INSTANCE_SIGNATURE"],
            mirror=suite / f"{NAME}.attached.json")
        generation = _generation(compositor, "compositor", proc_root)
        if generation is None:
            raise TierUnavailable(f"{NAME}: the compositor exited during attach")
        record["compositor"] = generation[:2]
        attached.save(work, record)
        tier = Tier(name=NAME, runtime_dir=work, display=None,
                    wayland_display=values["WAYLAND_DISPLAY"],
                    bus_address=values["DBUS_SESSION_BUS_ADDRESS"], a11y_bus_address="",
                    state_root=state_root, keyfile=work / "keyfile",
                    env=_tier_env(source, values, state_root))
        address = session._a11y_address(tier.env, monotonic_deadline)
        if address is None:
            raise TierUnavailable(f"{NAME}: the accessibility bus did not answer")
        tier.a11y_bus_address = address
        tier.env["AT_SPI_BUS_ADDRESS"] = address
        if not session._a11y_warm(tier.env, monotonic_deadline):
            raise TierUnavailable(f"{NAME}: the accessibility registry did not come up")
        runtime, signature = Path(record["shared_runtime"]), record["signature"]
        record["bystander"] = _bystander(runtime, signature, compositor, record, proc_root)
        record["survivors"] = survivors(tier, record, proc_root)
        attached.save(work, record)
        from runner import cancel
        cancel.publish(suite, tier)
    except BaseException:
        teardown(work, check_survivors=False)
        raise
    try:
        yield tier
    finally:
        teardown(work)


def launcher(tier: Tier, argv: list[str], env: dict[str, str], log: Path) -> Path:
    record = attached.load(tier.runtime_dir)
    existing = len(list(tier.runtime_dir.glob("launch-*.sh")))
    script = tier.runtime_dir / f"launch-{existing + 1}.sh"
    if not SAFE_PATH.fullmatch(str(script)):
        raise ValueError(f"unsafe launcher path: {script}")
    assignments = " ".join(shlex.quote(f"{key}={value}") for key, value in sorted(env.items()))
    command = " ".join(shlex.quote(part) for part in argv)
    script.write_text(
        "#!/bin/sh\n"
        f"exec systemd-run --user --scope --quiet --collect --slice={record['slice']} -- "
        f"/usr/bin/env -i {assignments} {command} >>{shlex.quote(str(log))} 2>&1\n")
    return script


def _placed(runtime: Path, signature: str, app_class: str, known: set[str]) -> list[dict]:
    return [item for item in hyprland.query(runtime, signature, "clients")
            if item.get("class") == app_class and item.get("address") not in known]


def _await_placed(runtime: Path, signature: str, toplevels: tuple[str, int], known: set[str],
                  timeout_s: float, workspace: str = WORKSPACE) -> None:
    app_class, count = toplevels
    deadline = time.monotonic() + timeout_s
    while len(found := _placed(runtime, signature, app_class, known)) < count:
        if time.monotonic() >= deadline:
            raise RuntimeError(f"{len(found)} of {count} {app_class} toplevels mapped in {timeout_s}s")
        time.sleep(0.05)
    stray = [item.get("address") for item in found
             if (item.get("workspace") or {}).get("name") != f"special:{workspace}"]
    if stray:
        raise RuntimeError(f"{app_class} toplevels {stray} mapped outside special:{workspace}")


def spawn(tier: Tier, argv: list[str], env: dict[str, str], log: Path, workspace: str = WORKSPACE, *,
          toplevels: tuple[str, int] | None = None, timeout_s: float = 10.0) -> None:
    """Start a process in the run's slice, through the compositor, without taking focus, on the
    special workspace `workspace`.

    The exec rule places only the first window the process maps. For a process that maps
    `toplevels = (class, count)` windows, a named class rule places the rest; it is enabled for
    this call only and stays recorded until it is disabled, so teardown can disable it too."""
    record = attached.load(tier.runtime_dir)
    runtime, signature = Path(record["shared_runtime"]), record["signature"]
    script = launcher(tier, argv, env, log)
    if toplevels is None:
        hyprland.exec_silently(runtime, signature, f"/bin/sh {script}", workspace)
        return
    if not hyprland.uses_lua(runtime, signature):
        raise RuntimeError(f"{toplevels[0]}: {hyprland.LEGACY_RULES}")
    known = {item.get("address") for item in hyprland.query(runtime, signature, "clients")}
    if RULE not in record.setdefault("rules", []):
        record["rules"].append(RULE)
    attached.save(tier.runtime_dir, record)
    try:
        hyprland.place_class(runtime, signature, RULE, toplevels[0], workspace)
        hyprland.exec_silently(runtime, signature, f"/bin/sh {script}", workspace)
        _await_placed(runtime, signature, toplevels, known, timeout_s, workspace)
    finally:
        _disable_rules(tier.runtime_dir)


def _disable_rules(work: Path) -> list[str]:
    record = attached.load(work)
    if record is None or not record.get("rules"):
        return []
    errors = []
    for name in list(record.get("rules", [])):
        try:
            hyprland.disable_rule(Path(record["shared_runtime"]), record["signature"], name)
        except (OSError, RuntimeError, subprocess.SubprocessError) as error:
            errors.append(f"window rule {name} was not disabled: {error}")
            continue
        record["rules"].remove(name)
    attached.save(work, record)
    return errors


def boot_ticks() -> int:
    return int(time.clock_gettime(time.CLOCK_BOOTTIME) * os.sysconf("SC_CLK_TCK"))


def adopt_activated(tier: Tier, commands: tuple[str, ...], since: int,
                    proc_root: Path = Path("/proc")) -> list[tuple[int, int, str]]:
    """Record bus-activated generations of a launched app that started outside the slice."""
    record = attached.load(tier.runtime_dir)
    names = {command[:15] for command in commands}
    adopted = []
    for directory in proc_root.glob("[0-9]*"):
        try:
            start, _, comm, env = process.facts(int(directory.name), proc_root)
        except (OSError, ValueError, IndexError):
            continue
        if (start >= since and comm in names
                and env.get("XDG_RUNTIME_DIR") == record["shared_runtime"]
                and env.get("WAYLAND_DISPLAY") == tier.wayland_display
                and not attached.is_member(record, int(directory.name), start, proc_root)):
            attached.adopt(tier.runtime_dir, int(directory.name), start, comm)
            adopted.append((int(directory.name), start, comm))
    return adopted


def _stop_slice(record: dict) -> None:
    shared = record["shared_runtime"]
    env = {"PATH": os.environ.get("PATH", os.defpath), "XDG_RUNTIME_DIR": shared,
           "DBUS_SESSION_BUS_ADDRESS": os.environ.get("DBUS_SESSION_BUS_ADDRESS")
           or f"unix:path={shared}/bus"}
    with contextlib.suppress(OSError, subprocess.TimeoutExpired):
        subprocess.run(["systemctl", "--user", "stop", record["slice"]], env=env,
                       capture_output=True, timeout=30, check=False)


def _alive(pid: int, start: int, proc_root: Path) -> bool:
    entry = _generation(pid, "", proc_root)
    return entry is not None and entry[1] == start


def remove_path(path: Path) -> bool:
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path, ignore_errors=True)
    else:
        with contextlib.suppress(FileNotFoundError):
            path.unlink()
    return not os.path.lexists(path)


def instance_paths(record: dict, instance: dict) -> list[Path]:
    shared = Path(record["shared_runtime"])
    paths = [attached.owned_path(record, shared / "hypr" / instance["signature"])]
    display = instance.get("wayland")
    if display:
        if not WAYLAND_SOCKET.fullmatch(display):
            raise ValueError(f"refusing to remove {display!r}")
        if not hyprland.listening(shared / display):
            paths.extend((shared / display, shared / f"{display}.lock"))
    return paths


def remaining(record: dict, proc_root: Path = Path("/proc")) -> list[str]:
    """What a finished teardown must not leave behind; empty when clean."""
    problems = [f"member survived: {member}" for member in attached.members(record, proc_root)]
    for instance in record["instances"]:
        if _alive(instance["pid"], instance["start"], proc_root):
            problems.append(f"second instance survived: {instance['signature']}")
    shared = Path(record["shared_runtime"])
    for path in record["fixtures"]:
        if os.path.lexists(path):
            problems.append(f"fixture left behind: {path}")
    for instance in record["instances"]:
        if os.path.lexists(shared / "hypr" / instance["signature"]):
            problems.append(f"instance directory left behind: {instance['signature']}")
    return problems


def dead_survivors(record: dict, proc_root: Path = Path("/proc")) -> list[str]:
    return [f"{role} {comm} ({pid}) did not survive the run"
            for pid, start, comm, role in record["survivors"] if not _alive(pid, start, proc_root)]


def teardown(work: Path, *, check_survivors: bool = True, proc_root: Path = Path("/proc"),
             stop_slice=None) -> None:
    errors = _disable_rules(work)
    record = attached.load(work)
    if record is not None:
        (stop_slice or _stop_slice)(record)
        session._sweep_members(work, proc_root)
        for instance in record["instances"]:
            if not _alive(instance["pid"], instance["start"], proc_root):
                for path in instance_paths(record, instance):
                    remove_path(path)
        for path in record["fixtures"]:
            remove_path(attached.owned_path(record, Path(path)))
        errors.extend(remaining(record, proc_root))
        if check_survivors:
            errors.extend(dead_survivors(record, proc_root))
        record["torn_down"] = True
        record["teardown_errors"] = errors
        mirror = record.get("mirror")
        if mirror and Path(mirror).parent.is_dir():
            attached.write_record(Path(mirror), record)
    if work.exists() and not session._remove_tree(work):
        errors.append(f"the work directory could not be removed: {work}")
    if errors:
        raise TeardownError("; ".join(errors))
