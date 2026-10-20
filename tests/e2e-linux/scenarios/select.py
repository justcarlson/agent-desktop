from runner.legs import implementation
from runner.verdict import LegContext


@implementation("select.screen_blank_value")
def screen_blank_value(ctx: LegContext) -> None:
    raise NotImplementedError("select.screen_blank_value")


@implementation("select.refusal_without_semantic_route")
def refusal_without_semantic_route(ctx: LegContext) -> None:
    raise NotImplementedError("select.refusal_without_semantic_route")
