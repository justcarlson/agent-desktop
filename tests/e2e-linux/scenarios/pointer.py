from runner.legs import implementation
from runner.verdict import LegContext


@implementation("pointer.click")
def click(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.click")


@implementation("pointer.double_click")
def double_click(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.double_click")


@implementation("pointer.triple_click")
def triple_click(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.triple_click")


@implementation("pointer.right_click")
def right_click(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.right_click")


@implementation("pointer.hover")
def hover(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.hover")


@implementation("pointer.drag")
def drag(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.drag")


@implementation("pointer.mouse_move")
def mouse_move(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.mouse_move")


@implementation("pointer.mouse_click")
def mouse_click(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.mouse_click")


@implementation("pointer.mouse_wheel")
def mouse_wheel(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.mouse_wheel")


@implementation("pointer.scroll")
def scroll(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.scroll")


@implementation("pointer.gtk3_extents_origin")
def gtk3_extents_origin(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.gtk3_extents_origin")


@implementation("pointer.gtk4_two_positions")
def gtk4_two_positions(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.gtk4_two_positions")


@implementation("pointer.qt_extents_origin")
def qt_extents_origin(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.qt_extents_origin")


@implementation("pointer.drag_delivery_report")
def drag_delivery_report(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.drag_delivery_report")


@implementation("pointer.mouse_wheel_delivery_report")
def mouse_wheel_delivery_report(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.mouse_wheel_delivery_report")


@implementation("pointer.mouse_click_delivery_report")
def mouse_click_delivery_report(ctx: LegContext) -> None:
    raise NotImplementedError("pointer.mouse_click_delivery_report")
