from runner.legs import implementation
from runner.verdict import LegContext


@implementation("lifecycle.close_app_delivery_report")
def close_app_delivery_report(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.close_app_delivery_report")


@implementation("lifecycle.close_app_undelivered_report")
def close_app_undelivered_report(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.close_app_undelivered_report")


@implementation("lifecycle.focus_window_active_reads")
def focus_window_active_reads(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.focus_window_active_reads")


@implementation("lifecycle.resize_window")
def resize_window(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.resize_window")


@implementation("lifecycle.move_window")
def move_window(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.move_window")


@implementation("lifecycle.minimize_restore")
def minimize_restore(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.minimize_restore")


@implementation("lifecycle.maximize_restore")
def maximize_restore(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.maximize_restore")


@implementation("lifecycle.window_operation_delivery_reports")
def window_operation_delivery_reports(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.window_operation_delivery_reports")


@implementation("lifecycle.mutter_lift_outcome_set")
def mutter_lift_outcome_set(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.mutter_lift_outcome_set")


@implementation("lifecycle.mutter_desktop_switch_read_back")
def mutter_desktop_switch_read_back(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.mutter_desktop_switch_read_back")


@implementation("lifecycle.mutter_activation_refusal_codes")
def mutter_activation_refusal_codes(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.mutter_activation_refusal_codes")


@implementation("lifecycle.openbox_lift_restored_and_active")
def openbox_lift_restored_and_active(ctx: LegContext) -> None:
    raise NotImplementedError("lifecycle.openbox_lift_restored_and_active")
