use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease, InteractionPolicy,
    MouseButton, MouseEvent, MouseEventKind, Point, Rect,
};
use serde_json::json;

use super::backend::{Backend, backend};
use super::keymap::Keymap;
use super::shell_aim::aim;
use super::target_pointer::send_bursts;
use super::wayland::pointer_send;
use crate::input::deliver::{ensure_budget, ensure_headed_click_policy};
use crate::input::target_world::LiveWorld;
use crate::input::target_world::PhysicalWorld;
use crate::system::shell::light::live_extents;
use crate::system::{SharedBus, list_displays};
use crate::tree::AtspiElement;

fn unreadable_extents() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The screen position of the shell control could not be read, so no event was sent",
    )
    .with_suggestion("Run 'snapshot' to refresh, then retry")
    .with_details(json!({
        "check": "geometry",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn read_extents(
    bus: &SharedBus,
    node: &AtspiElement,
    deadline: Deadline,
) -> Result<Rect, AdapterError> {
    live_extents(bus, node, deadline)?
        .map(Rect::from)
        .ok_or_else(unreadable_extents)
}

pub(crate) fn click(
    bus: &SharedBus,
    node: &AtspiElement,
    button: MouseButton,
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<Point, AdapterError> {
    ensure_headed_click_policy(policy)?;
    let deadline = lease.deadline();
    ensure_budget(deadline)?;
    let extents = read_extents(bus, node, deadline)?;
    let displays = list_displays(deadline)?;
    let point = aim(extents, &displays)?;
    let world = LiveWorld { bus, lease };
    match backend()? {
        Backend::X11(_) => {
            let plan = super::pointer_plan::plan_click(
                &Keymap::default(),
                point.clone(),
                &button,
                1,
                &[],
            )?;
            send_bursts(&world, node, &point, vec![plan], deadline)?;
        }
        Backend::Wayland => {
            world.validate_pointer(node, &point, deadline)?;
            pointer_send::send(
                &MouseEvent {
                    point: point.clone(),
                    button,
                    kind: MouseEventKind::Click { count: 1 },
                    modifiers: Vec::new(),
                },
                &displays,
                &mut || world.validate_pointer(node, &point, deadline),
                deadline,
            )?;
        }
    }
    Ok(point)
}

#[cfg(test)]
#[path = "shell_click_live_tests.rs"]
mod live_tests;
