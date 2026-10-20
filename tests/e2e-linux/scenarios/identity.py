from runner.legs import implementation
from runner.verdict import LegContext


@implementation("identity.restart_old_id_window_not_found")
def restart_old_id_window_not_found(ctx: LegContext) -> None:
    raise NotImplementedError("identity.restart_old_id_window_not_found")


@implementation("identity.restart_new_id_resolves")
def restart_new_id_resolves(ctx: LegContext) -> None:
    raise NotImplementedError("identity.restart_new_id_resolves")


@implementation("identity.id_survives_unrelated_launch")
def id_survives_unrelated_launch(ctx: LegContext) -> None:
    raise NotImplementedError("identity.id_survives_unrelated_launch")
