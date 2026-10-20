from runner.legs import implementation
from runner.verdict import LegContext


@implementation("duplicates.two_processes_distinct_window_ids")
def two_processes_distinct_window_ids(ctx: LegContext) -> None:
    raise NotImplementedError("duplicates.two_processes_distinct_window_ids")


@implementation("duplicates.main_menu_nodes_separate")
def main_menu_nodes_separate(ctx: LegContext) -> None:
    raise NotImplementedError("duplicates.main_menu_nodes_separate")


@implementation("duplicates.new_window_distinct_id")
def new_window_distinct_id(ctx: LegContext) -> None:
    raise NotImplementedError("duplicates.new_window_distinct_id")
