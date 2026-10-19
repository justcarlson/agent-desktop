#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{SnapshotSurface, SurfaceSignal};

use super::listed_window::ListedWindow;
use super::registry_app::RegistryApp;

pub(crate) const SIGNAL_SURFACE_KINDS: [SnapshotSurface; 1] = [SnapshotSurface::Sheet];

pub(crate) fn sheet_signals(
    _windows: &[ListedWindow],
    _registry_apps: &[RegistryApp],
) -> Vec<SurfaceSignal> {
    Vec::new()
}

#[cfg(test)]
#[path = "signal_surfaces_tests.rs"]
mod tests;
