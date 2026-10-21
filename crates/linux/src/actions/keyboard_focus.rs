use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::physical_target::{PhysicalWorld, ensure_budget};
use crate::system::{ACCESSIBLE_INTERFACE, SharedBus, within};
use crate::tree::state_bits::{FOCUSED, StateBits};
use crate::tree::{AtspiElement, COMPONENT_INTERFACE};

pub(super) fn refused() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Keyboard focus on the target could not be proven; keyboard delivery was refused",
    )
    .with_details(json!({
        "check": "target_focus",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

pub(super) fn grab(
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let connection = bus.get(deadline).map_err(|_| refused())?;
    within(deadline, async {
        connection
            .inner()
            .call_method(
                Some(element.bus_name()),
                element.path(),
                Some(COMPONENT_INTERFACE),
                "GrabFocus",
                &(),
            )
            .await
            .map_err(|_| refused())?;
        Ok(())
    })
    .map_err(|_| refused())
}

pub(super) fn read(
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let connection = bus.get(deadline).map_err(|_| refused())?;
    within(deadline, async {
        let reply = connection
            .inner()
            .call_method(
                Some(element.bus_name()),
                element.path(),
                Some(ACCESSIBLE_INTERFACE),
                "GetState",
                &(),
            )
            .await
            .map_err(|_| refused())?;
        let words: Vec<u32> = reply.body().deserialize().map_err(|_| refused())?;
        Ok(StateBits::new(&words).has(FOCUSED))
    })
    .map_err(|_| refused())
}

pub(super) fn prepare(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    if world.target_focused(element, deadline)? {
        return Ok(());
    }
    world.focus_target(element, deadline)?;
    for attempt in 0..=10 {
        ensure_budget(deadline)?;
        if world.target_focused(element, deadline)? {
            return Ok(());
        }
        if attempt < 10 {
            world.pause(Duration::from_millis(50));
        }
    }
    Err(refused())
}
