#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::SnapshotSurface;

use super::detect::ShellEnvironment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellKind {
    Dock,
    Tray,
    MessageList,
    SystemMenu,
}

pub(crate) fn kind_of(_surface: SnapshotSurface) -> Option<ShellKind> {
    None
}

pub(crate) fn advertised(_env: &ShellEnvironment) -> Vec<SnapshotSurface> {
    Vec::new()
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
