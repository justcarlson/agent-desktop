use agent_desktop_core::{
    AdapterError, DismissNotificationRequest, InteractionLease, NotificationInfo,
};

use super::close::close_entry;
use super::session::Session;
use super::target::{Wanted, locate, reconfirm};
use super::world::{LiveWorld, ShellWorld};
use crate::system::SharedBus;

pub(crate) fn dismiss(
    bus: &SharedBus,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    dismiss_in(&LiveWorld::new(bus), request, lease)
}

/// Removes exactly the entry at the index through its own header control and
/// proves it by re-reading the list: the entry's object left, and no other
/// entry's did.
pub(crate) fn dismiss_in(
    world: &dyn ShellWorld,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    if request.policy.is_headed() {
        locate(
            world,
            request.index,
            request.app_filter,
            request.identity,
            lease.deadline(),
        )?;
    }
    let session = Session::open(world, request.policy, lease)?;
    let result = run(world, &request, lease);
    session.finish(result)
}

fn run(
    world: &dyn ShellWorld,
    request: &DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    let deadline = lease.deadline();
    let (index, app) = (request.index, request.app_filter);
    let first = locate(world, index, app, request.identity, deadline)?;
    let wanted = Wanted {
        index,
        app_filter: app,
        identity: request.identity,
    };
    let ready = reconfirm(world, wanted, &first.entry, deadline)?;
    close_entry(world, &ready.entry, &ready.all, request.policy, lease)
        .map(|_| ready.entry.info)
        .map_err(|unclosed| unclosed.into_error())
}

#[cfg(test)]
#[path = "dismiss_tests.rs"]
mod tests;
