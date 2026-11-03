"""A Wayland socket whose kernel peer is a compositor process, but which never answers.

Usage: held_wayland_socket.py SOCKET BEHAVIOR -- COMMAND...

The process binds and listens on SOCKET, forks a holder that keeps the
listener, and then execs COMMAND (a nested Hyprland). The kernel records the
peer of a Unix listener when `listen` runs, so every client that connects
reads COMMAND's pid as the peer: the exec keeps the pid. The holder never
accepts (`silent`) or accepts and never replies (`mute`). It removes SOCKET
and exits when COMMAND ends or on SIGTERM.
"""

from __future__ import annotations

import contextlib
import ctypes
import os
import signal
import socket
import sys

SILENT = "silent"
MUTE = "mute"
BEHAVIORS = (SILENT, MUTE)
BACKLOG = 64
PR_SET_PDEATHSIG = 1


def _end(path: str) -> None:
    with contextlib.suppress(FileNotFoundError):
        os.unlink(path)
    os._exit(0)


def _hold(server: socket.socket, path: str, behavior: str, parent: int) -> None:
    signal.signal(signal.SIGTERM, lambda *_: _end(path))
    ctypes.CDLL(None, use_errno=True).prctl(PR_SET_PDEATHSIG, signal.SIGTERM)
    if os.getppid() != parent:
        _end(path)
    held = []
    while True:
        if behavior == MUTE:
            client, _ = server.accept()
            held.append(client)
        else:
            signal.pause()


def main(argv: list[str]) -> int:
    if len(argv) < 4 or argv[2] != "--" or argv[1] not in BEHAVIORS:
        print(__doc__.splitlines()[2], file=sys.stderr)
        return 2
    path, behavior, command = argv[0], argv[1], argv[3:]
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(path)
    server.listen(BACKLOG)
    parent = os.getpid()
    if os.fork() == 0:
        try:
            _hold(server, path, behavior, parent)
        finally:
            _end(path)
    server.close()
    os.execvp(command[0], command)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
