use agent_desktop_core::{AdapterError, Deadline};

use super::role_numbers::is_bulk_container;
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

const BULK_CHILDREN: usize = 48;

/// Walks up to `max_nodes` nodes below `root` and reads at most a few dozen
/// children of any one list, table or tree. The popup of a menu is never one of the many
/// rows of a list or the many cells of a table, so a container with
/// thousands of rows does not use up the budget before the walk reaches the
/// menu. A container cut this way is reported by [`is_pruned`].
pub(crate) fn scan_walk_pruned(
    bus: &SharedBus,
    root: &AtspiElement,
    max_nodes: usize,
    deadline: Deadline,
) -> Result<WalkedNode<ObservedRead>, AdapterError> {
    let limits = WalkLimits {
        max_nodes,
        bulk_children: BULK_CHILDREN,
        ..WalkLimits::default()
    };
    walk_tree(bus, root, &limits, deadline).map(|walk| walk.root)
}

/// Whether a node is a list, table or tree whose rows were cut at the child
/// limit on purpose, as opposed to a read that stopped short.
pub(crate) fn is_pruned(node: &WalkedNode<ObservedRead>) -> bool {
    node.truncated
        && is_bulk_container(node.payload.node.traits.role.present().copied())
        && node.children.len() == BULK_CHILDREN
        && node
            .child_count
            .is_some_and(|total| usize::try_from(total).is_ok_and(|total| total > BULK_CHILDREN))
}

/// Whether any part of the walk stopped short other than the rows pruned on
/// purpose.
pub(crate) fn is_cut_short_beyond_pruning(node: &WalkedNode<ObservedRead>) -> bool {
    (node.truncated && !is_pruned(node)) || node.children.iter().any(is_cut_short_beyond_pruning)
}

#[cfg(test)]
#[path = "observe_menu_tests.rs"]
mod tests;
