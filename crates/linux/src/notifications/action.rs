use std::time::{Duration, Instant};

use agent_desktop_core::{
    ActionResult, AdapterError, DeliverySemantics, ErrorCode, InteractionLease, MouseButton,
    NotificationActionRequest, NotificationInfo,
};
use serde_json::json;

use super::extract::{Control, is_laid_out};
use super::session::Session;
use super::signals::{BusSignals, SETTLE_BUDGET, SignalSource, Subscription};
use super::target::{Located, locate, reconfirm, require};
use super::verify::{evidence, matching_count, settle_read};
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
/// entry first when its buttons are not laid out. The result is always
/// delivered and unverified: the application's effect is out of reach, and a
/// missed daemon signal weakens the evidence but never turns a delivered
/// click into a failure.
pub(crate) fn action_in(
    world: &dyn ShellWorld,
    signals: &dyn SignalSource,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
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
    let mut ready = reconfirm(
        world,
        index,
        None,
        request.identity,
        &first.entry.info,
        deadline,
    )?;
    if !laid_out(&ready, name) {
        let expand = require(ready.entry.expand.take(), "expand button", index)?;
        world.click(&expand.element, MouseButton::Left, request.policy, lease)?;
        ready = expanded(world, request, &first.entry.info, lease)?;
    }
    let button = pick(&mut ready, name, index)?;
    let subscription = signals.subscribe(deadline).ok();
    world.click(&button.element, MouseButton::Left, request.policy, lease)?;
    let clicked = Instant::now();
    let target = ready.entry.info;
    let after = settle_read(world, deadline, |now| {
        matching_count(now, &target) < matching_count(&ready.all, &target)
    })?;
    let gone = matching_count(&after, &target) < matching_count(&ready.all, &target);
    let seen = collect(subscription, clicked);
    let proof = evidence(&seen, gone);
    Ok(
        ActionResult::delivered_unverified(name).with_details(json!({
            "evidence": proof.name(),
        })),
    )
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
    first: &NotificationInfo,
    lease: &InteractionLease,
) -> Result<Located, AdapterError> {
    let index = request.index;
    loop {
        let ready = reconfirm(
            world,
            index,
            None,
            request.identity,
            first,
            lease.deadline(),
        )?;
        if laid_out(&ready, request.action_name) {
            return Ok(ready);
        }
        if lease.deadline().is_expired() {
            return Err(not_laid_out(request.action_name));
        }
        std::thread::sleep(EXPAND_POLL);
    }
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
