from __future__ import annotations

import subprocess
import threading
from contextlib import contextmanager
from pathlib import Path

from apps.sni_item import WATCHER_PATH, WATCHER_SERVICE
from runner.legs import implementation
from runner.verdict import LegContext
from oracle import pixels
from oracle.atspi_raw import app_root, call, connect, poll

SHELL = "gnome-shell"


def walk(tree: dict | None) -> list[dict]:
    if not tree:
        return []
    found: list[dict] = []
    stack = [tree]
    while stack:
        node = stack.pop()
        found.append(node)
        stack.extend(node.get("children") or [])
    return found


def _capture(tier):
    target = Path(tier.runtime_dir) / "root-capture.xwd"
    subprocess.run(
        ["xwd", "-root", "-silent", "-out", str(target)],
        env=tier.env,
        check=True,
        timeout=20,
    )
    return pixels.decode_xwd(target.read_bytes())


def _snapshot_surface(ctx: LegContext, surface: str):
    envelope = ctx.product(["snapshot", "--surface", surface, "-i"])
    assert envelope.ok, envelope.raw
    return envelope


def _anchor(tier, name):
    nodes = []
    pending = [app_root(tier, SHELL)]
    while pending:
        node = pending.pop()
        label = node.name
        if label == "Overview":
            continue
        if label == name:
            nodes.append(node)
        pending.extend(node.children())
    assert len(nodes) == 1, f"expected one shell landmark {name!r}, got {len(nodes)}"
    return nodes[0]


@contextmanager
def _empty_watcher(tier):
    from gi.repository import Gio, GLib

    connection = connect(tier.bus_address)
    xml = (f'<node><interface name="{WATCHER_SERVICE}">'
           '<property name="RegisteredStatusNotifierItems" type="as" access="read"/>'
           '</interface></node>')
    interface = Gio.DBusNodeInfo.new_for_xml(xml).interfaces[0]
    registration = connection.register_object(
        WATCHER_PATH, interface, None, lambda *_: GLib.Variant("as", []), None,
    )
    loop = GLib.MainLoop()
    thread = threading.Thread(target=loop.run, daemon=True)
    thread.start()
    try:
        owned = call(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus",
                     "org.freedesktop.DBus", "RequestName", "(su)", (WATCHER_SERVICE, 4))
        assert owned == (1,), "the private session already has a tray watcher"
        assert call(connection, WATCHER_SERVICE, WATCHER_PATH,
                    "org.freedesktop.DBus.Properties", "Get", "(ss)",
                    (WATCHER_SERVICE, "RegisteredStatusNotifierItems")) == ([],)
        yield
    finally:
        connection.unregister_object(registration)
        loop.quit()
        thread.join(2)
        connection.close_sync(None)
        assert not thread.is_alive(), "empty watcher thread survived"


@implementation("shell_surfaces.system_button_open_close_by_ref")
def system_button_open_close_by_ref(ctx: LegContext) -> None:
    with _empty_watcher(ctx.tier):
        popup = _anchor(ctx.tier, "Power Off Menu")
        def visible():
            return "SHOWING" in popup.states

        assert not visible(), "system menu initially open"
        quiet = _capture(ctx.tier)
        ctx.before("atspi.system_menu", visible())
        listing = _snapshot_surface(ctx, "system-tray")
        refs = [node["ref_id"] for node in walk((listing.data or {}).get("tree"))
                if node.get("name") == "System" and node.get("role") == "menu"
                and node.get("ref_id")]
        assert len(refs) == 1, listing.raw
        clicked = ctx.product(["click", refs[0]], headed=True)
        assert clicked.ok, clicked.raw
        assert poll(visible, 4.0), "system menu did not open"
        busy = _capture(ctx.tier)
        assert not pixels.similar(quiet, busy), "the panel produced no pixel difference"
        ctx.after("atspi.system_menu", visible())
        clicked = ctx.product(["click", refs[0]], headed=True)
        assert clicked.ok, clicked.raw
        assert poll(lambda: not visible(), 4.0), "system menu did not close"
        assert poll(lambda: pixels.similar(quiet, _capture(ctx.tier)), 4.0), "pixels did not restore"
        ctx.restored("atspi.system_menu", visible())
