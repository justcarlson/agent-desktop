use agent_desktop_core::ref_alloc::is_ref_able_role_actions;
use agent_desktop_core::{LocatorField, Role};

use super::action_names::capabilities;
use super::role_numbers::{FILLER, PANEL, SECTION, is_web_document};
use super::value_read::ValueRead;
use super::{Field, NodeClass, NodeProperties, ObservedRead};

/// Whether a node is a wrapper: a panel, section or filler with no name,
/// description, identifier or value and no primary action. It is a pure
/// layout node, so it costs raw depth and no logical depth. A read that
/// failed or an unknown capability list never makes a node a wrapper, and a
/// node that advertises more than the ubiquitous context-menu, focus and
/// scroll affordances is addressable and keeps its place.
pub(crate) fn is_wrapper_shape(
    role: &Field<u32>,
    properties: &NodeProperties,
    value: &ValueRead,
    capabilities: &LocatorField<Vec<String>>,
) -> bool {
    let layout_role = matches!(role.present(), Some(&PANEL | &SECTION | &FILLER));
    let LocatorField::Known(tokens) = capabilities else {
        return false;
    };
    layout_role
        && properties.name.is_blank()
        && properties.description.is_blank()
        && properties.accessible_id.is_blank()
        && value.value.is_blank()
        && !is_ref_able_role_actions(Role::Group.as_str(), tokens)
}

/// Classifies one read for the walk.
pub(crate) fn classify(read: &ObservedRead) -> NodeClass {
    let node = &read.node;
    let caps = capabilities(&node.actions, &node.traits.role, &node.traits.states);
    NodeClass {
        wrapper: is_wrapper_shape(&node.traits.role, &node.properties, &read.value, &caps),
        web_document: is_web_document(node.traits.role.present().copied()),
    }
}

#[cfg(test)]
#[path = "wrapper_tests.rs"]
mod tests;
