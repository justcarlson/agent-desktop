//! Finds the bound toplevel of a ref element and the global point inside it
//! that the pointer aims at.

use agent_desktop_core::{AdapterError, Deadline, Point, Rect};

use super::aim_rect::aim_rect;
use super::bound_layout::BoundLayout;
use super::layout_rect::layout_rect;
use super::placed_target::PlacedTarget;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::read_desktop;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::inventory_read::read_inventory;
use crate::system::hyprland::identity::window_evidence::WindowEvidence;
use crate::system::hyprland::receiver::receiver_refusal::ReceiverRefusal;
use crate::system::window::compositor_identity::bound_toplevel;
use crate::system::window::compositor_listing::live_instance;
use crate::system::{SharedBus, within};
use crate::tree::AtspiElement;
use crate::tree::surfaces::window_extents::read_window_extents;

/// The toplevel `element`'s frame binds to and the point to aim at: the
/// verified point when the caller has one, else the centre of the element's
/// visible window rectangle placed at the toplevel's compositor origin.
/// Missing geometry or a toplevel that is not shown refuses before any
/// event.
pub(crate) fn place_target(
    binding: &HyprlandBinding,
    bus: &SharedBus,
    element: &AtspiElement,
    verified: Option<&Point>,
    deadline: Deadline,
) -> Result<PlacedTarget, AdapterError> {
    let frame = element.frame().ok_or_else(|| missing("frame"))?;
    let evidence = frame_evidence(bus, frame, deadline)?;
    let (clients, toplevels) = read_inventory(binding, deadline)?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let address = bound_toplevel(&evidence, live_instance(evidence.pid).as_deref(), inventory)?;
    if let Some(point) = verified {
        point.validate()?;
        return Ok(PlacedTarget {
            address,
            at: point.clone(),
        });
    }
    let client = clients
        .iter()
        .find(|client| client.address == address)
        .ok_or_else(|| missing("target_client"))?;
    let window_rect = aim_rect(
        read_window_extents(bus, element, deadline),
        element
            .scroll_container()
            .and_then(|container| read_window_extents(bus, container, deadline)),
    )
    .ok_or_else(|| missing("geometry"))?;
    let desktop = read_desktop(binding, deadline)?;
    let displays = crate::system::list_displays(deadline)?;
    let placed = layout_rect(
        &BoundLayout {
            client,
            displays: &displays,
            desktop: &desktop,
        },
        &window_rect,
    );
    if !placed.shown {
        return Err(ReceiverRefusal::TargetNotShown.to_error());
    }
    let at = centre(&placed.rect);
    at.validate()?;
    Ok(PlacedTarget { address, at })
}

fn frame_evidence(
    bus: &SharedBus,
    frame: &AtspiElement,
    deadline: Deadline,
) -> Result<WindowEvidence, AdapterError> {
    let connection = bus.get(deadline)?;
    let node = within(deadline, async {
        crate::tree::read_node(connection.inner(), frame).await
    })?;
    let title = node
        .properties
        .name
        .present()
        .cloned()
        .ok_or_else(|| missing("frame_title"))?;
    let size = read_window_extents(bus, frame, deadline)
        .map(|extents| (f64::from(extents.width), f64::from(extents.height)));
    Ok(WindowEvidence {
        pid: frame.pid(),
        instance: live_instance(frame.pid()),
        title,
        size,
    })
}

fn centre(rect: &Rect) -> Point {
    Point {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    }
}

fn missing(check: &'static str) -> AdapterError {
    ReceiverRefusal::EvidenceMissing { check }.to_error()
}
