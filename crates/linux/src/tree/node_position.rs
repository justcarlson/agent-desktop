use agent_desktop_core::Deadline;
use futures_util::future::join_all;

use agent_desktop_core::BoundsSpace;

use super::frame_geometry::pointer_bounds;
use super::toolkit_trust::{read_toolkit, reports_global_position};
use super::window_extents::read_window_extents_async;
use super::{AtspiElement, Extents, Field, ObservedRead, Walk, WalkedNode, read_node};
use crate::system::toolkit_id::ToolkitId;
use crate::system::window_position::frame_origin;
use crate::system::x11::stacking_source::StackingSource;
use crate::system::x11::{DisplayReader, FrameFacts, X11State};
use crate::system::{REGISTRY_ROOT_PATH, SharedBus, within};

const IN_FLIGHT: usize = 64;

fn is_placed(extents: &Field<Extents>) -> bool {
    matches!(extents, Field::Present(extents) if extents.x != i32::MIN && extents.y != i32::MIN)
}

fn for_each_read(node: &mut WalkedNode<ObservedRead>, visit: &mut impl FnMut(&mut ObservedRead)) {
    visit(&mut node.payload);
    for child in &mut node.children {
        for_each_read(child, visit);
    }
}

/// Rewrites the extents of every placed node to the rectangle a pointer is
/// aimed in: the window-coordinate rectangle moved to the client origin.
/// `windows` holds, in walk order, the window-coordinate rectangle of each
/// placed node. A node whose position cannot be proved ends without extents,
/// because the toolkit's own position is `[0, 0]` and is never a position.
fn apply(
    root: &mut WalkedNode<ObservedRead>,
    toolkit: &ToolkitId,
    origin: Option<(i32, i32)>,
    windows: &[Option<Extents>],
) {
    let mut next = windows.iter();
    for_each_read(root, &mut |read| {
        if !is_placed(&read.node.extents) {
            return;
        }
        let window = next.next().copied().flatten();
        place_read_bounds(read, toolkit, origin, window);
    });
}

/// Places one read for evidence: the pointer rectangle when the client origin
/// is known, otherwise the toolkit's own window rectangle. A Wayland session
/// reports no global position, so the window rectangle is the only stable
/// evidence and is tagged with its frame; a node with neither keeps none.
fn place_read_bounds(
    read: &mut ObservedRead,
    toolkit: &ToolkitId,
    origin: Option<(i32, i32)>,
    window: Option<Extents>,
) {
    let Field::Present(screen) = &read.node.extents else {
        read.evidence_bounds = None;
        read.node.extents = Field::Absent;
        return;
    };
    read.evidence_bounds = window
        .filter(|window| !is_sentinel(*window))
        .map(|window| (window, BoundsSpace::Window));
    read.node.extents = origin
        .zip(window)
        .and_then(|(origin, window)| pointer_bounds(toolkit, *screen, window, Some(origin)))
        .map_or(Field::Absent, Field::Present);
}

fn is_sentinel(extents: Extents) -> bool {
    extents.x == i32::MIN || extents.y == i32::MIN
}

fn placed_elements(root: &mut WalkedNode<ObservedRead>) -> Vec<AtspiElement> {
    let mut elements = Vec::new();
    for_each_read(root, &mut |read| {
        if is_placed(&read.node.extents) {
            elements.push(read.node.element.clone());
        }
    });
    elements
}

async fn window_rectangles(
    connection: &zbus::Connection,
    elements: &[AtspiElement],
) -> Vec<Option<Extents>> {
    let mut all = Vec::with_capacity(elements.len());
    for chunk in elements.chunks(IN_FLIGHT) {
        all.extend(
            join_all(
                chunk
                    .iter()
                    .map(|element| read_window_extents_async(connection, element)),
            )
            .await,
        );
    }
    all
}

async fn frame_title(connection: &zbus::Connection, frame: &AtspiElement) -> Option<String> {
    let node = read_node(connection, frame).await.ok()?;
    Some(node.properties.name.present().cloned().unwrap_or_default())
}

/// The origin of the frame's X11 client. The title is read on the bus; the
/// display-server read then runs here, outside any polled future, under the
/// caller's deadline.
fn origin_of(
    connection: &zbus::Connection,
    frame: &AtspiElement,
    deadline: Deadline,
) -> Option<(i32, i32)> {
    let title = within(deadline, async { Ok(frame_title(connection, frame).await) })
        .ok()
        .flatten()?;
    let facts = FrameFacts {
        pid: frame.pid(),
        title,
        extents: None,
        global_position: false,
    };
    let X11State::Ready { clients, .. } = DisplayReader.clients_within(deadline) else {
        return None;
    };
    frame_origin(&facts, &clients)
}

/// Puts the node rectangles of a finished walk where a pointer can be aimed
/// when the application's toolkit reports no global position. A toolkit with
/// global positions is left as read. Anything else gets its window-coordinate
/// rectangles moved to the window's X11 client origin, and the rectangles it
/// cannot place are dropped.
pub(crate) fn place_nodes(bus: &SharedBus, walk: &mut Walk<ObservedRead>, deadline: Deadline) {
    let Ok(connection) = bus.get(deadline) else {
        return;
    };
    let element = walk.root.payload.node.element.clone();
    let app_root = AtspiElement::new(element.bus_name(), REGISTRY_ROOT_PATH, element.pid());
    let toolkit = within(deadline, async {
        Ok(read_toolkit(connection.inner(), &app_root).await)
    })
    .unwrap_or_default();
    if reports_global_position(&toolkit) {
        return;
    }
    let frame = element.frame().cloned();
    let origin = frame.and_then(|frame| origin_of(connection.inner(), &frame, deadline));
    let elements = placed_elements(&mut walk.root);
    let windows = within(deadline, async {
        Ok(window_rectangles(connection.inner(), &elements).await)
    })
    .unwrap_or_default();
    apply(&mut walk.root, &toolkit, origin, &windows);
}

#[cfg(test)]
#[path = "node_position_tests.rs"]
mod tests;

/// Places one node read without walking its window: the same rule the walk
/// applies, used where only one node's evidence is wanted. A toolkit with
/// global positions is left as read.
pub(crate) fn place_single(bus: &SharedBus, read: &mut ObservedRead, deadline: Deadline) {
    let element = read.node.element.clone();
    let toolkit = super::live_offscreen::toolkit_of(bus, &element, deadline);
    if reports_global_position(&toolkit) {
        return;
    }
    let Ok(connection) = bus.get(deadline) else {
        return;
    };
    let origin = element
        .frame()
        .and_then(|frame| origin_of(connection.inner(), frame, deadline));
    let window = super::window_extents::read_window_extents(bus, &element, deadline);
    place_read_bounds(read, &toolkit, origin, window);
}
