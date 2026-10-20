use agent_desktop_core::{SnapshotSurface, SurfaceSignal};

use super::listed_window::ListedWindow;
use super::nested_scan::NestedScan;
use super::registry_app::RegistryApp;

pub(crate) const SIGNAL_SURFACE_KINDS: [SnapshotSurface; 2] =
    [SnapshotSurface::Sheet, SnapshotSurface::Menu];

fn surface_signal(
    app: &RegistryApp,
    kind: SnapshotSurface,
    id: String,
    title: Option<String>,
) -> SurfaceSignal {
    SurfaceSignal {
        kind,
        app: app.name.clone(),
        pid: app.root.pid().into(),
        process_instance: app.facts.instance.clone(),
        id,
        title,
    }
}

/// One `Sheet` signal for every listed window the shared sheet predicate
/// accepts. The window stays in the window list too, so a dialog is one
/// `window-opened` and one `surface-appeared`, keyed on its id and never on
/// its name.
pub(crate) fn sheet_signals(
    windows: &[ListedWindow],
    registry_apps: &[RegistryApp],
) -> Vec<SurfaceSignal> {
    windows
        .iter()
        .filter(|window| window.sheet)
        .filter_map(|window| {
            let app = registry_apps.get(window.app)?;
            Some(surface_signal(
                app,
                SnapshotSurface::Sheet,
                window.info.id.clone(),
                Some(window.info.title.clone()).filter(|title| !title.is_empty()),
            ))
        })
        .collect()
}

/// One `Menu` signal for every open menu a window is or holds, keyed on the
/// window's own id for a menu that is a window and on the popup's address for
/// a menu inside one. A menu that is itself a window is also one
/// `window-opened` and one `window-closed`; the surface signal is how its open
/// and close announce what the window was.
pub(crate) fn menu_signals(
    scan: &NestedScan,
    windows: &[ListedWindow],
    registry_apps: &[RegistryApp],
) -> Vec<SurfaceSignal> {
    scan.menus
        .iter()
        .filter_map(|menu| {
            let app = registry_apps.get(windows.get(menu.window)?.app)?;
            Some(surface_signal(
                app,
                SnapshotSurface::Menu,
                menu.id.clone(),
                menu.title.clone(),
            ))
        })
        .collect()
}

/// One `Sheet` signal for every dialog drawn inside a window, keyed on the
/// dialog's own address. It is announced while it shows and gone when it does
/// not, whether its node stays in the tree or leaves it.
pub(crate) fn nested_signals(
    scan: &NestedScan,
    windows: &[ListedWindow],
    registry_apps: &[RegistryApp],
) -> Vec<SurfaceSignal> {
    scan.found
        .iter()
        .filter_map(|nested| {
            let app = registry_apps.get(windows.get(nested.window)?.app)?;
            Some(surface_signal(
                app,
                SnapshotSurface::Sheet,
                nested.id.clone(),
                nested.title.clone(),
            ))
        })
        .collect()
}

#[cfg(test)]
#[path = "signal_surfaces_tests.rs"]
mod tests;
