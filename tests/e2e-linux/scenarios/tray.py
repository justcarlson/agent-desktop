from runner.legs import implementation
from runner.verdict import LegContext


@implementation("tray.sender_registered_before_first_leg")
def sender_registered_before_first_leg(ctx: LegContext) -> None:
    raise NotImplementedError("tray.sender_registered_before_first_leg")


@implementation("tray.headless_activate_logged")
def headless_activate_logged(ctx: LegContext) -> None:
    raise NotImplementedError("tray.headless_activate_logged")


@implementation("tray.headless_menu_event_clicked")
def headless_menu_event_clicked(ctx: LegContext) -> None:
    raise NotImplementedError("tray.headless_menu_event_clicked")


@implementation("tray.disabled_entry_refused")
def disabled_entry_refused(ctx: LegContext) -> None:
    raise NotImplementedError("tray.disabled_entry_refused")


@implementation("tray.item_without_activate_refused")
def item_without_activate_refused(ctx: LegContext) -> None:
    raise NotImplementedError("tray.item_without_activate_refused")


@implementation("tray.killed_owner_stale_ref")
def killed_owner_stale_ref(ctx: LegContext) -> None:
    raise NotImplementedError("tray.killed_owner_stale_ref")


@implementation("tray.headed_click_opens_shell_popup")
def headed_click_opens_shell_popup(ctx: LegContext) -> None:
    raise NotImplementedError("tray.headed_click_opens_shell_popup")
