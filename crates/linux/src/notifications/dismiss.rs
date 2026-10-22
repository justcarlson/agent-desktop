use agent_desktop_core::{
    AdapterError, DismissNotificationRequest, InteractionLease, MouseButton, NotificationInfo,
};

use super::session::Session;
use super::target::{Wanted, locate, reconfirm, require};
use super::verify::{Removal, bystander_error, judge_removal, settle_read, survived_error};
use super::world::{LiveWorld, ShellWorld};
use crate::system::SharedBus;

pub(crate) fn dismiss(
    bus: &SharedBus,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    dismiss_in(&LiveWorld::new(bus), request, lease)
}

/// Removes exactly the entry at the index through its own close button and
/// proves it by re-reading the list: the target left, and nothing else did.
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
    let ready = reconfirm(world, wanted, &first.entry.info, deadline)?;
    let close = require(ready.entry.close, "close button", index)?;
    let target = ready.entry.info;
    world.click(&close.element, MouseButton::Left, request.policy, lease)?;
    let after = settle_read(world, deadline, |now| {
        judge_removal(&ready.all, now, &target) != Removal::Pending
    })?;
    match judge_removal(&ready.all, &after, &target) {
        Removal::Settled => Ok(target),
        Removal::Pending => Err(survived_error("close button of the notification")),
        Removal::Bystander => Err(bystander_error()),
    }
}

#[cfg(test)]
#[path = "dismiss_tests.rs"]
mod tests;
