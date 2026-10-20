from runner.legs import implementation
from runner.verdict import LegContext


@implementation("notifications.list_equals_sender_record")
def list_equals_sender_record(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.list_equals_sender_record")


@implementation("notifications.app_filter_case_insensitive")
def app_filter_case_insensitive(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.app_filter_case_insensitive")


@implementation("notifications.wait_notification_passes")
def wait_notification_passes(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.wait_notification_passes")


@implementation("notifications.wait_notification_times_out")
def wait_notification_times_out(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.wait_notification_times_out")


@implementation("notifications.headless_mutations_policy_denied")
def headless_mutations_policy_denied(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.headless_mutations_policy_denied")


@implementation("notifications.wrong_app_not_found")
def wrong_app_not_found(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.wrong_app_not_found")


@implementation("notifications.empty_list_not_found")
def empty_list_not_found(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.empty_list_not_found")


@implementation("notifications.dismiss_one_reason_closed")
def dismiss_one_reason_closed(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.dismiss_one_reason_closed")


@implementation("notifications.dismiss_keeps_bystander")
def dismiss_keeps_bystander(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.dismiss_keeps_bystander")


@implementation("notifications.dismiss_all_closes_every_notification")
def dismiss_all_closes_every_notification(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.dismiss_all_closes_every_notification")


@implementation("notifications.action_invoked_signal")
def action_invoked_signal(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.action_invoked_signal")


@implementation("notifications.delivered_mutation_internal_fails")
def delivered_mutation_internal_fails(ctx: LegContext) -> None:
    raise NotImplementedError("notifications.delivered_mutation_internal_fails")
