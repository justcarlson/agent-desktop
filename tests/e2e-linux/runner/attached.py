"""Membership records for a tier attached to a live desktop session.

An attached tier shares the session's runtime directory, buses and compositor,
so the runtime directory cannot mark which processes the runner owns. The
record names the systemd user slice that holds every process the runner
starts, the bus-activated generations it adopted after a launch, and every
path it created under the shared runtime directory. Teardown signals and
removes only what the record names.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

from oracle import proc as process

RECORD = "attached.json"
PROC_ROOT = Path("/proc")


def slice_name(pid: int, start: int) -> str:
    return f"ad_e2e_{pid}_{start}.slice"


def new_record(*, slice_name: str, shared_runtime: str, signature: str, mirror: Path) -> dict:
    return {
        "slice": slice_name,
        "shared_runtime": str(shared_runtime),
        "signature": signature,
        "compositor": None,
        "generations": [],
        "instances": [],
        "fixtures": [],
        "survivors": [],
        "mirror": str(mirror),
    }


def load(work_dir: Path) -> dict | None:
    path = Path(work_dir) / RECORD
    if not path.is_file():
        return None
    return json.loads(path.read_text())


def write_record(path: Path, record: dict) -> None:
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(json.dumps(record, sort_keys=True))
    os.replace(temporary, path)


def save(work_dir: Path, record: dict) -> None:
    write_record(Path(work_dir) / RECORD, record)
    mirror = record.get("mirror")
    if mirror and Path(mirror).parent.is_dir():
        write_record(Path(mirror), record)


def _update(work_dir: Path, change) -> dict:
    record = load(work_dir)
    if record is None:
        raise FileNotFoundError(f"no attached record in {work_dir}")
    change(record)
    save(work_dir, record)
    return record


def adopt(work_dir: Path, pid: int, start: int, comm: str) -> None:
    entry = [int(pid), int(start), comm]

    def change(record):
        if entry not in record["generations"]:
            record["generations"].append(entry)

    _update(work_dir, change)


def owned_path(record: dict, path: Path) -> Path:
    hypr = Path(record["shared_runtime"]) / "hypr"
    resolved = Path(os.path.normpath(str(path)))
    if resolved.parent != hypr and hypr not in resolved.parents:
        raise ValueError(f"refusing a path outside {hypr}: {path}")
    if resolved == hypr:
        raise ValueError(f"refusing the shared directory itself: {path}")
    return resolved


def record_fixture(work_dir: Path, path: Path) -> None:
    def change(record):
        resolved = str(owned_path(record, path))
        if resolved not in record["fixtures"]:
            record["fixtures"].append(resolved)

    _update(work_dir, change)


def record_instance(work_dir: Path, instance: dict) -> None:
    def change(record):
        owned_path(record, Path(record["shared_runtime"]) / "hypr" / instance["signature"])
        record["instances"] = [entry for entry in record["instances"]
                               if entry["signature"] != instance["signature"]] + [dict(instance)]

    _update(work_dir, change)


def in_slice(pid: int, slice_unit: str, proc_root: Path = PROC_ROOT) -> bool:
    try:
        text = (Path(proc_root) / str(pid) / "cgroup").read_text()
    except OSError:
        return False
    marker = f"/{slice_unit}/"
    return any(marker in line.split(":", 2)[-1] + "/" for line in text.splitlines())


def _start(pid: int, proc_root: Path) -> tuple[int, str] | None:
    """A live process's start time and name; an exited one awaiting its reaper has none."""
    try:
        text = (Path(proc_root) / str(pid) / "stat").read_text()
        start, _, comm = process.stat_fields(text)
    except (OSError, ValueError, IndexError):
        return None
    if text[text.rindex(")") + 2:].split()[0] == "Z":
        return None
    return start, comm


def members(record: dict, proc_root: Path = PROC_ROOT) -> list[tuple[int, int, str]]:
    found = {}
    for directory in Path(proc_root).glob("[0-9]*"):
        if not directory.name.isdecimal() or not in_slice(int(directory.name), record["slice"], proc_root):
            continue
        facts = _start(int(directory.name), proc_root)
        if facts is not None:
            found[int(directory.name)] = (int(directory.name), facts[0], facts[1])
    for pid, start, _ in record["generations"]:
        facts = _start(pid, proc_root)
        if facts is not None and facts[0] == start:
            found[pid] = (pid, start, facts[1])
    return sorted(found.values())


def is_member(record: dict, pid: int, start: int, proc_root: Path = PROC_ROOT) -> bool:
    return any(member[:2] == (pid, start) for member in members(record, proc_root))
