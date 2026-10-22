use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::anchor::Anchor;
use super::identity::ShellRoot;
use super::light::{Light, LiveNodes, Nodes};
use super::located::Located;
use super::search::{Query, find_all};
use crate::system::SharedBus;
use crate::tree::AtspiElement;
use crate::tree::role_numbers::{LABEL, PUSH_BUTTON, TOGGLE_BUTTON};

pub(crate) const MENU: u32 = 33;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Landmark {
    MessageList,
    TrayPanel,
    DockPanel,
    SystemMenu,
    DateMenu,
}

fn unproven(anchor: Anchor) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        "The shell exposes no structural identity for this container or its opener",
    )
    .with_suggestion("Use an app surface or a watcher-backed tray item; labels and positions cannot prove shell popup identity")
    .with_details(json!({ "kind": "shell_container_unproven", "landmark": format!("{anchor:?}") }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn locate_in<N: Nodes>(
    nodes: &N,
    root: &ShellRoot,
    anchor: Anchor,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let start = AtspiElement::new(&root.bus_name, &root.path, root.pid);
    let root_node = nodes.read(&start)?;
    if anchor == Anchor::Landmark(Landmark::TrayPanel) {
        return Ok(Located {
            anchor: root_node.clone(),
            container: root_node,
        });
    }
    match anchor {
        Anchor::Landmark(Landmark::MessageList) => locate_list(nodes, &start, deadline),
        Anchor::Landmark(Landmark::DateMenu) => locate_date_menu(nodes, &start, deadline),
        _ => Err(unproven(anchor)),
    }
}

/// The list's footer is a row of a label, a toggle and a push button, in that
/// order, and the row sits two levels below the container holding the
/// entries. The roles and the row's shape carry the identity, not any text.
fn is_footer_control<N: Nodes>(nodes: &N, above: &[Light], light: &Light) -> bool {
    let Some(row) = above.last() else {
        return false;
    };
    let [label, toggle, button] = row.children.as_slice() else {
        return false;
    };
    button == &light.element
        && nodes.read(label).is_ok_and(|node| node.role == LABEL)
        && nodes
            .read(toggle)
            .is_ok_and(|node| node.role == TOGGLE_BUTTON)
}

fn single(mut found: Vec<Light>, anchor: Anchor, many: &str) -> Result<Light, AdapterError> {
    match found.len() {
        0 => Err(unproven(anchor)),
        1 => Ok(found.remove(0)),
        _ => Err(AdapterError::ambiguous_target(many)),
    }
}

fn locate_list<N: Nodes>(
    nodes: &N,
    start: &AtspiElement,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let paths = find_all(
        nodes,
        start,
        deadline,
        &Query {
            prune: &|_| false,
            candidate: &|node| node.role == PUSH_BUTTON,
            accept: &|above, node| is_footer_control(nodes, above, node),
        },
    )?;
    let mut containers: Vec<Light> = Vec::new();
    for path in paths {
        let [.., container, _, _] = path.as_slice() else {
            continue;
        };
        if !containers
            .iter()
            .any(|other| other.element == container.element)
        {
            containers.push(container.clone());
        }
    }
    let container = single(
        containers,
        Anchor::Landmark(Landmark::MessageList),
        "Several shell containers have the requested role chain",
    )?;
    Ok(Located {
        anchor: container.clone(),
        container,
    })
}

/// The date menu is the one showing menu in the shell with no accessible
/// name; every other top bar menu names its indicator.
fn locate_date_menu<N: Nodes>(
    nodes: &N,
    start: &AtspiElement,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let paths = find_all(
        nodes,
        start,
        deadline,
        &Query {
            prune: &|_| false,
            candidate: &|node| {
                node.role == MENU && node.name.is_empty() && node.showing && node.placed().is_some()
            },
            accept: &|_, _| true,
        },
    )?;
    let found = paths
        .into_iter()
        .filter_map(|path| path.last().cloned())
        .collect();
    let menu = single(
        found,
        Anchor::Landmark(Landmark::DateMenu),
        "Several unnamed menus are showing in the shell",
    )?;
    Ok(Located {
        anchor: menu.clone(),
        container: menu,
    })
}

pub(crate) fn locate_anchor(
    bus: &SharedBus,
    root: &ShellRoot,
    anchor: Anchor,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let nodes = LiveNodes::new(bus.get(deadline)?.inner().clone(), deadline);
    locate_in(&nodes, root, anchor, deadline)
}

#[cfg(test)]
#[path = "container_tests.rs"]
mod tests;
