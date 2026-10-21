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
    toggles = [node for node in matches if node.role == 62]
    if toggles:
        assert len(toggles) == 1, "ambiguous menu toggle"
        return toggles[0].do_action("click")
    if not matches:
        return False
    return matches[0].do_action("click")


def _visible(tier, owner):
    return any("SHOWING" in node.states for node in app_root(tier, owner).find(role=35))


def _toggle(tier, owner):
    assert _click_named(tier, owner, "Main Menu") or _click_named(tier, owner, "Menu"), "menu opener missing"


def _calculator(tier):
    app = launch(tier, CALCULATOR)
    return app, app_root(tier, "gnome-calculator")


def _terminal(tier):
    app = launch(tier, TERMINAL)
    return app, app_root(tier, "gnome-terminal-server")


def _menu_case(ctx, name, operation):
    from runner.helper import changed
    owner = "gnome-calculator" if name == CALCULATOR else "gnome-terminal-server"
    app = launch(ctx.tier, name)
    read = lambda: _visible(ctx.tier, owner)
    try:
        if operation in ("close", "already"):
            _toggle(ctx.tier, owner)
            changed(read, True)
        before = read()
        assert before == (operation in ("close", "already")), "unexpected initial menu visibility"
        ctx.before("atspi.menu_visible", before)
        if operation in ("open", "close"):
            _toggle(ctx.tier, owner)
        args = ["wait", "--menu-closed" if operation == "close" else "--menu", "--app", name,
                "--timeout", "1500" if operation == "hidden" else "4000"]
        envelope = ctx.product(args)
        if operation == "hidden":
            assert is_timeout(envelope), envelope.raw
            message = timeout_message(envelope).lower()
            assert "menu" in message or "hidden" in message, envelope.raw
        else:
            assert envelope.ok, envelope.raw
            if operation == "already":
                assert envelope.elapsed_ms < 1500, f"already-open menu took {envelope.elapsed_ms} ms"
        expected = operation in ("open", "already")
        ctx.after("atspi.menu_visible", changed(read, expected))
        if read() != before:
            _toggle(ctx.tier, owner)
        ctx.restored("atspi.menu_visible", changed(read, before))
    finally:
        stop(app)


@implementation("menus.wait_menu_opens_calculator")
def wait_menu_opens_calculator(ctx: LegContext) -> None:
    _menu_case(ctx, CALCULATOR, "open")


@implementation("menus.wait_menu_closes_calculator")
def wait_menu_closes_calculator(ctx: LegContext) -> None:
    _menu_case(ctx, CALCULATOR, "close")


def wait_menu_opens_terminal(ctx: LegContext) -> None:
    _menu_case(ctx, TERMINAL, "open")


@implementation("menus.wait_menu_already_open")
def wait_menu_already_open(ctx: LegContext) -> None:
    _menu_case(ctx, CALCULATOR, "already")


@implementation("menus.hidden_bar_timeout_message")
def hidden_bar_timeout_message(ctx: LegContext) -> None:
    _menu_case(ctx, TERMINAL, "hidden")


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
