use agent_desktop_core::{AdapterError, Deadline};

use super::walker::walk_tree;
use super::{AtspiElement, ObservedRead, WalkLimits, WalkedNode, ancestry};
use crate::system::SharedBus;

/// An element that is the root of its own walk: it is its own frame, so the
/// refs below it resolve against it and not against a window.
pub(crate) fn rooted(element: &AtspiElement) -> AtspiElement {
    let mut root = element.clone();
    root.set_ancestry(ancestry::of_root(element));
    root
}

/// Walks up to `max_nodes` nodes below `root`, with the ordinary reads.
pub(crate) fn scan_walk(
    bus: &SharedBus,
    root: &AtspiElement,
    max_nodes: usize,
    deadline: Deadline,
) -> Result<WalkedNode<ObservedRead>, AdapterError> {
    let limits = WalkLimits {
        max_nodes,
        ..WalkLimits::default()
    };
    walk_tree(bus, root, &limits, deadline).map(|walk| walk.root)
}

#[cfg(test)]
#[path = "observe_menu_tests.rs"]
mod tests;
