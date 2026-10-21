from __future__ import annotations

import subprocess
from pathlib import Path

from runner.legs import implementation
from runner.verdict import LegContext
from oracle import pixels
from oracle.atspi_raw import app_root, poll
from scenarios.tray import enabled_watcher

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


def _visible(tier):
    node = _anchor(tier, "Power Off Menu")
    return "SHOWING" in node.states


@implementation("shell_surfaces.system_button_open_close_by_ref")
def system_button_open_close_by_ref(ctx: LegContext) -> None:
    with enabled_watcher(ctx):
        assert not _visible(ctx.tier), "system menu initially open"
        quiet = _capture(ctx.tier)
        ctx.before("atspi.system_menu", _visible(ctx.tier))
        listing = _snapshot_surface(ctx, "system-tray")
        refs = [node["ref_id"] for node in walk((listing.data or {}).get("tree"))
                if node.get("name") == "System" and node.get("role") == "menu"
                and node.get("ref_id")]
        assert len(refs) == 1, listing.raw
        clicked = ctx.product(["click", refs[0]], headed=True)
        assert clicked.ok, clicked.raw
        assert poll(lambda: _visible(ctx.tier), 4.0), "system menu did not open"
        busy = _capture(ctx.tier)
        assert not pixels.similar(quiet, busy), "the panel produced no pixel difference"
        ctx.after("atspi.system_menu", _visible(ctx.tier))
        clicked = ctx.product(["click", refs[0]], headed=True)
        assert clicked.ok, clicked.raw
        assert poll(lambda: not _visible(ctx.tier), 4.0), "system menu did not close"
        assert poll(lambda: pixels.similar(quiet, _capture(ctx.tier)), 4.0), "pixels did not restore"
        ctx.restored("atspi.system_menu", _visible(ctx.tier))
