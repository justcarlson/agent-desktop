use std::time::{Duration, Instant};

use agent_desktop_core::{
    ActionResult, AdapterError, DeliverySemantics, ErrorCode, InteractionLease, MouseButton,
    NotificationActionRequest,
};
use serde_json::json;

use super::extract::{Control, Entry, is_laid_out};
use super::list::read_listing;
use super::seen::holds;
use super::session::Session;
use super::signals::{BusSignals, SETTLE_BUDGET, SignalSource, Subscription};
use super::target::{Located, Wanted, locate, reconfirm, require};
use super::verify::{REMOVAL_SETTLE, evidence, settle_read, unproven_action_error};
use super::world::{LiveWorld, ShellWorld};
use crate::system::SharedBus;

const EXPAND_POLL: Duration = Duration::from_millis(100);

pub(crate) fn action(
    bus: &SharedBus,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    action_in(&LiveWorld::new(bus), &BusSignals, request, lease)
}

/// Clicks the named action button of the entry at the index, expanding the
/// entry first when its buttons are not laid out. The click only counts when
/// its effect shows: the entry's own object left the list, with the daemon's
/// action signal as the sharper proof when exactly one arrived. A click whose
/// effect never shows is a failure that says it was delivered, and the
/// application's own effect is never claimed.
pub(crate) fn action_in(
    world: &dyn ShellWorld,
    signals: &dyn SignalSource,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    if request.policy.is_headed() {
        locate(
            world,
            request.index,
            None,
            request.identity,
            lease.deadline(),
        )?;
    }
    let session = Session::open(world, request.policy, lease)?;
    let result = run(world, signals, &request, lease);
    session.finish(result)
}

fn run(
    world: &dyn ShellWorld,
    signals: &dyn SignalSource,
    request: &NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    let deadline = lease.deadline();
    let index = request.index;
    let first = locate(world, index, None, request.identity, deadline)?;
    let name = request.action_name;
    require_known(&first, name, index)?;
    let mut ready = reconfirm(world, wanted_by(request), &first.entry, deadline)?;
    if !laid_out(&ready, name) {
        let expand = require(ready.entry.expand.take(), "expand button", index)?;
        world.click(&expand.element, MouseButton::Left, request.policy, lease)?;
        ready = expanded(world, request, &first.entry, lease)?;
    }
    let button = pick(&mut ready, name, index)?;
    let subscription = signals.subscribe(deadline).ok();
    world.click(&button.element, MouseButton::Left, request.policy, lease)?;
    let clicked = Instant::now();
    let node = ready.entry.node.clone();
    let after = settle_read(world, deadline, REMOVAL_SETTLE, |now| !holds(now, &node))?;
    let seen = collect(subscription, clicked);
    match evidence(&seen, !holds(&after, &node)) {
        Some(proof) => Ok(
            ActionResult::delivered_unverified(name).with_details(json!({
                "evidence": proof.name(),
            })),
        ),
        None => Err(unproven_action_error(name)),
    }
}

fn wanted_by<'a>(request: &NotificationActionRequest<'a>) -> Wanted<'a> {
    Wanted {
        index: request.index,
        app_filter: None,
        identity: request.identity,
    }
}

fn collect(
    subscription: Option<Box<dyn Subscription>>,
    clicked: Instant,
) -> Vec<super::signals::Signal> {
    subscription
        .map(|held| held.collect(SETTLE_BUDGET.saturating_sub(clicked.elapsed())))
        .unwrap_or_default()
}

fn expanded(
    world: &dyn ShellWorld,
    request: &NotificationActionRequest<'_>,
    first: &Entry,
    lease: &InteractionLease,
) -> Result<Located, AdapterError> {
    loop {
        let ready = reconfirm(world, wanted_by(request), first, lease.deadline())
            .map_err(|error| explain_missing(world, error, first, request, lease))?;
        if laid_out(&ready, request.action_name) {
            return Ok(ready);
        }
        if lease.deadline().is_expired() {
            return Err(not_laid_out(request.action_name));
        }
        std::thread::sleep(EXPAND_POLL);
    }
}

/// A reconfirm that finds the entry missing after the expand press is told
/// apart by its object: when the object is gone the press removed the entry,
/// and any other mismatch keeps its own error.
fn explain_missing(
    world: &dyn ShellWorld,
    error: AdapterError,
    first: &Entry,
    request: &NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> AdapterError {
    if error.code != ErrorCode::NotificationNotFound {
        return error;
    }
    let gone = read_listing(world, lease.deadline())
        .is_ok_and(|listing| !holds(&listing.seen(), &first.node));
    if !gone {
        return error;
    }
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!(
            "The control pressed to reach the '{}' button removed the notification instead",
            request.action_name
        ),
    )
    .with_suggestion(
        "Run list-notifications; the shell's header controls are not in the expected order",
    )
    .with_disposition(DeliverySemantics::delivered_unverified())
}

fn not_laid_out(name: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!("The entry was expanded but the '{name}' button never appeared"),
    )
    .with_suggestion("Run list-notifications and retry")
    .with_disposition(DeliverySemantics::delivered_unverified())
}

fn matches_name(control: &Control, name: &str) -> bool {
    control.name.to_lowercase() == name.to_lowercase()
}

fn laid_out(located: &Located, name: &str) -> bool {
    located
        .entry
        .actions
        .iter()
        .find(|control| matches_name(control, name))
        .is_some_and(|control| is_laid_out(control.extents))
}

fn pick(located: &mut Located, name: &str, index: usize) -> Result<Control, AdapterError> {
    let position = located
        .entry
        .actions
        .iter()
        .position(|control| matches_name(control, name));
    require(
        position.map(|at| located.entry.actions.remove(at)),
        "action button",
        index,
    )
}

fn require_known(located: &Located, name: &str, index: usize) -> Result<(), AdapterError> {
    if located
        .entry
        .actions
        .iter()
        .any(|control| matches_name(control, name))
    {
        return Ok(());
    }
    let offered: Vec<&str> = located
        .entry
        .actions
        .iter()
        .map(|control| control.name.as_str())
        .collect();
    Err(AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("Action '{name}' is not offered by the notification at index {index}"),
    )
    .with_suggestion("Run list-notifications and use one of the entry's reported actions")
    .with_details(json!({ "available_actions": offered }))
    .with_disposition(DeliverySemantics::not_delivered()))
}

#[cfg(test)]
#[path = "action_tests.rs"]
mod tests;
