//! Finds the bound toplevel of a ref element and the global point inside it
//! that the pointer aims at.

use agent_desktop_core::{AdapterError, Deadline, Point, Rect};

use super::aim_rect::aim_rect;
use super::bound_layout::BoundLayout;
use super::layout_placement::Placement;
use super::layout_rect::layout_rect;
use super::placed_target::PlacedTarget;
use super::popup_surface::popup_surface_refusal;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::read_desktop;
use crate::system::hyprland::clients::hypr_client::HyprClient;
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
/// event, and so does a frame that is a popup surface.
pub(crate) fn place_target(
    binding: &HyprlandBinding,
    bus: &SharedBus,
    element: &AtspiElement,
    verified: Option<&Point>,
    deadline: Deadline,
) -> Result<PlacedTarget, AdapterError> {
    let frame = element.frame().ok_or_else(|| missing("frame"))?;
    let (address, clients) =
        bound_clients(binding, bus, frame, element.is_menu_target(), deadline)?;
    if let Some(point) = verified {
        point.validate()?;
        return Ok(PlacedTarget {
            address,
            at: point.clone(),
        });
    }
    let client = client_at(&clients, address)?;
    let placed = place_element(binding, bus, element, client, deadline)?;
    if !placed.shown {
        return Err(ReceiverRefusal::TargetNotShown.to_error());
    }
    let at = centre(&placed.rect);
    at.validate()?;
    Ok(PlacedTarget { address, at })
}

/// The global layout rectangle of `element`: its visible window rectangle
/// moved to the compositor origin of the toplevel its frame binds to.
pub(crate) fn placed_bounds(
    binding: &HyprlandBinding,
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Result<Rect, AdapterError> {
    let frame = element.frame().ok_or_else(|| missing("frame"))?;
    let (address, clients) =
        bound_clients(binding, bus, frame, element.is_menu_target(), deadline)?;
    let client = client_at(&clients, address)?;
    Ok(place_element(binding, bus, element, client, deadline)?.rect)
}

/// The compositor client `frame` binds to. `menu_target` is whether the
/// node placed below `frame` was reached from a menu target.
pub(crate) fn frame_client(
    binding: &HyprlandBinding,
    bus: &SharedBus,
    frame: &AtspiElement,
    menu_target: bool,
    deadline: Deadline,
) -> Result<HyprClient, AdapterError> {
    let (address, clients) = bound_clients(binding, bus, frame, menu_target, deadline)?;
    client_at(&clients, address).cloned()
}

fn bound_clients(
    binding: &HyprlandBinding,
    bus: &SharedBus,
    frame: &AtspiElement,
    menu_target: bool,
    deadline: Deadline,
) -> Result<(u64, Vec<HyprClient>), AdapterError> {
    let evidence = frame_evidence(bus, frame, menu_target, deadline)?;
    let (clients, toplevels) = read_inventory(binding, deadline)?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let address = bound_toplevel(&evidence, live_instance(evidence.pid).as_deref(), inventory)?;
    Ok((address, clients))
}

fn client_at(clients: &[HyprClient], address: u64) -> Result<&HyprClient, AdapterError> {
    clients
        .iter()
        .find(|client| client.address == address)
        .ok_or_else(|| missing("target_client"))
}

fn place_element(
    binding: &HyprlandBinding,
    bus: &SharedBus,
    element: &AtspiElement,
    client: &HyprClient,
    deadline: Deadline,
) -> Result<Placement, AdapterError> {
    let window_rect = aim_rect(
        read_window_extents(bus, element, deadline),
        element
            .scroll_container()
            .and_then(|container| read_window_extents(bus, container, deadline)),
    )
    .ok_or_else(|| missing("geometry"))?;
    let desktop = read_desktop(binding, deadline)?;
    let displays = crate::system::list_displays(deadline)?;
    Ok(layout_rect(
        &BoundLayout {
            client,
            displays: &displays,
            desktop: &desktop,
        },
        &window_rect,
    ))
}

/// The refusal for an element whose frame is a popup surface, read from the
/// frame's role alone.
pub(crate) fn popup_refusal(
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Result<Option<AdapterError>, AdapterError> {
    let frame = element.frame().ok_or_else(|| missing("frame"))?;
    let node = read_frame(bus, frame, deadline)?;
    Ok(popup_surface_refusal(
        node.traits.role.present().copied(),
        element.is_menu_target(),
    ))
}

fn read_frame(
    bus: &SharedBus,
    frame: &AtspiElement,
    deadline: Deadline,
) -> Result<crate::tree::RawNode, AdapterError> {
    let connection = bus.get(deadline)?;
    within(deadline, async {
        crate::tree::read_node(connection.inner(), frame).await
    })
}

fn frame_evidence(
    bus: &SharedBus,
    frame: &AtspiElement,
    menu_target: bool,
    deadline: Deadline,
) -> Result<WindowEvidence, AdapterError> {
    let node = read_frame(bus, frame, deadline)?;
    if let Some(refusal) = popup_surface_refusal(node.traits.role.present().copied(), menu_target) {
        return Err(refusal);
    }
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
