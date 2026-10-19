use agent_desktop_core::{
    ActionResult, AdapterError, Deadline, DismissAllNotificationsRequest,
    DismissNotificationRequest, InteractionLease, InteractionPolicy, NotificationActionRequest,
    NotificationFilter, NotificationInfo,
};

use crate::system::SharedBus;

mod action;
mod dismiss;
mod dismiss_all;
mod extract;
mod landmark;
mod list;
mod session;
mod signals;
mod verify;
mod world;

#[cfg(test)]
mod live_tests;

pub(crate) fn list(
    _bus: &SharedBus,
    _filter: &NotificationFilter,
    _policy: InteractionPolicy,
    _deadline: Deadline,
    _lease: Option<&InteractionLease>,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    Err(AdapterError::not_supported("list_notifications"))
}

pub(crate) fn dismiss(
    _bus: &SharedBus,
    _request: DismissNotificationRequest<'_>,
    _lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    Err(AdapterError::not_supported("dismiss_notification"))
}

pub(crate) fn dismiss_all(
    _bus: &SharedBus,
    _request: DismissAllNotificationsRequest<'_>,
    _lease: &InteractionLease,
) -> Result<(Vec<NotificationInfo>, Vec<String>), AdapterError> {
    Err(AdapterError::not_supported("dismiss_all_notifications"))
}

pub(crate) fn action(
    _bus: &SharedBus,
    _request: NotificationActionRequest<'_>,
    _lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("notification_action"))
}
