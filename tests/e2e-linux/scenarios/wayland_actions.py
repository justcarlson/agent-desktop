from runner.legs import implementation
from runner.verdict import LegContext


@implementation("wayland_actions.semantic_action_no_input_device")
def semantic_action_no_input_device(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_actions.semantic_action_no_input_device")


@implementation("wayland_actions.seat_without_keyboard_reported_as_seat")
def seat_without_keyboard_reported_as_seat(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_actions.seat_without_keyboard_reported_as_seat")


@implementation("wayland_actions.hit_test_unknown")
def hit_test_unknown(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_actions.hit_test_unknown")


@implementation("wayland_actions.focus_window_wayland_session_refusal")
def focus_window_wayland_session_refusal(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_actions.focus_window_wayland_session_refusal")


@implementation("wayland_actions.list_windows_no_position")
def list_windows_no_position(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_actions.list_windows_no_position")


@implementation("wayland_actions.unhealthy_shell_named_reason")
def unhealthy_shell_named_reason(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_actions.unhealthy_shell_named_reason")
