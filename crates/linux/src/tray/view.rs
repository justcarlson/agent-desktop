use agent_desktop_core::{AdapterError, Deadline, ObservedSubtree};

use super::item_props::{self, ItemProps};
use super::menu_layout::{self, Layout};
use super::node_evidence::{ITEM_ROLE, fresh, item_states};
use super::{TrayTarget, call, capability, introspect, menu_nodes, registration, watcher};

pub(crate) struct TrayItem {
    pub(crate) target: TrayTarget,
    pub(crate) props: ItemProps,
    pub(crate) methods: Vec<String>,
    pub(crate) layout: Option<Layout>,
}

/// One read of every item the watcher lists, with identity, capabilities and
/// menu, without inferring a native widget from its title.
pub(crate) struct TrayView {
    pub(crate) items: Vec<TrayItem>,
}

fn item_target(owner: String, object_path: String) -> TrayTarget {
    TrayTarget {
        owner,
        object_path,
        menu_path: None,
        entry_id: None,
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
    /// Watcher identities and menus, with no inferred native placement.
    pub(crate) fn subtrees(&self) -> Vec<ObservedSubtree> {
        self.items
            .iter()
            .map(|item| {
                let evidence = fresh(
                    ITEM_ROLE,
                    &item.props.title,
                    &item.target,
                    item_states(item.props.passive, Vec::new()),
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
