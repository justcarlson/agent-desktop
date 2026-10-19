#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline};

use crate::system::SharedBus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fact {
    Present,
    Absent(String),
    Unreadable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellEnvironment {
    pub(crate) daemon: Fact,
    pub(crate) tree: Fact,
    pub(crate) watcher: Fact,
    pub(crate) server: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Need {
    Daemon,
    Tree,
    Watcher,
}

fn unavailable() -> Fact {
    Fact::Unreadable("shell detection is not available".to_owned())
}

pub(crate) fn detect(_bus: &SharedBus, _deadline: Deadline) -> ShellEnvironment {
    ShellEnvironment {
        daemon: unavailable(),
        tree: unavailable(),
        watcher: unavailable(),
        server: None,
    }
}

pub(crate) fn require(_env: &ShellEnvironment, _need: Need) -> Result<(), AdapterError> {
    Err(AdapterError::not_supported("shell detection"))
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
