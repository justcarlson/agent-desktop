"""Independent reads of a Hyprland instance over its own IPC socket.

Every read checks the socket's peer process first, so a stale or foreign
listener at an instance path never answers for the compositor.
"""

from __future__ import annotations

import json
import os
import re
import socket
import struct
import subprocess
import time
from pathlib import Path

SIGNATURE = re.compile(r"[A-Za-z0-9_]+")


def instance_dir(runtime: Path, signature: str) -> Path:
    if not SIGNATURE.fullmatch(signature):
        raise ValueError(f"invalid instance signature: {signature!r}")
    return Path(runtime) / "hypr" / signature


def socket_path(runtime: Path, signature: str) -> Path:
    return instance_dir(runtime, signature) / ".socket.sock"


def wayland_path(runtime: Path, display: str) -> Path:
    return Path(display) if os.path.isabs(display) else Path(runtime) / display


def _connect(path: Path, timeout: float) -> socket.socket:
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(timeout)
    try:
        client.connect(str(path))
    except OSError:
        client.close()
        raise
    return client


def peer_pid(path: Path, timeout: float = 1.0) -> int:
    with _connect(path, timeout) as client:
        credentials = client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i"))
        return struct.unpack("3i", credentials)[0]


def listening(path: Path) -> bool:
    try:
        peer_pid(path, 0.5)
    except OSError:
        return False
    return True


def request(runtime: Path, signature: str, command: str, *, pid: int | None = None,
            timeout: float = 2.0) -> bytes:
    deadline = time.monotonic() + timeout
    with _connect(socket_path(runtime, signature), timeout) as client:
        if pid is not None:
            credentials = client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i"))
            peer = struct.unpack("3i", credentials)[0]
            if peer != pid:
                raise RuntimeError(f"instance {signature} answers from pid {peer}, expected {pid}")
        client.sendall(command.encode())
        chunks = []
        while True:
            client.settimeout(max(0.01, deadline - time.monotonic()))
            chunk = client.recv(65536)
            if not chunk:
                return b"".join(chunks)
            chunks.append(chunk)


def query(runtime: Path, signature: str, name: str, *, pid: int | None = None):
    return json.loads(request(runtime, signature, f"j/{name}", pid=pid))


def desktop(runtime: Path, signature: str, *, pid: int | None = None) -> dict:
    """The user-visible state a run must leave alone. The focused window is
    named by its initial title: a shell inside it may retitle it at any time."""
    active = query(runtime, signature, "activewindow", pid=pid)
    monitors = query(runtime, signature, "monitors", pid=pid)
    return {
        "active_window": {key: active.get(key) for key in ("address", "pid", "class", "initialTitle")},
        "cursor": query(runtime, signature, "cursorpos", pid=pid),
        "workspace": query(runtime, signature, "activeworkspace", pid=pid).get("name"),
        "monitors": sorted((item["name"], item.get("focused"), item["activeWorkspace"]["name"],
                            (item.get("specialWorkspace") or {}).get("name", ""))
                           for item in monitors),
    }


def control(runtime: Path, signature: str, argv: list[str], *, timeout: float = 5.0) -> str:
    """Run one hyprctl command against exactly one instance."""
    instance_dir(runtime, signature)
    env = {"PATH": os.environ.get("PATH", os.defpath), "XDG_RUNTIME_DIR": str(runtime),
           "HYPRLAND_INSTANCE_SIGNATURE": signature}
    result = subprocess.run(["hyprctl", *argv], env=env, capture_output=True, text=True,
                            timeout=timeout, check=False)
    output = (result.stdout + result.stderr).strip()
    if result.returncode != 0:
        raise RuntimeError(f"hyprctl {argv[0]} failed: {output}")
    return output


def uses_lua(runtime: Path, signature: str) -> bool:
    return control(runtime, signature, ["eval", "return 1"]) == "ok"


def lua_string(text: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9_ ./:=,@+-]*", text):
        raise ValueError(f"refusing to embed {text!r} in a compositor command")
    return f'"{text}"'


def exec_silently(runtime: Path, signature: str, command: str, workspace: str) -> None:
    """Start a command through the compositor on a special workspace, without focus."""
    target = f"special:{workspace} silent"
    quoted = lua_string(command)
    if uses_lua(runtime, signature):
        argv = ["dispatch", f"hl.dsp.exec_cmd({quoted}, {{ workspace = {lua_string(target)} }})"]
    else:
        argv = ["dispatch", "exec", f"[workspace {target}] {command}"]
    output = control(runtime, signature, argv)
    if output != "ok":
        raise RuntimeError(f"compositor refused to start {command!r}: {output}")


APP_CLASS = re.compile(r"[A-Za-z0-9_.-]+")
LEGACY_RULES = "a legacy hyprlang config cannot remove a runtime window rule without a reload"


def class_rule_code(name: str, app_class: str, workspace: str) -> str:
    """A named Lua rule that opens every new window of exactly `app_class` on a special workspace,
    without focus."""
    if not SIGNATURE.fullmatch(name) or not APP_CLASS.fullmatch(app_class):
        raise ValueError(f"refusing rule {name!r} for class {app_class!r}")
    pattern = "^" + app_class.replace(".", "[.]") + "$"
    target = lua_string(f"special:{workspace} silent")
    return (f'hl.window_rule({{ name = "{name}", enabled = true, match = {{ class = "{pattern}" }}, '
            f"workspace = {target} }})")


def disable_rule_code(name: str) -> str:
    if not SIGNATURE.fullmatch(name):
        raise ValueError(f"refusing rule {name!r}")
    return f'hl.window_rule({{ name = "{name}", enabled = false }})'


def _eval_rule(runtime: Path, signature: str, name: str, code: str) -> None:
    if not uses_lua(runtime, signature):
        raise RuntimeError(LEGACY_RULES)
    output = control(runtime, signature, ["eval", code])
    if output != "ok":
        raise RuntimeError(f"compositor refused window rule {name}: {output}")


def place_class(runtime: Path, signature: str, name: str, app_class: str, workspace: str) -> None:
    """Hyprland consumes an exec rule on the first window its process maps (v0.56.2
    src/desktop/rule/windowRule/WindowRuleApplicator.cpp, readStaticRules: unregisterRule), so a
    process with more than one toplevel needs this named class rule for the rest. Only the Lua
    config can disable a runtime rule; a legacy `keyword windowrule` stays until a config reload."""
    _eval_rule(runtime, signature, name, class_rule_code(name, app_class, workspace))


def disable_rule(runtime: Path, signature: str, name: str) -> None:
    _eval_rule(runtime, signature, name, disable_rule_code(name))


def instances(runtime: Path) -> list[str]:
    directory = Path(runtime) / "hypr"
    if not directory.is_dir():
        return []
    return sorted(entry.name for entry in directory.iterdir() if SIGNATURE.fullmatch(entry.name))
