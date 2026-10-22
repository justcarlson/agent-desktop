use agent_desktop_core::Deadline;

use super::window_filter::is_window_shape;
use crate::system::process::toolkit_id::ToolkitId;
use crate::system::registry::registry_frames::{RootRead, read_frames};
use crate::system::{REGISTRY_ROOT_PATH, SharedBus};
use crate::tree::role_numbers::PANEL;
use crate::tree::state_bits::{MODAL, SHOWING, StateBits};
use crate::tree::toolkit_trust::{ToolkitKind, kind};
use crate::tree::{AtspiElement, Extents, RawNode};

fn size(node: &RawNode) -> Option<(i32, i32)> {
    let bounds = node.extents.present()?;
    (bounds.width > 0 && bounds.height > 0 && bounds.x != i32::MIN && bounds.y != i32::MIN)
        .then_some((bounds.width, bounds.height))
}

/// An unnamed GTK4 modal panel may use its size to identify an X11 client
/// only when no other accessible window could claim that size. An unread
/// window size leaves the association unknown.
pub(crate) fn permits_size_match(
    node: &RawNode,
    siblings: &[RawNode],
    toolkit: &ToolkitId,
    expected: Option<Extents>,
) -> bool {
    let eligible = kind(toolkit) == ToolkitKind::Gtk4
        && node.traits.role.present() == Some(&PANEL)
        && node.properties.name.present().is_some_and(String::is_empty)
        && node.traits.states.present().is_some_and(|words| {
            let bits = StateBits::new(words);
            bits.has(MODAL) && bits.has(SHOWING)
        });
    let Some(target) = size(node).filter(|_| eligible) else {
        return false;
    };
    if expected.map(|bounds| (bounds.width, bounds.height)) != Some(target) {
        return false;
    }
    let mut matching = 0;
    for sibling in siblings {
        if sibling.traits.role.is_failed()
            || sibling.traits.interfaces.is_failed()
            || sibling.traits.states.is_failed()
        {
            return false;
        }
        if is_window_shape(
            &sibling.traits.role,
            &sibling.traits.interfaces,
            &sibling.traits.states,
        ) {
            let Some(other) = size(sibling) else {
                return false;
            };
            matching += usize::from(target == other);
        }
    }
    matching == 1
}

pub(crate) fn permits_on_bus(
    bus: &SharedBus,
    frame: &AtspiElement,
    toolkit: &ToolkitId,
    expected: Option<Extents>,
    deadline: Deadline,
) -> bool {
    if kind(toolkit) != ToolkitKind::Gtk4 {
        return false;
    }
    let root = AtspiElement::new(frame.bus_name(), REGISTRY_ROOT_PATH, frame.pid());
    let Ok(mut apps) = read_frames(bus, &[root], deadline) else {
        return false;
    };
    let Some(app) = apps.pop().filter(|app| app.read == RootRead::Read) else {
        return false;
    };
    app.nodes
        .iter()
        .find(|node| node.element == *frame)
        .is_some_and(|node| permits_size_match(node, &app.nodes, toolkit, expected))
}

#[cfg(test)]
#[path = "frame_identity_tests.rs"]
mod tests;
