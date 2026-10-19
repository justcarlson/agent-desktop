use super::role_numbers::{DIALOG, is_web_document};
use super::state_bits::{SHOWING, StateBits};
use super::{AtspiElement, ObservedRead, WalkedNode};

pub(crate) type Node = WalkedNode<ObservedRead>;

/// A dialog drawn inside its window's tree: a `SHOWING` dialog node below the
/// window, which no listing of top-level windows sees.
pub(crate) struct Nested<'a> {
    pub(crate) element: &'a AtspiElement,
    pub(crate) title: Option<&'a str>,
}

fn role(node: &Node) -> Option<u32> {
    node.payload.node.traits.role.present().copied()
}

fn showing(node: &Node) -> bool {
    node.payload
        .node
        .traits
        .states
        .present()
        .is_some_and(|words| StateBits::new(words).has(SHOWING))
}

fn collect<'a>(node: &'a Node, found: &mut Vec<Nested<'a>>) {
    if node.in_web_document || is_web_document(role(node)) {
        return;
    }
    if role(node) == Some(DIALOG) && showing(node) {
        found.push(Nested {
            element: &node.payload.node.element,
            title: node
                .payload
                .node
                .properties
                .name
                .present()
                .map(String::as_str)
                .filter(|title| !title.is_empty()),
        });
        return;
    }
    for child in &node.children {
        collect(child, found);
    }
}

/// The outermost showing dialogs below a window root, in document order. The
/// root itself is never one: a top-level dialog is a window, and listed as
/// such. A dialog that does not show is not found, whether its node is gone
/// or stays in the tree.
pub(crate) fn find(window: &Node) -> Vec<Nested<'_>> {
    let mut found = Vec::new();
    for child in &window.children {
        collect(child, &mut found);
    }
    found
}

/// Whether the walk left any subtree unread, so a dialog may sit where it did
/// not reach.
pub(crate) fn is_cut_short(node: &Node) -> bool {
    node.truncated || node.children.iter().any(is_cut_short)
}

#[cfg(test)]
#[path = "nested_dialog_tests.rs"]
mod tests;
