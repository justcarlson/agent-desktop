from runner.legs import implementation
from runner.verdict import LegContext


@implementation("state.undo_enable_delayed_default_timeout")
def undo_enable_delayed_default_timeout(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_enable_delayed_default_timeout")


@implementation("state.undo_enable_delayed_explicit_timeout")
def undo_enable_delayed_explicit_timeout(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_enable_delayed_explicit_timeout")


@implementation("state.undo_enable_delayed_short_timeout_fails")
def undo_enable_delayed_short_timeout_fails(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_enable_delayed_short_timeout_fails")


@implementation("state.undo_dispatches_exactly_once")
def undo_dispatches_exactly_once(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_dispatches_exactly_once")


@implementation("state.undo_disabled_explicit_timeout")
def undo_disabled_explicit_timeout(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_disabled_explicit_timeout")


@implementation("state.undo_disabled_default_timeout")
def undo_disabled_default_timeout(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_disabled_default_timeout")


@implementation("state.undo_zero_timeout_single_check")
def undo_zero_timeout_single_check(ctx: LegContext) -> None:
    raise NotImplementedError("state.undo_zero_timeout_single_check")


@implementation("state.terminal_copy_follows_selection")
def terminal_copy_follows_selection(ctx: LegContext) -> None:
    raise NotImplementedError("state.terminal_copy_follows_selection")


@implementation("state.text_editor_semantic_type")
def text_editor_semantic_type(ctx: LegContext) -> None:
    raise NotImplementedError("state.text_editor_semantic_type")
