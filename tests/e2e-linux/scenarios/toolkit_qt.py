from runner.legs import implementation
from runner.verdict import LegContext


@implementation("toolkit_qt.refs_and_completeness")
def refs_and_completeness(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.refs_and_completeness")


@implementation("toolkit_qt.headless_click_logged_once")
def headless_click_logged_once(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.headless_click_logged_once")


@implementation("toolkit_qt.scroll_refused_no_container")
def scroll_refused_no_container(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.scroll_refused_no_container")


@implementation("toolkit_qt.wait_menu_opens")
def wait_menu_opens(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.wait_menu_opens")


@implementation("toolkit_qt.wait_menu_closes")
def wait_menu_closes(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.wait_menu_closes")


@implementation("toolkit_qt.menu_already_open")
def menu_already_open(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.menu_already_open")


@implementation("toolkit_qt.headed_click_logged_once")
def headed_click_logged_once(ctx: LegContext) -> None:
    raise NotImplementedError("toolkit_qt.headed_click_logged_once")
