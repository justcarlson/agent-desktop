use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::anchor::Anchor;
use super::identity::ShellRoot;
use super::light::{Light, LiveNodes, Nodes};
use super::located::Located;
use super::search::{Query, find_all};
use crate::system::SharedBus;
use crate::tree::AtspiElement;
use crate::tree::role_numbers::{LIST, NOTIFICATION};

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
    if anchor != Anchor::Landmark(Landmark::MessageList) {
        return Err(unproven(anchor));
    }
    let paths = find_all(
        nodes,
        &start,
        deadline,
        &Query {
            prune: &|_| false,
            candidate: &|node| node.role == NOTIFICATION,
            accept: &|_, _| true,
        },
    )?;
    let mut containers: Vec<Light> = Vec::new();
    for path in paths {
        let container = path
            .iter()
            .rev()
            .skip(1)
            .take_while(|node| node.role != NOTIFICATION)
            .find(|node| node.role == LIST);
        if let Some(container) = container {
            if !containers
                .iter()
                .any(|other| other.element == container.element)
            {
                containers.push(container.clone());
            }
        }
    }
    let container = match containers.len() {
        0 => return Err(unproven(anchor)),
        1 => containers.remove(0),
        _ => {
            return Err(AdapterError::ambiguous_target(
                "Several shell containers have the requested role chain",
            ));
        }
    };
    Ok(Located {
        anchor: container.clone(),
        container,
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
