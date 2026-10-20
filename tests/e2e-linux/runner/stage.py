from __future__ import annotations

import hashlib
import stat
from pathlib import Path

from runner.session import _load_bounded_process


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def stage_binary(source: Path, suite_dir: Path, bounded=None) -> Path:
    source = Path(source)
    if not source.is_file():
        raise FileNotFoundError(f"release binary not found: {source}")
    bin_dir = Path(suite_dir) / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    target = bin_dir / "agent-desktop"
    staged_sha = bin_dir / "agent-desktop.sha256"
    if target.exists():
        recorded = staged_sha.read_text().split()[0] if staged_sha.exists() else ""
        if recorded and recorded == _sha256(target):
            return target
        raise StagedBinaryModified(
            f"the staged binary at {target} does not match its recorded hash"
        )
    source_hash = _sha256(source)
    target.write_bytes(source.read_bytes())
    target.chmod(stat.S_IRUSR | stat.S_IXUSR | stat.S_IRGRP | stat.S_IXGRP)
    staged_hash = _sha256(target)
    if staged_hash != source_hash:
        raise StagedBinaryModified("the staged copy does not match the source hash")
    bounded = bounded or _load_bounded_process()
    result = bounded.run_bounded([str(target), "--version"], timeout_seconds=15)
    if result.returncode != 0:
        raise RuntimeError(f"the staged binary does not answer --version: {result.stderr}")
    staged_sha.write_text(f"{staged_hash}  {source}\n")
    return target


def verify_staged(target: Path, suite_dir: Path) -> bool:
    staged_sha = Path(suite_dir) / "bin" / "agent-desktop.sha256"
    if not staged_sha.is_file() or not Path(target).is_file():
        return False
    recorded = staged_sha.read_text().split()[0]
    return recorded == _sha256(target)


class StagedBinaryModified(Exception):
    pass
