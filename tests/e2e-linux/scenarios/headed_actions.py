from runner.legs import implementation
from runner.verdict import LegContext


@implementation("headed_actions.click_button")
def click_button(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.click_button")


@implementation("headed_actions.toggle_switch")
def toggle_switch(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.toggle_switch")


@implementation("headed_actions.delayed_enable")
def delayed_enable(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.delayed_enable")


@implementation("headed_actions.focus_reads_active_window")
def focus_reads_active_window(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.focus_reads_active_window")


@implementation("headed_actions.set_value_commit")
def set_value_commit(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.set_value_commit")


@implementation("headed_actions.select_combo")
def select_combo(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.select_combo")


@implementation("headed_actions.select_combo_modifier_keys")
def select_combo_modifier_keys(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.select_combo_modifier_keys")


@implementation("headed_actions.expand_collapse_row")
def expand_collapse_row(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.expand_collapse_row")


@implementation("headed_actions.type_text")
def type_text(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.type_text")


@implementation("headed_actions.never_focused_window_refusal")
def never_focused_window_refusal(ctx: LegContext) -> None:
    raise NotImplementedError("headed_actions.never_focused_window_refusal")
