from __future__ import annotations

import re
import subprocess
from pathlib import Path

from runner.session import Tier

REPO_ROOT = Path(__file__).resolve().parents[3]
ORACLE_SCRIPT = REPO_ROOT / "scripts" / "notification-oracle.sh"


def _script_path() -> Path:
    return ORACLE_SCRIPT


def notification_argv(
    app: str,
    summary: str,
    body: str = "",
    actions: tuple[tuple[str, str], ...] = (),
    *,
    script: Path | None = None,
) -> list[str]:
    path = script or _script_path()
    argv = ["bash", str(path), "notify", "--app", app, "--summary", summary]
    if body:
        argv += ["--body", body]
    for key, label in actions:
        argv += ["--action", f"{key}={label}"]
    return argv


def monitor_argv() -> list[str]:
    return [
        "dbus-monitor",
        "--session",
        "type='signal',interface='org.freedesktop.Notifications'",
    ]


def parse_notification_id(stdout: str) -> int:
    found = re.findall(r"\d+", stdout)
    if not found:
        raise ValueError(f"no notification id in oracle output: {stdout!r}")
    return int(found[-1])


def send(
    tier: Tier,
    app: str,
    summary: str,
    body: str = "",
    actions: tuple[tuple[str, str], ...] = (),
) -> int:
    argv = notification_argv(app, summary, body, actions)
    completed = subprocess.run(
        argv, env=tier.env, capture_output=True, text=True, timeout=25, check=False
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"notification oracle failed ({completed.returncode}): {completed.stderr.strip()}"
        )
    return parse_notification_id(completed.stdout)


def monitor(tier: Tier, path: Path) -> int:
    handle = open(path, "w", encoding="utf-8")
    try:
        spawned = subprocess.Popen(
            monitor_argv(),
            env=tier.env,
            stdout=handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
    finally:
        handle.close()
    return spawned.pid


def parse_monitor_text(text: str) -> list[tuple[str, dict]]:
    signals: list[tuple[str, dict]] = []
    for block in re.split(r"\n\s*\n", text):
        member = re.search(r"member=(\w+)", block)
        if not member:
            continue
        name = member.group(1)
        if name == "NotificationClosed":
            values = [int(v) for v in re.findall(r"^\s+uint32 (\d+)$", block, re.M)]
            if len(values) >= 2:
                signals.append((name, {"id": values[0], "reason": values[1]}))
        elif name == "ActionInvoked":
            numbers = re.findall(r"^\s+uint32 (\d+)$", block, re.M)
            strings = re.findall(r'^\s+string "(.*)"$', block, re.M)
            if numbers:
                signals.append(
                    (
                        name,
                        {"id": int(numbers[0]), "action": strings[0] if strings else ""},
                    )
                )
    return signals


def compare_records(expected: list[dict], listed: list[dict]) -> list[str]:
    problems: list[str] = []
    if len(expected) != len(listed):
        problems.append(f"count {len(listed)} != {len(expected)}")
    for position, (want, got) in enumerate(zip(expected, listed), start=1):
        if got.get("app_name") != want["app"]:
            problems.append(f"{position}: app {got.get('app_name')!r} != {want['app']!r}")
        if got.get("title") != want["summary"]:
            problems.append(f"{position}: title {got.get('title')!r} != {want['summary']!r}")
        if got.get("index") != position:
            problems.append(f"{position}: index {got.get('index')!r}")
    return problems
