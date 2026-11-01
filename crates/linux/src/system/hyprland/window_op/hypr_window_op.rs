//! Hyprland window operations. The window is bound to one toplevel and its
//! state read before anything is sent. Each step is sent by the toplevel's
//! address and proven by a `j/clients` read-back, because a reply of `ok`
//! does not show that anything changed (S9-31). The desktop is read before
//! and after, so a focus or view change the operation caused is reported.

use agent_desktop_core::{
    AdapterError, DeliverySemantics, OperationReport, OperationTarget, WindowInfo, WindowOp,
    WindowReadback,
};
use serde_json::{Value, json};

use super::client_state::ClientState;
use super::config_dialect::{ConfigDialect, DIALECT_PROBE, dialect_of};
use super::window_op_refusal::{refused, rejected, unverified, window_gone};
use super::window_plan::plan;
use super::window_step::WindowStep;
use super::window_world::WindowWorld;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::side_effects::side_effects;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::window_bind::bind_window;
use crate::system::window::window_op::window_op_verdict::requested;
use crate::system::window::window_report::window_readback;

const ACCEPTED: &str = "ok";
const RESTORED_SCOPE: &str = "window_restored_state";

/// Applies `op` to `win` and reports the read-back that proves it.
pub(crate) fn window_op_reported(
    world: &mut impl WindowWorld,
    win: &WindowInfo,
    op: &WindowOp,
) -> Result<OperationReport, AdapterError> {
    requested(op)?;
    let target = OperationTarget::for_window(win)?;
    let (state, dialect, before) = read_before(world, win)
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    let steps = plan(op, &state, before.active, dialect)?;
    if steps.is_empty() {
        let mut report = OperationReport::satisfied_without_delivery(action(op), target);
        report.result.details = Some(json!({ "observed": state.to_json() }));
        return Ok(report);
    }
    let dispatched = send_steps(world, state.address, &steps);
    let observed = dispatched.as_ref().ok().map(ClientState::to_json);
    let after = world.desktop().ok();
    let mut report =
        OperationReport::from_window_dispatch(win, op, Some(readback(op)), dispatched.map(drop))?;
    if let Some(Value::Object(details)) = report.result.details.as_mut() {
        if let Some(observed) = observed {
            details.insert("observed".to_owned(), observed);
        }
        let effects = after.map_or_else(Vec::new, |after| changes(&before, &after));
        if !effects.is_empty() {
            details.insert("side_effects".to_owned(), Value::Array(effects));
        }
    }
    Ok(report)
}

fn read_before(
    world: &mut impl WindowWorld,
    win: &WindowInfo,
) -> Result<(ClientState, ConfigDialect, DesktopState), AdapterError> {
    let (clients, toplevels) = world.inventory()?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let live = world.live_instance(win.pid.get());
    let address = bind_window(win, inventory, live.as_deref(), world.compositor())?
        .client
        .address;
    let state = world
        .states()?
        .into_iter()
        .find(|state| state.address == address)
        .ok_or_else(window_gone)?;
    let dialect = dialect_of(&world.request(DIALECT_PROBE)?).ok_or_else(|| {
        refused(
            "config_dialect_unknown",
            "The compositor answered neither as a Lua nor as a legacy Hyprland config",
            "Run a Hyprland release that answers 'hyprctl eval', then retry",
        )
    })?;
    let before = world.desktop()?;
    Ok((state, dialect, before))
}

fn send_steps(
    world: &mut impl WindowWorld,
    address: u64,
    steps: &[WindowStep],
) -> Result<ClientState, AdapterError> {
    let mut last = None;
    for (index, step) in steps.iter().enumerate() {
        let reply = world
            .request(&step.command)
            .map_err(|error| error.with_disposition(DeliverySemantics::uncertain()))?;
        if reply.trim() != ACCEPTED {
            return Err(rejected(&reply, index > 0));
        }
        last = Some(settle(world, address, step)?);
    }
    last.ok_or_else(window_gone)
}

fn settle(
    world: &mut impl WindowWorld,
    address: u64,
    step: &WindowStep,
) -> Result<ClientState, AdapterError> {
    let mut seen = None;
    loop {
        seen = match world.states() {
            Ok(states) => states.into_iter().find(|state| state.address == address),
            Err(_) => seen,
        };
        if let Some(state) = seen.as_ref().filter(|state| step.holds(state)) {
            return Ok(state.clone());
        }
        if !world.wait_more() {
            return Err(unverified(step, seen.as_ref()));
        }
    }
}

fn readback(op: &WindowOp) -> WindowReadback {
    window_readback(op).unwrap_or(WindowReadback::state(RESTORED_SCOPE))
}

fn action(op: &WindowOp) -> &'static str {
    match op {
        WindowOp::Resize { .. } => "resize-window",
        WindowOp::Move { .. } => "move-window",
        WindowOp::Minimize => "minimize",
        WindowOp::Maximize => "maximize",
        WindowOp::Restore => "restore",
    }
}

/// The focus and view changes between `before` and `after`: the focused
/// window first, then every workspace and cursor change.
fn changes(before: &DesktopState, after: &DesktopState) -> Vec<Value> {
    let mut effects = Vec::new();
    if before.active != after.active {
        effects.push(json!({
            "kind": "active_window_changed",
            "from": before.active.map(|address| format!("{address:#x}")),
            "to": after.active.map(|address| format!("{address:#x}")),
        }));
    }
    effects.extend(
        side_effects(before, after)
            .iter()
            .filter_map(|effect| serde_json::to_value(effect).ok()),
    );
    effects
}

#[cfg(test)]
#[path = "hypr_window_op_tests.rs"]
mod tests;
