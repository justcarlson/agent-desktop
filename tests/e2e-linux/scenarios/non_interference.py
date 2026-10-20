from runner.legs import implementation
from runner.verdict import LegContext


@implementation("non_interference.pointer_unchanged_by_semantic_click")
def pointer_unchanged_by_semantic_click(ctx: LegContext) -> None:
    raise NotImplementedError("non_interference.pointer_unchanged_by_semantic_click")


@implementation("non_interference.active_window_unchanged_by_semantic_click")
def active_window_unchanged_by_semantic_click(ctx: LegContext) -> None:
    raise NotImplementedError("non_interference.active_window_unchanged_by_semantic_click")


@implementation("non_interference.pointer_unchanged_by_value_write")
def pointer_unchanged_by_value_write(ctx: LegContext) -> None:
    raise NotImplementedError("non_interference.pointer_unchanged_by_value_write")
