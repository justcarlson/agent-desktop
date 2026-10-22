use agent_desktop_core::NotificationInfo;

use super::world::{Node, Role};
use crate::tree::{AtspiElement, Extents};

const OFF_SCREEN_FLOOR: i32 = -1_000_000;

#[derive(Debug, Clone)]
pub(crate) struct Control {
    pub(crate) name: String,
    pub(crate) extents: Option<Extents>,
    pub(crate) element: AtspiElement,
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) info: NotificationInfo,
    pub(crate) expand: Option<Control>,
    pub(crate) close: Option<Control>,
    pub(crate) actions: Vec<Control>,
}

/// Splits a container reading into the subtree of each notification node and
/// the nodes that sit outside every notification.
pub(crate) fn partition(nodes: &[Node]) -> (Vec<&[Node]>, Vec<&Node>) {
    let mut entries = Vec::new();
    let mut outside = Vec::new();
    let mut position = 0;
    while position < nodes.len() {
        let node = &nodes[position];
        if node.role == Role::Notification {
            let end = nodes[position + 1..]
                .iter()
                .position(|next| next.depth <= node.depth)
                .map_or(nodes.len(), |offset| position + 1 + offset);
            entries.push(&nodes[position..end]);
            position = end;
        } else {
            outside.push(node);
            position += 1;
        }
    }
    (entries, outside)
}

/// Whether the shell has laid the control out. Hidden controls report a
/// sentinel or empty rectangle until their entry is expanded.
pub(crate) fn is_laid_out(extents: Option<Extents>) -> bool {
    extents.is_some_and(|rect| {
        rect.width > 0 && rect.height > 0 && rect.x > OFF_SCREEN_FLOOR && rect.y > OFF_SCREEN_FLOOR
    })
}

/// The index of each node's parent within the slice, from depths alone.
fn parents(subtree: &[Node]) -> Vec<Option<usize>> {
    let mut stack: Vec<usize> = Vec::new();
    subtree
        .iter()
        .enumerate()
        .map(|(position, node)| {
            while stack
                .last()
                .is_some_and(|top| subtree[*top].depth >= node.depth)
            {
                stack.pop();
            }
            let parent = stack.last().copied();
            stack.push(position);
            parent
        })
        .collect()
}

fn inside(parents: &[Option<usize>], mut node: usize, ancestor: usize) -> bool {
    while let Some(parent) = parents[node] {
        if parent == ancestor {
            return true;
        }
        node = parent;
    }
    false
}

fn control_of(node: &Node) -> Control {
    Control {
        name: node.name.clone(),
        extents: node.extents,
        element: node.element.clone(),
    }
}

/// Reads one entry from its subtree. The first node is the notification node.
///
/// The shell lays an entry out as a header, the body and the action row. The
/// header is the one container that holds the unnamed push buttons, so it is
/// found by the controls it carries and not by any text: its first label is
/// the app and later labels are the age. Labels outside the header are the
/// title, then the body. Named push buttons outside the header are the
/// actions. The header's last two unnamed push buttons are expand and close;
/// a header with fewer proves neither.
pub(crate) fn extract(subtree: &[Node], index: usize) -> Option<Entry> {
    let parents = parents(subtree);
    let mut buttons = Vec::new();
    for (position, node) in subtree.iter().enumerate().skip(1) {
        let nested = buttons
            .iter()
            .any(|(outer, _)| inside(&parents, position, *outer));
        if node.role == Role::PushButton && !nested {
            buttons.push((position, node.name.trim().is_empty()));
        }
    }
    let header = buttons
        .iter()
        .find(|(_, unnamed)| *unnamed)
        .and_then(|(position, _)| parents[*position])?;
    let in_header = |position: usize| position == header || inside(&parents, position, header);
    let in_button = |position: usize| {
        buttons
            .iter()
            .any(|(button, _)| *button == position || inside(&parents, position, *button))
    };
    let mut header_labels = Vec::new();
    let mut content_labels = Vec::new();
    for (position, node) in subtree.iter().enumerate().skip(1) {
        if node.role != Role::Label || node.name.trim().is_empty() || in_button(position) {
            continue;
        }
        if in_header(position) {
            header_labels.push(node.name.clone());
        } else {
            content_labels.push(node.name.clone());
        }
    }
    let mut unnamed = Vec::new();
    let mut actions = Vec::new();
    for (position, is_unnamed) in &buttons {
        let control = control_of(&subtree[*position]);
        if !in_header(*position) {
            if !is_unnamed {
                actions.push(control);
            }
        } else if *is_unnamed {
            unnamed.push(control);
        }
    }
    let app_name = header_labels.into_iter().next()?;
    let mut content = content_labels.into_iter();
    let title = content.next()?;
    let body: Vec<String> = content.collect();
    let close = unnamed.pop().filter(|_| !unnamed.is_empty());
    let expand = close.as_ref().and_then(|_| unnamed.pop());
    Some(Entry {
        info: NotificationInfo {
            index,
            app_name,
            title,
            body: (!body.is_empty()).then(|| body.join("\n")),
            actions: actions.iter().map(|action| action.name.clone()).collect(),
        },
        expand,
        close,
        actions,
    })
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
