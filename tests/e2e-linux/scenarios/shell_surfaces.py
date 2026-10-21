from __future__ import annotations

import subprocess
from pathlib import Path

from runner.apps import launch, launch_qt, stop
from runner.legs import implementation
from runner.verdict import LegContext
from oracle import pixels
from oracle.atspi_raw import app_root, poll

SHELL = "gnome-shell"
MENU_ITEM_ROLE = 35
SURFACES = ("system-tray", "action-center", "quick-settings")


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


def count_role(tree: dict | None, role: str) -> int:
    return sum(1 for node in walk(tree) if node.get("role") == role)


def raw_showing_items(root) -> int:
    return sum(
        1
        for node in root.find(role=MENU_ITEM_ROLE)
        if "SHOWING" in getattr(node, "states", frozenset())
    )


def supported(data: dict | None) -> list[str]:
    return list((data or {}).get("supported_surfaces") or [])


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


def _shell_named(root, needle: str) -> list[str]:
    return [node.name for node in root.find() if needle.lower() in node.name.lower()]


@implementation("shell_surfaces.snapshot_system_tray")
def snapshot_system_tray(ctx: LegContext) -> None:
    root = app_root(ctx.tier, SHELL)
    ctx.before("atspi.shell_nodes", len(root.find()))
    envelope = _snapshot_surface(ctx, "system-tray")
    observed = len(root.find())
    ctx.after("atspi.shell_nodes", observed)
    assert walk((envelope.data or {}).get("tree")), envelope.raw
    assert observed > 0


@implementation("shell_surfaces.snapshot_action_center")
def snapshot_action_center(ctx: LegContext) -> None:
    root = app_root(ctx.tier, SHELL)
    ctx.before("atspi.shell_nodes", len(root.find()))
    envelope = _snapshot_surface(ctx, "action-center")
    ctx.after("atspi.shell_nodes", len(root.find()))
    tree = (envelope.data or {}).get("tree")
    assert walk(tree), envelope.raw
    assert (envelope.data or {}).get("window", {}).get("id"), envelope.raw


@implementation("shell_surfaces.snapshot_quick_settings")
def snapshot_quick_settings(ctx: LegContext) -> None:
    root = app_root(ctx.tier, SHELL)
    ctx.before("atspi.shell_nodes", len(root.find()))
    envelope = _snapshot_surface(ctx, "quick-settings")
    ctx.after("atspi.shell_nodes", len(root.find()))
    assert walk((envelope.data or {}).get("tree")), envelope.raw


@implementation("shell_surfaces.dock_supported_surfaces_agree")
def dock_supported_surfaces_agree(ctx: LegContext) -> None:
    status = ctx.product(["status"])
    assert status.ok, status.raw
    advertised = supported(status.data)
    ctx.before("atspi.shell_nodes", len(app_root(ctx.tier, SHELL).find()))
    for surface in SURFACES:
        envelope = ctx.product(["open-system-surface", "--surface", surface])
        if surface in advertised:
            assert envelope.ok, envelope.raw
        else:
            assert not envelope.ok, envelope.raw
            assert (envelope.error or {}).get("code") == "PLATFORM_NOT_SUPPORTED", envelope.raw
    ctx.after("atspi.shell_nodes", len(app_root(ctx.tier, SHELL).find()))


@implementation("shell_surfaces.system_button_open_close_by_ref")
def system_button_open_close_by_ref(ctx: LegContext) -> None:
    quiet = _capture(ctx.tier)
    ctx.before("pixels.root", "quiet")
    opened = ctx.product(["open-system-surface", "--surface", "quick-settings"])
    assert opened.ok, opened.raw
    listing = _snapshot_surface(ctx, "quick-settings")
    ref = _first_ref((listing.data or {}).get("tree"))
    assert ref, "quick-settings allocated no ref"
    ctx.product(["click", ref], headed=True)
    panel = poll(lambda: _shell_named(app_root(ctx.tier, SHELL), "Do Not Disturb"), 3.0)
    busy = _capture(ctx.tier)
    assert not pixels.similar(quiet, busy), "the panel produced no pixel difference"
    ctx.after("pixels.root", "panel")
    ctx.product(["press", "escape"], headed=True)
    restored = poll(lambda: _capture(ctx.tier) if _closed(ctx.tier) else None, 4.0)
    assert restored is not None, "the panel did not close"
    ctx.restored("pixels.root", "quiet")
    assert pixels.similar(quiet, restored)
    assert panel


def _closed(tier) -> bool:
    return not _shell_named(app_root(tier, SHELL), "Do Not Disturb")


@implementation("shell_surfaces.message_list_closed_after_leg")
def message_list_closed_after_leg(ctx: LegContext) -> None:
    root = app_root(ctx.tier, SHELL)
    ctx.before("atspi.message_list", bool(_shell_named(root, "Notification")))
    opened = ctx.product(["open-system-surface", "--surface", "action-center"])
    assert opened.ok, opened.raw
    assert poll(lambda: _shell_named(app_root(ctx.tier, SHELL), "Notification"), 3.0)
    ctx.product(["press", "escape"], headed=True)
    closed = poll(lambda: not _shell_named(app_root(ctx.tier, SHELL), "Notification"), 4.0)
    ctx.after("atspi.message_list", bool(_shell_named(app_root(ctx.tier, SHELL), "Notification")))
    assert closed


def _menu_counts(ctx: LegContext, app: str, owner: str, opener) -> None:
    if opener is not None:
        launching = opener(ctx.tier)
    else:
        launching = launch(ctx.tier, app)
    root = app_root(ctx.tier, owner)
    trigger = root.find(role=MENU_ITEM_ROLE)
    if not trigger:
        button = root.find(name="Main Menu")
        assert button, f"{app} exposes no menu to open"
        button[0].do_action("click")
    expected = poll(lambda: raw_showing_items(app_root(ctx.tier, owner)), 3.0)
    ctx.before("atspi.showing_items", expected)
    envelope = ctx.product(["snapshot", "--surface", "menu", "--app", app, "-i"])
    assert envelope.ok, envelope.raw
    tree = (envelope.data or {}).get("tree")
    listed = count_role(tree, "menuitem")
    ctx.after("atspi.showing_items", listed)
    assert listed == expected, f"menu surface listed {listed}, raw showing count {expected}"
    stop(launching)


@implementation("shell_surfaces.menu_surface_counts_gtk4")
def menu_surface_counts_gtk4(ctx: LegContext) -> None:
    _menu_counts(ctx, "Calculator", "gnome-calculator", None)


@implementation("shell_surfaces.menu_surface_counts_gtk3")
def menu_surface_counts_gtk3(ctx: LegContext) -> None:
    _menu_counts(ctx, "Terminal", "gnome-terminal-server", None)


@implementation("shell_surfaces.menu_surface_counts_qt")
def menu_surface_counts_qt(ctx: LegContext) -> None:
    _menu_counts(ctx, "Qt window", "qt_window", lambda tier: launch_qt(tier))


def _first_ref(tree: dict | None) -> str | None:
    for node in walk(tree):
        if node.get("ref"):
            return node["ref"]
    return None
