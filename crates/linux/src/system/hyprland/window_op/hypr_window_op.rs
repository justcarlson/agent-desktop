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
use super::output_origin::OutputOrigin;
use super::restore_geometry::{needs_origins, placed, recorded};
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
    let origins = if needs_origins(op, &state) {
        world
            .origins()
            .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?
    } else {
        Vec::new()
    };
    let steps = if matches!(op, WindowOp::Maximize) {
        [recorded(&state, &origins)?, steps].concat()
    } else {
        steps
    };
    let dispatched = applied(world, &state, &steps, &origins, op);
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

/// Sends `steps`, then, for a restore that cleared a maximize, the steps
/// that put the window back where its record says.
fn applied(
    world: &mut impl WindowWorld,
    state: &ClientState,
    steps: &[WindowStep],
    origins: &[OutputOrigin],
    op: &WindowOp,
) -> Result<ClientState, AdapterError> {
    let settled = send_steps(world, state.address, steps, 0)?;
    let after = settled.last().cloned().ok_or_else(window_gone)?;
    if !matches!(op, WindowOp::Restore) || state.fullscreen == 0 {
        return Ok(after);
    }
    let held = settled
        .len()
        .checked_sub(2)
        .map_or(state, |index| &settled[index]);
    let placing = placed(held, &after, origins)?;
    let mut placed_states = send_steps(world, state.address, &placing, steps.len())?;
    Ok(placed_states.pop().unwrap_or(after))
}

/// Sends each step and returns the state each one settled in. `sent` is
/// how many steps of the same operation went before.
fn send_steps(
    world: &mut impl WindowWorld,
    address: u64,
    steps: &[WindowStep],
    sent: usize,
) -> Result<Vec<ClientState>, AdapterError> {
    let mut settled = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        let reply = world
            .request(&step.command)
            .map_err(|error| error.with_disposition(DeliverySemantics::uncertain()))?;
        if reply.trim() != ACCEPTED {
            return Err(rejected(&reply, sent + index > 0));
        }
        settled.push(settle(world, address, step)?);
    }
    Ok(settled)
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

#[cfg(test)]
#[path = "hypr_window_op_restore_tests.rs"]
mod restore_tests;
