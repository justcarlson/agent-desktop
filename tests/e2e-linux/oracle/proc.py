from __future__ import annotations

from pathlib import Path


def stat_fields(text: str) -> tuple[int, int, str]:
    left = text.index("(")
    right = text.rindex(")")
    fields = text[right + 2:].split()
    return int(fields[19]), int(fields[2]), text[left + 1:right]


def environment(data: bytes) -> dict[str, str]:
    return dict(item.decode(errors="surrogateescape").split("=", 1)
                for item in data.split(b"\0") if b"=" in item)


def facts(pid: int, root: Path = Path("/proc")) -> tuple[int, int, str, dict]:
    directory = root / str(pid)
    first = stat_fields((directory / "stat").read_text())
    env = environment((directory / "environ").read_bytes())
    if stat_fields((directory / "stat").read_text()) != first:
        raise ProcessLookupError(f"process generation changed: {pid}")
    return (*first, env)


def may_match(pid: int, comm: str, names) -> bool:
    if any(comm.casefold() == name.casefold()[:15] for name in names):
        return True
    if "python" not in comm.casefold():
        return False
    try:
        args = (Path("/proc") / str(pid) / "cmdline").read_bytes().split(b"\0")
    except (FileNotFoundError, PermissionError):
        return True
    return any(Path(arg.decode(errors="surrogateescape")).name in names for arg in args if arg)


def owned(tier, pid: int, start: int, env: dict) -> bool:
    """Whether a process belongs to the tier: a recorded run member, or a private-runtime process."""
    from runner import attached

    if pid <= 1 or start <= 0:
        return False
    record = attached.load(tier.runtime_dir)
    if record is not None:
        return attached.is_member(record, pid, start, attached.PROC_ROOT)
    return env.get("XDG_RUNTIME_DIR") == str(tier.runtime_dir)


def members(runtime_dir: Path, root: Path = Path("/proc")) -> list[tuple[int, int, str]]:
    from runner import attached

    record = attached.load(runtime_dir)
    if record is not None:
        return attached.members(record, root)
    result = []
    for directory in root.iterdir():
        if not directory.name.isdecimal():
            continue
        try:
            start, _, comm, env = facts(int(directory.name), root)
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
        if env.get("XDG_RUNTIME_DIR") == str(runtime_dir):
            result.append((int(directory.name), start, comm))
    return sorted(result)
