#![expect(dead_code, reason = "stub the owning lane replaces")]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerState {
    Entries,
    Empty,
    Unrecognised,
}

pub(crate) fn classify(_labels: &[String], _entries: usize) -> ContainerState {
    ContainerState::Unrecognised
}

#[cfg(test)]
#[path = "landmark_tests.rs"]
mod tests;
