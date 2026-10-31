"""Fixture endpoints for the binding legs: silent, hung-up, answering and stale.

Each listener runs in the runner process, so its socket peer is the runner and
never a compositor. Paths under the shared `hypr/` directory are recorded in
the attached record before they are created, so teardown removes them even
after a cancelled run.
"""

from __future__ import annotations

import contextlib
import itertools
import os
import socket
import tempfile
import threading
from pathlib import Path

from runner import attached

SILENT = "silent"
MUTE = "mute"
HANGUP = "hangup"
ANSWER = "answer"
BEHAVIORS = (SILENT, MUTE, HANGUP, ANSWER)
REPLY = b'{"tag": "ad-e2e-fixture"}'
_COUNTER = itertools.count(1)


class Listener:
    """One AF_UNIX listener with a fixed behavior.

    `silent` never accepts, `mute` accepts and never replies, `hangup` accepts
    and closes, and `answer` replies to one request with `REPLY` and closes.
    """

    def __init__(self, path: Path, behavior: str):
        if behavior not in BEHAVIORS:
            raise ValueError(f"unknown listener behavior: {behavior}")
        self.path = Path(path)
        self.behavior = behavior
        self._held: list[socket.socket] = []
        self._stop = threading.Event()
        self._server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self._server.bind(str(self.path))
        self._server.listen(64)
        self._thread = None
        if behavior != SILENT:
            self._server.settimeout(0.1)
            self._thread = threading.Thread(target=self._serve, daemon=True)
            self._thread.start()

    def _serve(self) -> None:
        while not self._stop.is_set():
            try:
                client, _ = self._server.accept()
            except TimeoutError:
                continue
            except OSError:
                return
            if self.behavior == MUTE:
                self._held.append(client)
                continue
            with client:
                if self.behavior == ANSWER:
                    client.settimeout(1.0)
                    with contextlib.suppress(OSError):
                        client.recv(4096)
                        client.sendall(REPLY)

    def close(self) -> None:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(2.0)
        for client in self._held:
            client.close()
        self._server.close()
        with contextlib.suppress(FileNotFoundError):
            self.path.unlink()


def fixture_dir(work: Path, kind: str) -> Path:
    """Record and create a fake instance directory under the shared `hypr/`."""
    record = attached.load(work)
    directory = Path(record["shared_runtime"]) / "hypr" / f"ad_e2e_{kind}_{os.getpid()}_{next(_COUNTER)}"
    attached.record_fixture(work, directory)
    directory.mkdir(mode=0o700)
    return directory


def instance(work: Path, behavior: str) -> tuple[str, Listener]:
    """A fake instance whose IPC socket has `behavior`; returns its signature."""
    directory = fixture_dir(work, behavior)
    return directory.name, Listener(directory / ".socket.sock", behavior)


def stale_instances(work: Path) -> list[str]:
    """Two dead instance directories: a plain file and a bound socket nobody serves."""
    plain = fixture_dir(work, "stale")
    (plain / ".socket.sock").write_bytes(b"")
    dead = fixture_dir(work, "stale")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as orphan:
        orphan.bind(str(dead / ".socket.sock"))
    return [plain.name, dead.name]


def private_runtime(work: Path) -> Path:
    """A new runtime directory with only stale instances, for the no-live-instance case."""
    runtime = Path(tempfile.mkdtemp(prefix="nl.", dir=work))
    (runtime / "hypr" / "ad_e2e_stale_plain").mkdir(parents=True, mode=0o700)
    (runtime / "hypr" / "ad_e2e_stale_plain" / ".socket.sock").write_bytes(b"")
    dead = runtime / "hypr" / "ad_e2e_stale_dead"
    dead.mkdir(mode=0o700)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as orphan:
        orphan.bind(str(dead / ".socket.sock"))
    runtime.chmod(0o700)
    return runtime
