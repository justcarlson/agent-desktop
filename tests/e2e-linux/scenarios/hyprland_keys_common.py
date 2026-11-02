"""The key log app and the compositor reads the `hyprland` key legs share.

The app is one GTK3 entry that logs every key event it receives, with its
keysym, modifier state and layout group. `j/devices` names every keyboard the
compositor has, so a virtual keyboard left behind, or a layout left switched,
shows in it.

While the app runs, the harness keeps one AT-SPI listener registered for it.
Each product process registers as a listener, and the app's ATK bridge drops
GTK's key event listener when its last listener's bus name goes away. A
product killed while its keys still arrive made that drop run inside GTK's
own walk of its key listeners, a reentrant D-Bus call, and GTK then read the
freed list node (SIGSEGV in gtk_main_do_event, gtkaccessibilityutil.c:154,
gtk3 3.24.52 with at-spi2-core 2.62). The harness listener keeps the bridge
from ever having no listener during a leg.
"""

from __future__ import annotations

import contextlib
import json
import shutil
import time
from pathlib import Path

from oracle import atspi_raw as raw
from oracle import hyprland
from runner import apps, attached_session
from runner.helper import require
from scenarios.hyprland_close import _register, _release, private_interpreter
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_focus_controls import active, clients_of, control, focus_argv
from scenarios.wayland_input import text_ref

APP_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "key_log.py"
KEY_LOG_CLASS = "ad-key-log"
KEY_LOG_NAMES = ("key log", KEY_LOG_CLASS)
OPAQUE = "opaque"
OPAQUE_NAME = "opaque key log field"
SETTLE_S = 10.0
REGISTRY = "org.a11y.atspi.Registry"
REGISTRY_PATH = "/org/a11y/atspi/registry"
LISTENED_EVENT = "object:"
TEXT_ROLE = 61
SHIFT, CONTROL, ALT, SUPER = 1, 4, 8, 64
HELD_MODIFIERS = SHIFT | CONTROL | ALT | SUPER


def _owner(tier):
    found = [app for app in apps._owners(tier, names=KEY_LOG_NAMES) if len(app.windows) == 1]
    return found[0] if len(found) == 1 else None


@contextlib.contextmanager
def listener_anchor(tier, owner_name: str):
    """One AT-SPI client that listens to `owner_name` until the block ends."""
    connection = raw.connect(tier.a11y_bus_address)
    try:
        raw.call(connection, REGISTRY, REGISTRY_PATH, REGISTRY, "RegisterEvent", "(sass)",
                 (LISTENED_EVENT, [], owner_name))
        yield connection
    finally:
        connection.close_sync(None)


def key_log_argv(interpreter: Path, log: Path, opaque: bool) -> list[str]:
    argv = [str(interpreter), str(APP_SCRIPT), str(log)]
    return [*argv, OPAQUE] if opaque else argv


@contextlib.contextmanager
def key_log(tier, label: str, opaque: bool = False):
    """The key log app, registered to the run. Yields the app and its log path.

    With `opaque`, its field reports the entry role with no editable text."""
    require(not apps._owners(tier, names=KEY_LOG_NAMES), "a key log app from an earlier leg still runs")
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    python = shutil.which("python3", path=env.get("PATH"))
    require(python, "no python3 on the session PATH")
    interpreter = private_interpreter(tier.runtime_dir, Path(python).resolve(), KEY_LOG_CLASS)
    log = tier.runtime_dir / f"key-log-{label}.jsonl"
    log.write_text("")
    attached_session.spawn(tier, key_log_argv(interpreter, log, opaque), env,
                           tier.runtime_dir / f"key-log-{label}.log", toplevels=(KEY_LOG_CLASS, 1))
    with raw.bounded_until(time.monotonic() + SETTLE_S):
        owner = raw.poll(lambda: _owner(tier), SETTLE_S)
    require(owner, "the key log app never registered its window")
    owned = _register(tier, owner)
    with contextlib.ExitStack() as stack:
        try:
            stack.enter_context(listener_anchor(tier, owned.owner_names[0]))
        finally:
            stack.callback(_release, tier, owned)
        yield owned, log


def records(log: Path) -> list[dict]:
    return [json.loads(line) for line in log.read_text().splitlines() if line.strip()]


def presses(log: Path) -> list[dict]:
    return [item for item in records(log) if item.get("event") == "press"]


def entry_text(tier, app) -> str | None:
    """The entry's text as its accessible Text interface reports it."""
    found = [item.text() for item in raw.app_root(tier, app.owner_names[0]).find(role=TEXT_ROLE)]
    return found[0] if len(found) == 1 else None


def entry_ref(ctx, app) -> str:
    envelope = ctx.product(["snapshot", "-i", "--window-id", app.windows[0]])
    require(envelope.ok, envelope.raw[:400])
    found = text_ref((envelope.data or {}).get("tree"))
    require(found, "the key log app exposes no entry ref")
    return found


def toplevel(tier, app) -> dict:
    found = clients_of(tier, app.pid)
    require(len(found) == 1, f"the key log app has toplevels {found}")
    return found[0]


def give_focus(tier, address: str) -> None:
    control(tier, lambda lua: focus_argv(address, lua))
    require(raw.poll(lambda: active(tier).get("address") == address, 3.0), f"{address} did not take focus")


def keyboards(tier) -> list[tuple]:
    """Every keyboard the compositor has, with the layout state the product reads."""
    main = main_instance(tier)
    found = hyprland.query(shared(tier), main["signature"], "devices", pid=main["pid"]).get("keyboards", [])
    return sorted((item.get("name"), item.get("main"), item.get("layout"), item.get("active_keymap"),
                   item.get("active_layout_index")) for item in found)


def main_layout_index(tier) -> int | None:
    found = [item for item in keyboards(tier) if item[1]]
    return found[0][4] if len(found) == 1 else None


def step_labels(envelope) -> list[str]:
    return [step.get("label") for step in (envelope.data or {}).get("steps") or [] if isinstance(step, dict)]


def labelled(label: str, envelope, expected: str) -> list[str]:
    if not envelope.ok:
        return [f"{label}: {envelope.raw[:400]}"]
    if expected not in step_labels(envelope):
        return [f"{label}: steps {step_labels(envelope)} do not name {expected}"]
    return []
