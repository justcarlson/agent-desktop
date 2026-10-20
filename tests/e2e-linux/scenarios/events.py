from runner.legs import implementation
from runner.verdict import LegContext


@implementation("events.window_opened_transition")
def window_opened_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.window_opened_transition")


@implementation("events.window_opened_quiet")
def window_opened_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.window_opened_quiet")


@implementation("events.window_closed_transition")
def window_closed_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.window_closed_transition")


@implementation("events.window_closed_quiet")
def window_closed_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.window_closed_quiet")


@implementation("events.app_launched_transition")
def app_launched_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.app_launched_transition")


@implementation("events.app_launched_quiet")
def app_launched_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.app_launched_quiet")


@implementation("events.app_terminated_transition")
def app_terminated_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.app_terminated_transition")


@implementation("events.app_terminated_quiet")
def app_terminated_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.app_terminated_quiet")


@implementation("events.surface_appeared_transition")
def surface_appeared_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.surface_appeared_transition")


@implementation("events.surface_appeared_quiet")
def surface_appeared_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.surface_appeared_quiet")


@implementation("events.surface_dismissed_transition")
def surface_dismissed_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.surface_dismissed_transition")


@implementation("events.surface_dismissed_quiet")
def surface_dismissed_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.surface_dismissed_quiet")


@implementation("events.focus_changed_transition")
def focus_changed_transition(ctx: LegContext) -> None:
    raise NotImplementedError("events.focus_changed_transition")


@implementation("events.focus_changed_quiet")
def focus_changed_quiet(ctx: LegContext) -> None:
    raise NotImplementedError("events.focus_changed_quiet")


@implementation("events.scoped_and_unscoped_forms")
def scoped_and_unscoped_forms(ctx: LegContext) -> None:
    raise NotImplementedError("events.scoped_and_unscoped_forms")


@implementation("events.open_and_close_inside_one_wait")
def open_and_close_inside_one_wait(ctx: LegContext) -> None:
    raise NotImplementedError("events.open_and_close_inside_one_wait")


@implementation("events.target_killed_mid_wait")
def target_killed_mid_wait(ctx: LegContext) -> None:
    raise NotImplementedError("events.target_killed_mid_wait")
