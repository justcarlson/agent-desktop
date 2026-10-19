use agent_desktop_core::state;
use agent_desktop_core::{AdapterError, Deadline, LocatorEvidence, LocatorField, ObservedSubtree};

use super::item_props::{self, ItemProps};
use super::menu_layout::{self, Layout};
use super::node_evidence::{
    ITEM_ROLE, UNBOUND_DESCRIPTION, fresh, identifiers, item_states, name_field,
};
use super::{
    TrayTarget, call, capability, introspect, menu_nodes, panel_correlate, registration, watcher,
};

pub(crate) struct TrayItem {
    pub(crate) target: TrayTarget,
    pub(crate) props: ItemProps,
    pub(crate) methods: Vec<String>,
    pub(crate) layout: Option<Layout>,
}

/// One read of every item the watcher lists, with identity, capabilities and
/// menu, ready to be attached to the panel nodes that show them.
pub(crate) struct TrayView {
    pub(crate) items: Vec<TrayItem>,
}

fn item_target(owner: String, object_path: String) -> TrayTarget {
    TrayTarget {
        owner,
        object_path,
        menu_path: None,
        entry_id: None,
        revision: None,
        label_path: Vec::new(),
    }
}

fn describe(
    connection: &zbus::Connection,
    target: TrayTarget,
    deadline: Deadline,
) -> Result<TrayItem, AdapterError> {
    let props = item_props::read(connection, &target.owner, &target.object_path, deadline)?;
    let methods = call::introspect(connection, &target.owner, &target.object_path, deadline)
        .map(|xml| introspect::item_methods(&xml))
        .unwrap_or_default();
    let layout = props
        .menu
        .as_deref()
        .and_then(|menu| menu_layout::read(connection, &target.owner, menu, deadline).ok());
    Ok(TrayItem {
        target,
        props,
        methods,
        layout,
    })
}

pub(crate) fn read(deadline: Deadline) -> Result<TrayView, AdapterError> {
    let listed = watcher::registered_items(deadline)?;
    let connection = call::session(deadline)?;
    let mut items: Vec<TrayItem> = Vec::new();
    for registered in registration::parse_all(&listed) {
        let Some(owner) = call::unique_owner(&connection, &registered.bus_name, deadline)? else {
            continue;
        };
        let known = items.iter().any(|item| {
            item.target.owner == owner && item.target.object_path == registered.object_path
        });
        if known {
            continue;
        }
        if let Ok(item) = describe(
            &connection,
            item_target(owner, registered.object_path),
            deadline,
        ) {
            items.push(item);
        }
    }
    Ok(TrayView { items })
}

fn apply(evidence: &mut LocatorEvidence, item: &TrayItem) {
    let states = evidence.states.known().cloned().unwrap_or_default();
    evidence.role = LocatorField::Known(ITEM_ROLE.to_owned());
    evidence.name = name_field(&item.props.title);
    evidence.description = LocatorField::Absent;
    evidence.value = LocatorField::Absent;
    evidence.identifiers = identifiers(&item.target);
    evidence.states = LocatorField::Known(item_states(item.props.passive, states));
    evidence.ref_evidence.available_actions =
        LocatorField::Known(capability::actions(&item.methods));
}

fn menu_children(item: &TrayItem, first_index: usize) -> Vec<ObservedSubtree> {
    let (Some(layout), Some(menu)) = (&item.layout, &item.props.menu) else {
        return Vec::new();
    };
    menu_nodes::subtrees(&item.target, menu, layout)
        .into_iter()
        .enumerate()
        .map(|(offset, node)| node.with_source_child_index(first_index + offset))
        .collect()
}

impl TrayView {
    pub(crate) fn bind(&self, panel_titles: &[String]) -> Vec<Option<usize>> {
        let titles: Vec<String> = self
            .items
            .iter()
            .map(|item| item.props.title.clone())
            .collect();
        panel_correlate::pair(&titles, panel_titles)
    }

    /// Gives a panel node the identity, hidden state and capabilities of the
    /// item it shows, and returns the item's menu entries to append as its
    /// children, numbered from `first_child_index`.
    pub(crate) fn decorate(
        &self,
        item: usize,
        evidence: &mut LocatorEvidence,
        first_child_index: usize,
    ) -> Vec<ObservedSubtree> {
        let Some(item) = self.items.get(item) else {
            return Vec::new();
        };
        apply(evidence, item);
        menu_children(item, first_child_index)
    }

    /// Marks a panel node that no item could be paired with: a title the
    /// panel or the watcher repeats, or an item that is not on the watcher.
    pub(crate) fn mark_unbound(evidence: &mut LocatorEvidence) {
        evidence.description = LocatorField::Known(UNBOUND_DESCRIPTION.to_owned());
    }

    /// The items no panel node was paired with, as nodes of their own, so a
    /// duplicate-title item stays reachable by its D-Bus identity.
    pub(crate) fn unpaired(&self, bound: &[Option<usize>]) -> Vec<ObservedSubtree> {
        self.items
            .iter()
            .enumerate()
            .filter(|(index, _)| !bound.contains(&Some(*index)))
            .map(|(_, item)| {
                let evidence = fresh(
                    ITEM_ROLE,
                    &item.props.title,
                    &item.target,
                    item_states(item.props.passive, vec![state::OFFSCREEN.to_owned()]),
                    capability::actions(&item.methods),
                );
                ObservedSubtree::new(evidence, menu_children(item, 0), true, None)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
