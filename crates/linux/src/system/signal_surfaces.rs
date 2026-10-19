use agent_desktop_core::{SnapshotSurface, SurfaceSignal};

use super::listed_window::ListedWindow;
use super::registry_app::RegistryApp;

pub(crate) const SIGNAL_SURFACE_KINDS: [SnapshotSurface; 1] = [SnapshotSurface::Sheet];

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
            Some(SurfaceSignal {
                kind: SnapshotSurface::Sheet,
                app: app.name.clone(),
                pid: app.root.pid().into(),
                process_instance: app.facts.instance.clone(),
                id: window.info.id.clone(),
                title: Some(window.info.title.clone()).filter(|title| !title.is_empty()),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "signal_surfaces_tests.rs"]
mod tests;
