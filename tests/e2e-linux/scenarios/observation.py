from runner.legs import implementation
from runner.verdict import LegContext


@implementation("observation.qualified_refs_survive_snapshots")
def qualified_refs_survive_snapshots(ctx: LegContext) -> None:
    raise NotImplementedError("observation.qualified_refs_survive_snapshots")


@implementation("observation.stale_ref_after_app_killed")
def stale_ref_after_app_killed(ctx: LegContext) -> None:
    raise NotImplementedError("observation.stale_ref_after_app_killed")


@implementation("observation.selector_wait_resolves")
def selector_wait_resolves(ctx: LegContext) -> None:
    raise NotImplementedError("observation.selector_wait_resolves")


@implementation("observation.enabled_state_canonical_and_raw")
def enabled_state_canonical_and_raw(ctx: LegContext) -> None:
    raise NotImplementedError("observation.enabled_state_canonical_and_raw")


@implementation("observation.drilldown_root_replaces_subtree")
def drilldown_root_replaces_subtree(ctx: LegContext) -> None:
    raise NotImplementedError("observation.drilldown_root_replaces_subtree")


@implementation("observation.session_namespace_isolation")
def session_namespace_isolation(ctx: LegContext) -> None:
    raise NotImplementedError("observation.session_namespace_isolation")


@implementation("observation.trace_segment_written")
def trace_segment_written(ctx: LegContext) -> None:
    raise NotImplementedError("observation.trace_segment_written")


@implementation("observation.unreachable_bus_permission_error")
def unreachable_bus_permission_error(ctx: LegContext) -> None:
    raise NotImplementedError("observation.unreachable_bus_permission_error")
