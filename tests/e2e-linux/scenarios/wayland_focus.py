from runner.legs import implementation
from runner.verdict import LegContext


@implementation("wayland_focus.seat_keyboard_makes_active")
def seat_keyboard_makes_active(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_focus.seat_keyboard_makes_active")


@implementation("wayland_focus.later_mapped_window_takes_focus")
def later_mapped_window_takes_focus(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_focus.later_mapped_window_takes_focus")


@implementation("wayland_focus.focus_changed_times_out_with_baseline")
def focus_changed_times_out_with_baseline(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_focus.focus_changed_times_out_with_baseline")


@implementation("wayland_focus.headed_ref_action_refusal")
def headed_ref_action_refusal(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_focus.headed_ref_action_refusal")
