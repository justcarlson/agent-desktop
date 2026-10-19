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
mod target;
mod verify;
mod world;

#[cfg(test)]
mod fake;

#[cfg(test)]
mod live_tests;

pub(crate) fn list(
    bus: &SharedBus,
    filter: &NotificationFilter,
    policy: InteractionPolicy,
    deadline: Deadline,
    lease: Option<&InteractionLease>,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    list::list(bus, filter, policy, deadline, lease)
}

pub(crate) fn dismiss(
    bus: &SharedBus,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    dismiss::dismiss(bus, request, lease)
}

pub(crate) fn dismiss_all(
    bus: &SharedBus,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<(Vec<NotificationInfo>, Vec<String>), AdapterError> {
    dismiss_all::dismiss_all(bus, request, lease)
}

pub(crate) fn action(
    bus: &SharedBus,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    action::action(bus, request, lease)
}
