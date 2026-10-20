from runner.legs import implementation
from runner.verdict import LegContext


@implementation("launch.launch_calculator_attributed")
def launch_calculator_attributed(ctx: LegContext) -> None:
    raise NotImplementedError("launch.launch_calculator_attributed")


@implementation("launch.launch_text_editor_new_window")
def launch_text_editor_new_window(ctx: LegContext) -> None:
    raise NotImplementedError("launch.launch_text_editor_new_window")


@implementation("launch.launch_terminal_owner")
def launch_terminal_owner(ctx: LegContext) -> None:
    raise NotImplementedError("launch.launch_terminal_owner")


@implementation("launch.list_apps_alias")
def list_apps_alias(ctx: LegContext) -> None:
    raise NotImplementedError("launch.list_apps_alias")


@implementation("launch.launch_suggestion")
def launch_suggestion(ctx: LegContext) -> None:
    raise NotImplementedError("launch.launch_suggestion")


@implementation("launch.recovery_strategy_on_error")
def recovery_strategy_on_error(ctx: LegContext) -> None:
    raise NotImplementedError("launch.recovery_strategy_on_error")
