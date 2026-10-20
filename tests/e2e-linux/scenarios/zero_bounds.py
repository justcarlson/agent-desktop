from runner.legs import implementation
from runner.verdict import LegContext


@implementation("zero_bounds.files_container_raw_only")
def files_container_raw_only(ctx: LegContext) -> None:
    raise NotImplementedError("zero_bounds.files_container_raw_only")


@implementation("zero_bounds.settings_container_raw_only")
def settings_container_raw_only(ctx: LegContext) -> None:
    raise NotImplementedError("zero_bounds.settings_container_raw_only")


@implementation("zero_bounds.history_listbox_outcome")
def history_listbox_outcome(ctx: LegContext) -> None:
    raise NotImplementedError("zero_bounds.history_listbox_outcome")


@implementation("zero_bounds.hidden_menu_items_not_visible")
def hidden_menu_items_not_visible(ctx: LegContext) -> None:
    raise NotImplementedError("zero_bounds.hidden_menu_items_not_visible")
