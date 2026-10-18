use agent_desktop_core::{LocatorField, Role};

use super::attributes::{AttributeRead, first_xml_role};
use super::role_numbers::{LANDMARK, LIST, PUSH_BUTTON, TERMINAL, TEXT};
use super::role_table::base_role;
use super::state_bits::{EDITABLE, HAS_POPUP, StateBits};
use super::{Field, NodeTraits};

const SELECTION_INTERFACE: &str = "org.a11y.atspi.Selection";
const LANDMARK_TOKENS: [&str; 7] = [
    "banner",
    "main",
    "navigation",
    "complementary",
    "contentinfo",
    "search",
    "form",
];

fn state_set(states: &Field<Vec<u32>>) -> Option<StateBits<'_>> {
    states.present().map(|words| StateBits::new(words))
}

fn text_role(states: &Field<Vec<u32>>) -> Role {
    if state_set(states).is_some_and(|bits| bits.has(EDITABLE)) {
        Role::TextField
    } else {
        Role::Document
    }
}

fn push_button_role(states: &Field<Vec<u32>>) -> Role {
    if state_set(states).is_some_and(|bits| bits.has(HAS_POPUP)) {
        Role::MenuButton
    } else {
        Role::Button
    }
}

fn list_role(interfaces: &Field<Vec<String>>) -> Role {
    let selectable = interfaces
        .present()
        .is_some_and(|names| names.iter().any(|name| name == SELECTION_INTERFACE));
    if selectable {
        Role::ListBox
    } else {
        Role::List
    }
}

fn landmark_role(attributes: &AttributeRead) -> Role {
    match first_xml_role(attributes) {
        Some(token) if LANDMARK_TOKENS.contains(&token.as_str()) => Role::from_token(&token),
        _ => Role::Region,
    }
}

/// The canonical role for an AT-SPI role number. A refinement that depends
/// on another read falls back to the unrefined role when that read failed.
pub(crate) fn role_of(
    number: u32,
    states: &Field<Vec<u32>>,
    interfaces: &Field<Vec<String>>,
    attributes: &AttributeRead,
) -> Role {
    match number {
        TEXT | TERMINAL => text_role(states),
        PUSH_BUTTON => push_button_role(states),
        LIST => list_role(interfaces),
        LANDMARK => landmark_role(attributes),
        other => base_role(other),
    }
}

/// The role token of a node, or unknown when the role read failed. That is
/// not the canonical `unknown` role: it says nothing was read.
pub(crate) fn resolve_role(
    traits: &NodeTraits,
    attributes: &AttributeRead,
) -> LocatorField<String> {
    match traits.role.present() {
        Some(number) => LocatorField::Known(
            role_of(*number, &traits.states, &traits.interfaces, attributes)
                .as_str()
                .to_owned(),
        ),
        None => LocatorField::Unknown,
    }
}

#[cfg(test)]
#[path = "roles_tests.rs"]
mod tests;
