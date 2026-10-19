use agent_desktop_core::{
    ElementIdentifier, IdentifierEvidence, IdentifierKind, LocatorEvidence, LocatorField,
    LocatorRefEvidence, Rect, state,
};

use super::TrayTarget;
use super::identity;

pub(super) const ITEM_ROLE: &str = "menubutton";
pub(super) const ENTRY_ROLE: &str = "menuitem";
pub(super) const SEPARATOR_ROLE: &str = "separator";
pub(super) const UNBOUND_DESCRIPTION: &str = "tray item without D-Bus identity";

/// The on-screen presence of a tray node: the rectangle the shell panel node
/// reports, or none with the node marked offscreen. A rectangle is never
/// made up for a node that was not observed.
pub(super) fn presence(observed: Option<Rect>) -> (Option<Rect>, bool) {
    match observed.filter(|rect| rect.width > 0.0 && rect.height > 0.0) {
        Some(rect) => (Some(rect), false),
        None => (None, true),
    }
}

pub(super) fn name_field(name: &str) -> LocatorField<String> {
    if name.is_empty() {
        LocatorField::Absent
    } else {
        LocatorField::Known(name.to_owned())
    }
}

pub(super) fn identifiers(target: &TrayTarget) -> IdentifierEvidence {
    IdentifierEvidence::typed(
        [ElementIdentifier {
            kind: IdentifierKind::RuntimeId,
            value: identity::encode(target),
        }],
        Some(0),
        true,
    )
}

pub(super) fn item_states(passive: bool, mut states: Vec<String>) -> Vec<String> {
    states.retain(|token| token != state::HIDDEN);
    if passive {
        states.push(state::HIDDEN.to_owned());
    }
    states
}

pub(super) fn fresh(
    role: &str,
    name: &str,
    target: &TrayTarget,
    states: Vec<String>,
    actions: Vec<String>,
) -> LocatorEvidence {
    LocatorEvidence {
        role: LocatorField::Known(role.to_owned()),
        name: name_field(name),
        description: LocatorField::Absent,
        value: LocatorField::Absent,
        identifiers: identifiers(target),
        states: LocatorField::Known(states),
        ref_evidence: LocatorRefEvidence {
            bounds: LocatorField::Absent,
            available_actions: LocatorField::Known(actions),
            descriptors: Default::default(),
        },
    }
}

#[cfg(test)]
#[path = "node_evidence_tests.rs"]
mod tests;
