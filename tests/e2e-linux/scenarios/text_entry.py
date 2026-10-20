from runner.legs import implementation
from runner.verdict import LegContext


@implementation("text_entry.typed_text_reaches_document")
def typed_text_reaches_document(ctx: LegContext) -> None:
    raise NotImplementedError("text_entry.typed_text_reaches_document")


@implementation("text_entry.find_bar_ctrl_f")
def find_bar_ctrl_f(ctx: LegContext) -> None:
    raise NotImplementedError("text_entry.find_bar_ctrl_f")


@implementation("text_entry.replace_bar_ctrl_h")
def replace_bar_ctrl_h(ctx: LegContext) -> None:
    raise NotImplementedError("text_entry.replace_bar_ctrl_h")


@implementation("text_entry.replace_all_gating")
def replace_all_gating(ctx: LegContext) -> None:
    raise NotImplementedError("text_entry.replace_all_gating")


@implementation("text_entry.held_key_released")
def held_key_released(ctx: LegContext) -> None:
    raise NotImplementedError("text_entry.held_key_released")


@implementation("text_entry.unfocused_window_refusal")
def unfocused_window_refusal(ctx: LegContext) -> None:
    raise NotImplementedError("text_entry.unfocused_window_refusal")
