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

/// Entries mean a list; the empty landmark with no entries means an empty
/// list; anything else is a container this reader does not recognise.
pub(crate) fn classify(labels: &[String], entries: usize) -> ContainerState {
    if entries > 0 {
        return ContainerState::Entries;
    }
    if labels
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
