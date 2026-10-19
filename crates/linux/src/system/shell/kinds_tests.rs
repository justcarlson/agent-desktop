use agent_desktop_core::SnapshotSurface;

use crate::system::shell::detect::{Fact, ShellEnvironment};
use crate::system::shell::kinds::{ShellKind, advertised, kind_of, surface_of};

fn environment(tree: Fact) -> ShellEnvironment {
    ShellEnvironment {
        daemon: Fact::Present,
        tree,
        watcher: Fact::Present,
        server: Some("gnome-shell".to_owned()),
    }
}

#[test]
fn each_shell_surface_maps_to_one_kind_and_back() {
    let pairs = [
        (SnapshotSurface::Taskbar, ShellKind::Dock),
        (SnapshotSurface::SystemTray, ShellKind::Tray),
        (SnapshotSurface::ActionCenter, ShellKind::MessageList),
        (SnapshotSurface::QuickSettings, ShellKind::SystemMenu),
    ];
    for (surface, kind) in pairs {
        assert_eq!(kind_of(surface), Some(kind));
        assert_eq!(surface_of(kind), surface);
    }
}

#[test]
fn window_menu_and_start_menu_are_not_shell_kinds() {
    for surface in [
        SnapshotSurface::Window,
        SnapshotSurface::Menu,
        SnapshotSurface::StartMenu,
        SnapshotSurface::Sheet,
    ] {
        assert_eq!(kind_of(surface), None);
    }
}

#[test]
fn a_readable_shell_tree_advertises_the_measured_kinds_and_not_the_dock() {
    let surfaces = advertised(&environment(Fact::Present));
    assert_eq!(
        surfaces,
        [
            SnapshotSurface::SystemTray,
            SnapshotSurface::ActionCenter,
            SnapshotSurface::QuickSettings,
        ]
    );
}

#[test]
fn an_absent_or_unreadable_tree_advertises_nothing_whatever_the_daemon_says() {
    for tree in [Fact::Absent("x".into()), Fact::Unreadable("y".into())] {
        assert!(advertised(&environment(tree)).is_empty());
    }
}
