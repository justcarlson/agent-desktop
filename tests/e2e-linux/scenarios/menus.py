from __future__ import annotations

from runner.apps import launch, stop
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root, poll

CALCULATOR = "Calculator"
TERMINAL = "Terminal"


def timeout_message(envelope) -> str:
    error = envelope.error or {}
    return str(error.get("message", ""))


def is_timeout(envelope) -> bool:
    return (not envelope.ok) and (envelope.error or {}).get("code") == "TIMEOUT"


def names_containing(root, needle: str) -> list[str]:
    return [node.name for node in root.find() if needle.lower() in node.name.lower()]


def _click_named(tier, owner: str, name: str) -> bool:
    matches = app_root(tier, owner).find(name=name)
    if not matches:
        return False
    return matches[0].do_action("click")


def _calculator(tier):
    app = launch(tier, CALCULATOR)
    return app, app_root(tier, "gnome-calculator")


def _terminal(tier):
    app = launch(tier, TERMINAL)
    return app, app_root(tier, "gnome-terminal-server")


@implementation("menus.wait_menu_opens_calculator")
def wait_menu_opens_calculator(ctx: LegContext) -> None:
    app, root = _calculator(ctx.tier)
    ctx.before("atspi.menu_visible", False)
    assert _click_named(ctx.tier, "gnome-calculator", "Main Menu")
    envelope = ctx.product(["wait", "--menu", "--app", CALCULATOR, "--timeout", "4000"])
    assert envelope.ok, envelope.raw
    assert envelope.data and envelope.data.get("found") is True, envelope.raw
    ctx.after("atspi.menu_visible", True)
    stop(app)


@implementation("menus.wait_menu_closes_calculator")
def wait_menu_closes_calculator(ctx: LegContext) -> None:
    app, _root = _calculator(ctx.tier)
    assert _click_named(ctx.tier, "gnome-calculator", "Main Menu")
    ctx.before("atspi.menu_visible", True)
    assert _click_named(ctx.tier, "gnome-calculator", "Main Menu")
    envelope = ctx.product(
        ["wait", "--menu-closed", "--app", CALCULATOR, "--timeout", "4000"]
    )
    assert envelope.ok, envelope.raw
    ctx.after("atspi.menu_visible", False)
    stop(app)


@implementation("menus.wait_menu_opens_terminal")
def wait_menu_opens_terminal(ctx: LegContext) -> None:
    app, _root = _terminal(ctx.tier)
    ctx.before("atspi.menu_visible", False)
    assert _click_named(ctx.tier, "gnome-terminal-server", "Main Menu") or _click_named(
        ctx.tier, "gnome-terminal-server", "Menu"
    )
    envelope = ctx.product(["wait", "--menu", "--app", TERMINAL, "--timeout", "4000"])
    assert envelope.ok, envelope.raw
    ctx.after("atspi.menu_visible", True)
    stop(app)


@implementation("menus.wait_menu_already_open")
def wait_menu_already_open(ctx: LegContext) -> None:
    app, _root = _calculator(ctx.tier)
    assert _click_named(ctx.tier, "gnome-calculator", "Main Menu")
    ctx.before("atspi.menu_visible", True)
    envelope = ctx.product(["wait", "--menu", "--app", CALCULATOR, "--timeout", "4000"])
    assert envelope.ok, envelope.raw
    assert envelope.elapsed_ms < 1500, f"already-open menu took {envelope.elapsed_ms} ms"
    ctx.after("atspi.menu_visible", True)
    stop(app)


@implementation("menus.hidden_bar_timeout_message")
def hidden_bar_timeout_message(ctx: LegContext) -> None:
    app, root = _terminal(ctx.tier)
    ctx.before("atspi.menu_visible", False)
    envelope = ctx.product(["wait", "--menu", "--app", TERMINAL, "--timeout", "1500"])
    assert is_timeout(envelope), envelope.raw
    message = timeout_message(envelope).lower()
    assert "menu" in message or "hidden" in message, envelope.raw
    ctx.after("atspi.menu_visible", False)
    stop(app)


@implementation("menus.terminal_preferences_modal")
def terminal_preferences_modal(ctx: LegContext) -> None:
    app, root = _terminal(ctx.tier)
    before = len(root.children())
    ctx.before("atspi.root_children", before)
    assert _click_named(ctx.tier, "gnome-terminal-server", "Main Menu") or _click_named(
        ctx.tier, "gnome-terminal-server", "Menu"
    )
    entry = poll(
        lambda: app_root(ctx.tier, "gnome-terminal-server").find(name="Preferences"),
        4.0,
        floor_s=0.6,
    )
    assert entry, "Preferences did not appear in the Terminal menu"
    entry[0].do_action("click")
    dialog = poll(
        lambda: names_containing(app_root(ctx.tier, "gnome-terminal-server"), "Preferences"),
        4.0,
    )
    ctx.after("atspi.root_children", len(app_root(ctx.tier, "gnome-terminal-server").children()))
    assert dialog
    stop(app)


@implementation("menus.settings_panel_navigation")
def settings_panel_navigation(ctx: LegContext) -> None:
    before = names_containing(app_root(ctx.tier, "gnome-control-center"), "Power") if _registered(ctx.tier) else []
    ctx.before("atspi.panel", before)
    app = launch(ctx.tier, "Settings", ("power",))
    observed = poll(
        lambda: names_containing(app_root(ctx.tier, "gnome-control-center"), "Power"),
        8.0,
    )
    ctx.after("atspi.panel", observed)
    assert observed
    stop(app)


def _registered(tier) -> bool:
    try:
        app_root(tier, "gnome-control-center")
    except Exception:
        return False
    return True
