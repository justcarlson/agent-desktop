from runner.legs import implementation
from runner.verdict import LegContext


@implementation("shell_surfaces.snapshot_system_tray")
def snapshot_system_tray(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.snapshot_system_tray")


@implementation("shell_surfaces.snapshot_action_center")
def snapshot_action_center(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.snapshot_action_center")


@implementation("shell_surfaces.snapshot_quick_settings")
def snapshot_quick_settings(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.snapshot_quick_settings")


@implementation("shell_surfaces.dock_supported_surfaces_agree")
def dock_supported_surfaces_agree(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.dock_supported_surfaces_agree")


@implementation("shell_surfaces.system_button_open_close_by_ref")
def system_button_open_close_by_ref(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.system_button_open_close_by_ref")


@implementation("shell_surfaces.message_list_closed_after_leg")
def message_list_closed_after_leg(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.message_list_closed_after_leg")


@implementation("shell_surfaces.menu_surface_counts_gtk4")
def menu_surface_counts_gtk4(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.menu_surface_counts_gtk4")


@implementation("shell_surfaces.menu_surface_counts_gtk3")
def menu_surface_counts_gtk3(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.menu_surface_counts_gtk3")


@implementation("shell_surfaces.menu_surface_counts_qt")
def menu_surface_counts_qt(ctx: LegContext) -> None:
    raise NotImplementedError("shell_surfaces.menu_surface_counts_qt")
