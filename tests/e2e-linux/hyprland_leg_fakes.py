"""Fakes that run a Hyprland pointer leg without a compositor, so a unit test can check the leg's
oracle reads against `runner.verdict.evaluate`."""

from __future__ import annotations

import contextlib
import json
from pathlib import Path
from types import SimpleNamespace

from runner.apps import OwnedApp
from runner.product import Envelope
from runner.verdict import LegContext

CURSOR = {"x": 640, "y": 400}
CLIENT = {"at": [100, 100], "size": [560, 420], "address": "0x1"}
GONE = OwnedApp("gone", 2 ** 22 + 7, 1, [], [], "")


class FakeContext(LegContext):
    """A headed leg context whose product calls answer `answer(argv)`."""

    def __init__(self, answer):
        super().__init__(SimpleNamespace(), "headed", member_reader=lambda _: frozenset())
        self._answer = answer

    def product(self, argv, *, env=None, **kw):
        envelope = self._answer(argv)
        self._products.append(envelope)
        return envelope


def delivered(data: dict) -> Envelope:
    return Envelope(ok=True, data=data, error=None, raw=json.dumps({"data": data}), elapsed_ms=40.0,
                    argv=["--headed"], exit_code=0)


def fake_app(directory: Path, label: str, lines: list[dict]):
    """A context manager like `pointer_pad` or `receiver_fixture`: it yields an app that has already
    exited, the client, and a log holding `lines`."""
    @contextlib.contextmanager
    def app(*_args, **_kw):
        log = directory / f"{label}.log"
        log.write_text("".join(json.dumps(line) + "\n" for line in lines))
        yield GONE, CLIENT, log
    return app


def nested_patches() -> dict:
    """The nested instance's reads: the cursor never moves."""
    return {"nested": lambda tier: ({"signature": "nested"}, False), "instance_env": lambda *a: {},
            "instance_query": lambda *a: dict(CURSOR), "cursor_at": lambda *a: (640.0, 400.0),
            "instance_control": lambda *a: None, "cursor_argv": lambda *a: []}
