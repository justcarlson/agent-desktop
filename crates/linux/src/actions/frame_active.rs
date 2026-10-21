use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::physical_target::{PhysicalWorld, ensure_budget};

use crate::system::{SharedBus, within};
use crate::tree::state_bits::{ACTIVE, StateBits};
use crate::tree::{AtspiElement, Field, read_node};

/// Reads the frame's own `ACTIVE` state. `None` means the state set could not
/// be read, which is not the same as the frame being inactive.
pub(crate) fn frame_active(
    bus: &SharedBus,
    frame: &AtspiElement,
    deadline: Deadline,
) -> Result<Option<bool>, AdapterError> {
    let shared = bus.get(deadline)?;
    let node = within(deadline, async { read_node(shared.inner(), frame).await })?;
    Ok(active_of(&node.traits.states))
}

pub(crate) fn active_of(states: &Field<Vec<u32>>) -> Option<bool> {
    states
        .present()
        .map(|words| StateBits::new(words).has(ACTIVE))
}

const ACTIVE_BUDGET: Duration = Duration::from_millis(1500);
const ACTIVE_POLL: Duration = Duration::from_millis(50);

pub(crate) fn poll_active(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    budget: Duration,
    deadline: Deadline,
) -> Result<Option<bool>, AdapterError> {
    let reads = budget.as_millis() / ACTIVE_POLL.as_millis();
    let mut last = None;
    for attempt in 0..=reads {
        ensure_budget(deadline)?;
        last = world.frame_active(element, deadline)?;
        if last == Some(true) {
            break;
        }
        if attempt < reads {
            world.pause(ACTIVE_POLL);
        }
    }
    Ok(last)
}

/// The focus gate of one keyboard delivery. The first check waits for the
/// frame to become active and refuses the call before any key is sent; later
/// checks, made between chunks, read once and stop the delivery where it is.
pub(crate) struct ActiveGate<'a, W: PhysicalWorld> {
    world: &'a W,
    element: &'a AtspiElement,
    deadline: Deadline,
    checked: bool,
}

impl<'a, W: PhysicalWorld> ActiveGate<'a, W> {
    pub(crate) fn new(world: &'a W, element: &'a AtspiElement, deadline: Deadline) -> Self {
        Self {
            world,
            element,
            deadline,
            checked: false,
        }
    }

    pub(crate) fn check(&mut self) -> Result<(), AdapterError> {
        let first = !self.checked;
        self.checked = true;
        let budget = if first { ACTIVE_BUDGET } else { Duration::ZERO };
        match poll_active(self.world, self.element, budget, self.deadline)? {
            Some(true) => Ok(()),
            read if first => Err(window_inactive(read)),
            _ => Err(window_lost()),
        }
    }
}

fn window_inactive(read: Option<bool>) -> AdapterError {
    let cause = match read {
        Some(_) => "not_active",
        None => "unreadable",
    };
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The target window is not the active window, so no key was sent",
    )
    .with_suggestion(
        "A pending system dialog can hold the keyboard; dismiss it, or focus the window, then retry",
    )
    .with_details(json!({
        "check": "window_active",
        "cause": cause,
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn window_lost() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The target window stopped being the active window while keys were being sent",
    )
    .with_suggestion("Focus the window again, then retry the remaining input")
    .with_details(json!({ "check": "window_active", "cause": "lost" }))
}

#[cfg(test)]
#[path = "frame_active_tests.rs"]
mod tests;
