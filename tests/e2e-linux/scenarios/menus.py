from runner.legs import implementation
from runner.verdict import LegContext


@implementation("menus.wait_menu_opens_calculator")
def wait_menu_opens_calculator(ctx: LegContext) -> None:
    raise NotImplementedError("menus.wait_menu_opens_calculator")


@implementation("menus.wait_menu_closes_calculator")
def wait_menu_closes_calculator(ctx: LegContext) -> None:
    raise NotImplementedError("menus.wait_menu_closes_calculator")


@implementation("menus.wait_menu_opens_terminal")
def wait_menu_opens_terminal(ctx: LegContext) -> None:
    raise NotImplementedError("menus.wait_menu_opens_terminal")


@implementation("menus.wait_menu_already_open")
def wait_menu_already_open(ctx: LegContext) -> None:
    raise NotImplementedError("menus.wait_menu_already_open")


@implementation("menus.hidden_bar_timeout_message")
def hidden_bar_timeout_message(ctx: LegContext) -> None:
    raise NotImplementedError("menus.hidden_bar_timeout_message")


@implementation("menus.terminal_preferences_modal")
def terminal_preferences_modal(ctx: LegContext) -> None:
    raise NotImplementedError("menus.terminal_preferences_modal")


@implementation("menus.settings_panel_navigation")
def settings_panel_navigation(ctx: LegContext) -> None:
    raise NotImplementedError("menus.settings_panel_navigation")
