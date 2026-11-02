"""The product run as a child process of a leg, so the leg can signal it or race it, on the attached `hyprland` tier."""

from __future__ import annotations

import subprocess
from pathlib import Path

from runner import product


DEBUG_FILTER = "agent_desktop_linux=debug"


def running_product(tier, argv: list[str], headed: bool = True, debug_log: Path | None = None) -> subprocess.Popen:
    """The product as a child of this leg, with the run's environment and lease.

    With `debug_log`, the product writes the Linux adapter's debug records to
    that file instead of discarding its stderr.
    """
    full = [product._binary_path(), *(["--headed"] if headed else []), *argv]
    env = product._result_env(tier)
    stderr = subprocess.DEVNULL
    if debug_log is not None:
        env["RUST_LOG"] = DEBUG_FILTER
        stderr = debug_log.open("ab")
    try:
        return subprocess.Popen(full, env=env, stdout=subprocess.PIPE, stderr=stderr, text=True,
                                pass_fds=product._load_bounded_process()._interaction_lease_fds(env))
    finally:
        if stderr is not subprocess.DEVNULL:
            stderr.close()


def finished(running: subprocess.Popen, timeout_s: float) -> tuple[int | None, str]:
    try:
        out, _ = running.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        running.kill()
        out, _ = running.communicate(timeout=5)
    return running.returncode, out or ""
