use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use agent_desktop_core::Deadline;
use futures_util::future::join3;
use zbus::Connection;

use super::absence::Liveness;
use super::node_read::{COMPONENT_INTERFACE, call, get_one, string_value};
use super::offscreen::{OffscreenInput, offscreen};
use super::placement::Placement;
use super::state_bits::{ICONIFIED, StateBits};
use super::toolkit_trust::reports_global_position;
use super::window_extents::read_window_extents;
use super::{AtspiElement, Extents};
use crate::system::registry::toolkit;
use crate::system::toolkit_id::ToolkitId;
use crate::system::x11::stacking::frame_window_state;
use crate::system::x11::{DisplayReader, FrameFacts};
use crate::system::{ACCESSIBLE_INTERFACE, REGISTRY_ROOT_PATH, SharedBus, within};

const SCREEN_COORDINATES: u32 = 0;

fn known_toolkits() -> &'static Mutex<HashMap<String, ToolkitId>> {
    static KNOWN: OnceLock<Mutex<HashMap<String, ToolkitId>>> = OnceLock::new();
    KNOWN.get_or_init(Mutex::default)
}

/// The toolkit of the application `element` belongs to. An application keeps
/// its toolkit for as long as its unique bus name lives, so a name that was
/// read is remembered and costs no call afterwards; a read that gave no
/// toolkit name is not remembered.
pub(crate) fn toolkit_of(bus: &SharedBus, element: &AtspiElement, deadline: Deadline) -> ToolkitId {
    let remembered = known_toolkits()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(element.bus_name())
        .cloned();
    if let Some(known) = remembered {
        return known;
    }
    let Ok(connection) = bus.get(deadline) else {
        return ToolkitId::default();
    };
    let root = AtspiElement::new(element.bus_name(), REGISTRY_ROOT_PATH, element.pid());
    let read = within(deadline, async {
        Ok(toolkit(connection.inner(), &root).await)
    })
    .unwrap_or_default();
    if read.name.is_some() {
        known_toolkits()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(element.bus_name().to_owned(), read.clone());
    }
    read
}

/// Whether the application of `element` reports screen positions that are
/// global.
pub(crate) fn global_position(bus: &SharedBus, element: &AtspiElement, deadline: Deadline) -> bool {
    reports_global_position(&toolkit_of(bus, element, deadline))
}

struct FrameRead {
    iconified: bool,
    title: String,
    screen: Option<Extents>,
}

async fn read_frame(connection: &Connection, frame: &AtspiElement) -> FrameRead {
    let (state, title, extents) = join3(
        call(connection, frame, ACCESSIBLE_INTERFACE, "GetState", &()),
        get_one(connection, frame, "Name", string_value, Liveness::Alive),
        call(
            connection,
            frame,
            COMPONENT_INTERFACE,
            "GetExtents",
            &(SCREEN_COORDINATES,),
        ),
    )
    .await;
    let words = state
        .ok()
        .and_then(|reply| reply.body().deserialize::<Vec<u32>>().ok());
    FrameRead {
        iconified: words.is_some_and(|words| StateBits::new(&words).has(ICONIFIED)),
        title: title
            .ok()
            .and_then(|field| field.present().cloned())
            .unwrap_or_default(),
        screen: extents
            .ok()
            .and_then(|reply| reply.body().deserialize::<(i32, i32, i32, i32)>().ok())
            .map(|(x, y, width, height)| Extents {
                x,
                y,
                width,
                height,
            }),
    }
}

/// Whether the display server need not be asked: the window is known
/// minimized, or the node is already provably off the window and the toolkit's
/// own positions are global, so no client origin is wanted either.
pub(crate) fn skips_display_server(
    iconified: bool,
    wholly_outside: bool,
    global_position: bool,
) -> bool {
    iconified || (wholly_outside && global_position)
}

fn wholly_outside(placement: &Placement) -> bool {
    offscreen(OffscreenInput {
        node: placement.node,
        frame: placement.frame,
        container: placement.container,
        visible: true,
        showing: true,
        sentinel: false,
        minimized: Some(false),
    }) == Some(true)
}

/// Reads where `element` sits in its window once: its own rectangle, the
/// frame's and the scroll container's in window coordinates, whether the frame
/// is iconified, the toolkit, and what the display server says about the
/// window. Every part that could not be read stays absent; nothing is guessed.
pub(crate) fn placement(bus: &SharedBus, element: &AtspiElement, deadline: Deadline) -> Placement {
    let toolkit = toolkit_of(bus, element, deadline);
    let mut placed = Placement {
        node: read_window_extents(bus, element, deadline),
        container: element
            .scroll_container()
            .and_then(|container| read_window_extents(bus, container, deadline)),
        toolkit,
        ..Placement::default()
    };
    let (Some(frame), Ok(connection)) = (element.frame(), bus.get(deadline)) else {
        return placed;
    };
    placed.frame = read_window_extents(bus, frame, deadline);
    let read = within(deadline, async {
        Ok(read_frame(connection.inner(), frame).await)
    });
    let Ok(read) = read else {
        return placed;
    };
    placed.frame_iconified = read.iconified;
    let global = reports_global_position(&placed.toolkit);
    if !skips_display_server(read.iconified, wholly_outside(&placed), global) {
        placed.x11 = frame_window_state(
            &DisplayReader,
            deadline,
            &FrameFacts {
                pid: frame.pid(),
                title: read.title,
                extents: read.screen,
                global_position: global,
            },
        );
    }
    placed
}

#[cfg(test)]
#[path = "live_offscreen_tests.rs"]
mod tests;
