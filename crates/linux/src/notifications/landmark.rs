use super::world::{Node, Role};

const EMPTY_LANDMARK: &str = "No Notifications";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerState {
    Entries,
    Empty,
    Unrecognised,
}

pub(crate) fn labels_of(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .filter(|node| node.role == Role::Label && !node.name.trim().is_empty())
        .map(|node| node.name.clone())
        .collect()
}

/// Entries mean a list. With no entries, the empty landmark or the footer
/// button that sits outside every entry means an empty list: the shell does
/// not always expose the landmark text, and the footer button carries no
/// language. Anything else is a container this reader does not recognise.
pub(crate) fn classify(labels: &[String], entries: usize, footer_buttons: usize) -> ContainerState {
    if entries > 0 {
        return ContainerState::Entries;
    }
    if footer_buttons > 0
        || labels
            .iter()
            .any(|label| label.trim().eq_ignore_ascii_case(EMPTY_LANDMARK))
    {
        return ContainerState::Empty;
    }
    ContainerState::Unrecognised
}

#[cfg(test)]
#[path = "landmark_tests.rs"]
mod tests;
