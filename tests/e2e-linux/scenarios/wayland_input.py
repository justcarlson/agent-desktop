from runner.legs import implementation
from runner.verdict import LegContext


@implementation("wayland_input.first_key_lands")
def first_key_lands(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_input.first_key_lands")


@implementation("wayland_input.modifiers_work")
def modifiers_work(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_input.modifiers_work")


@implementation("wayland_input.ctrl_f_two_keycodes")
def ctrl_f_two_keycodes(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_input.ctrl_f_two_keycodes")


@implementation("wayland_input.clipboard_paste_into_app")
def clipboard_paste_into_app(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_input.clipboard_paste_into_app")


@implementation("wayland_input.key_before_throwaway_not_rescued")
def key_before_throwaway_not_rescued(ctx: LegContext) -> None:
    raise NotImplementedError("wayland_input.key_before_throwaway_not_rescued")
