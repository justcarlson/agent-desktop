from runner.legs import implementation
from runner.verdict import LegContext


@implementation("values.hot_corner_flip_restore")
def hot_corner_flip_restore(ctx: LegContext) -> None:
    raise NotImplementedError("values.hot_corner_flip_restore")


@implementation("values.pointer_speed_commit")
def pointer_speed_commit(ctx: LegContext) -> None:
    raise NotImplementedError("values.pointer_speed_commit")


@implementation("values.workspace_count_commit")
def workspace_count_commit(ctx: LegContext) -> None:
    raise NotImplementedError("values.workspace_count_commit")


@implementation("values.terminal_read_only_flip")
def terminal_read_only_flip(ctx: LegContext) -> None:
    raise NotImplementedError("values.terminal_read_only_flip")
