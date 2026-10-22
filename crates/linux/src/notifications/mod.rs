use agent_desktop_core::{
    ActionResult, AdapterError, Deadline, DeliverySemantics, DismissAllNotificationsRequest,
    DismissNotificationRequest, ErrorCode, InteractionLease, InteractionPolicy,
    NotificationActionRequest, NotificationFilter, NotificationInfo,
};
use serde_json::json;

use crate::system::SharedBus;

pub(crate) fn check_support(deadline: Deadline) -> Result<(), AdapterError> {
    Err(unavailable(deadline))
}

pub(crate) fn list(
    _bus: &SharedBus,
    _filter: &NotificationFilter,
    _policy: InteractionPolicy,
    deadline: Deadline,
    _lease: Option<&InteractionLease>,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    Err(unavailable(deadline))
}

pub(crate) fn dismiss(
    _bus: &SharedBus,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    Err(mutation_unavailable(request.policy, lease.deadline()))
}

pub(crate) fn dismiss_all(
    _bus: &SharedBus,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<(Vec<NotificationInfo>, Vec<String>), AdapterError> {
    Err(mutation_unavailable(request.policy, lease.deadline()))
}

pub(crate) fn action(
    _bus: &SharedBus,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    Err(mutation_unavailable(request.policy, lease.deadline()))
}

fn mutation_unavailable(policy: InteractionPolicy, deadline: Deadline) -> AdapterError {
    if !policy.is_headed() {
        return AdapterError::policy_denied_for_policy(
            "Changing notifications requires headed mode",
            policy,
        );
    }
    unavailable(deadline)
}

fn unavailable(deadline: Deadline) -> AdapterError {
    if let Err(error) = crate::system::shell::detect::require_daemon(deadline) {
        return error.with_disposition(DeliverySemantics::not_delivered());
    }
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        "The notification backend exposes no provable notification identity or control semantics",
    )
    .with_suggestion(
        "Use the application's accessible controls; the notification protocol has no history query",
    )
    .with_details(json!({
        "kind": "notification_identity_unavailable",
        "retryable": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
mod backend_live_tests;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod refusal_oracle;
