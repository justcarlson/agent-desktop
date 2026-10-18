use std::collections::BTreeMap;

use super::{AtspiElement, Extents, Field, NodeProperties, NodeTraits};

/// Bus name and object path of a child as `GetChildren` reports it.
pub(crate) type ChildAddress = (String, String);

/// Everything read for one node, each piece with its own outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawNode {
    pub(crate) element: AtspiElement,
    pub(crate) properties: NodeProperties,
    pub(crate) traits: NodeTraits,
    pub(crate) attributes: Field<BTreeMap<String, String>>,
    pub(crate) extents: Field<Extents>,
    pub(crate) actions: Field<Vec<String>>,
    pub(crate) children: Field<Vec<ChildAddress>>,
}

impl RawNode {
    /// A node whose every read failed, kept in place so the walk can go on.
    pub(crate) fn unreadable(element: AtspiElement, detail: &str) -> Self {
        Self {
            element,
            properties: NodeProperties::failed(detail),
            traits: NodeTraits::failed(detail),
            attributes: Field::Failed(detail.to_owned()),
            extents: Field::Failed(detail.to_owned()),
            actions: Field::Failed(detail.to_owned()),
            children: Field::Failed(detail.to_owned()),
        }
    }

    /// The number of fields whose read failed.
    pub(crate) fn failed_reads(&self) -> usize {
        [
            self.properties.name.is_failed(),
            self.properties.description.is_failed(),
            self.properties.child_count.is_failed(),
            self.properties.accessible_id.is_failed(),
            self.traits.role.is_failed(),
            self.traits.role_name.is_failed(),
            self.traits.states.is_failed(),
            self.traits.interfaces.is_failed(),
            self.attributes.is_failed(),
            self.extents.is_failed(),
            self.actions.is_failed(),
            self.children.is_failed(),
        ]
        .into_iter()
        .filter(|failed| *failed)
        .count()
    }
}
