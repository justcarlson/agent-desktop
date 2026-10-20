from runner.legs import implementation
from runner.verdict import LegContext


@implementation("clipboard.text_roundtrip")
def text_roundtrip(ctx: LegContext) -> None:
    raise NotImplementedError("clipboard.text_roundtrip")


@implementation("clipboard.html_roundtrip")
def html_roundtrip(ctx: LegContext) -> None:
    raise NotImplementedError("clipboard.html_roundtrip")


@implementation("clipboard.image_roundtrip")
def image_roundtrip(ctx: LegContext) -> None:
    raise NotImplementedError("clipboard.image_roundtrip")


@implementation("clipboard.owner_outlives_setter")
def owner_outlives_setter(ctx: LegContext) -> None:
    raise NotImplementedError("clipboard.owner_outlives_setter")


@implementation("clipboard.clear_leaves_empty")
def clear_leaves_empty(ctx: LegContext) -> None:
    raise NotImplementedError("clipboard.clear_leaves_empty")


@implementation("clipboard.foreign_read_after_cli_exit")
def foreign_read_after_cli_exit(ctx: LegContext) -> None:
    raise NotImplementedError("clipboard.foreign_read_after_cli_exit")
