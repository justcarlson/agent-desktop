"""The product run as a child process of a leg, so the leg can signal it or race it, on the attached `hyprland` tier."""

from __future__ import annotations

import subprocess

from runner import product


def running_product(tier, argv: list[str], headed: bool = True) -> subprocess.Popen:
    """The product as a child of this leg, with the run's environment and lease."""
    full = [product._binary_path(), *(["--headed"] if headed else []), *argv]
    env = product._result_env(tier)
    return subprocess.Popen(full, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                            pass_fds=product._load_bounded_process()._interaction_lease_fds(env))


def finished(running: subprocess.Popen, timeout_s: float) -> tuple[int | None, str]:
    try:
        out, _ = running.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        running.kill()
        out, _ = running.communicate(timeout=5)
    return running.returncode, out or ""
