from runner.legs import implementation
from runner.verdict import LegContext


@implementation("capture.window_pixels_match_xwd")
def window_pixels_match_xwd(ctx: LegContext) -> None:
    raise NotImplementedError("capture.window_pixels_match_xwd")


@implementation("capture.stable_at_idle")
def stable_at_idle(ctx: LegContext) -> None:
    raise NotImplementedError("capture.stable_at_idle")


@implementation("capture.content_change_moves_region")
def content_change_moves_region(ctx: LegContext) -> None:
    raise NotImplementedError("capture.content_change_moves_region")


@implementation("capture.covered_window_capture")
def covered_window_capture(ctx: LegContext) -> None:
    raise NotImplementedError("capture.covered_window_capture")


@implementation("capture.minimised_window_capture")
def minimised_window_capture(ctx: LegContext) -> None:
    raise NotImplementedError("capture.minimised_window_capture")


@implementation("capture.display_capture")
def display_capture(ctx: LegContext) -> None:
    raise NotImplementedError("capture.display_capture")


@implementation("capture.wayland_route_gated")
def wayland_route_gated(ctx: LegContext) -> None:
    raise NotImplementedError("capture.wayland_route_gated")
