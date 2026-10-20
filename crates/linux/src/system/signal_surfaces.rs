use agent_desktop_core::{SnapshotSurface, SurfaceSignal};

use super::listed_window::ListedWindow;
use super::nested_scan::NestedScan;
use super::registry_app::RegistryApp;

pub(crate) const SIGNAL_SURFACE_KINDS: [SnapshotSurface; 1] = [SnapshotSurface::Sheet];

fn sheet_signal(app: &RegistryApp, id: String, title: Option<String>) -> SurfaceSignal {
    SurfaceSignal {
        kind: SIGNAL_SURFACE_KINDS[0],
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
            Some(sheet_signal(
                app,
                window.info.id.clone(),
                Some(window.info.title.clone()).filter(|title| !title.is_empty()),
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
            Some(sheet_signal(app, nested.id.clone(), nested.title.clone()))
        })
        .collect()
}

#[cfg(test)]
#[path = "signal_surfaces_tests.rs"]
mod tests;
