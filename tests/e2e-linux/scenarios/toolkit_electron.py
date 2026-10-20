from runner.legs import implementation
from runner.verdict import LegContext


@implementation("toolkit_electron.frame_stays_registered")
def frame_stays_registered(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_electron.frame_stays_registered")


@implementation("toolkit_electron.refs_and_completeness")
def refs_and_completeness(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_electron.refs_and_completeness")


@implementation("toolkit_electron.menu_bar_by_keyboard")
def menu_bar_by_keyboard(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_electron.menu_bar_by_keyboard")


@implementation("toolkit_electron.wait_menu_both_directions")
def wait_menu_both_directions(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_electron.wait_menu_both_directions")
