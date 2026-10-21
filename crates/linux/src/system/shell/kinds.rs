use agent_desktop_core::SnapshotSurface;

use super::detect::Fact;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ShellKind {
    Dock,
    Tray,
    MessageList,
    SystemMenu,
}

pub(crate) fn kind_of(surface: SnapshotSurface) -> Option<ShellKind> {
    match surface {
        SnapshotSurface::Taskbar => Some(ShellKind::Dock),
        SnapshotSurface::SystemTray => Some(ShellKind::Tray),
        SnapshotSurface::ActionCenter => Some(ShellKind::MessageList),
        SnapshotSurface::QuickSettings => Some(ShellKind::SystemMenu),
        _ => None,
    }
}

pub(crate) fn surface_of(kind: ShellKind) -> SnapshotSurface {
    match kind {
        ShellKind::Dock => SnapshotSurface::Taskbar,
        ShellKind::Tray => SnapshotSurface::SystemTray,
        ShellKind::MessageList => SnapshotSurface::ActionCenter,
        ShellKind::SystemMenu => SnapshotSurface::QuickSettings,
    }
}

pub(crate) fn title_of(kind: ShellKind) -> &'static str {
    match kind {
        ShellKind::Dock => "Dock",
        ShellKind::Tray => "Top bar tray",
        ShellKind::MessageList => "Message list",
        ShellKind::SystemMenu => "System menu",
    }
}

const PRESENTED: [ShellKind; 3] = [
    ShellKind::Tray,
    ShellKind::MessageList,
    ShellKind::SystemMenu,
];

pub(crate) fn is_presented(kind: ShellKind) -> bool {
    PRESENTED.contains(&kind)
}

/// The kinds a session can snapshot: those whose container lives in the shell
/// tree, which is the only fact they depend on.
pub(crate) fn advertised(tree: &Fact) -> Vec<SnapshotSurface> {
    match tree {
        Fact::Present => PRESENTED.iter().copied().map(surface_of).collect(),
        Fact::Absent(_) | Fact::Unreadable(_) => Vec::new(),
    }
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
