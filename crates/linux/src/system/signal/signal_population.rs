use std::collections::BTreeSet;

use agent_desktop_core::AppInfo;

use crate::system::app::app_ops::{info, one_entry_per_process};
use crate::system::registry::listed_window::ListedWindow;
use crate::system::registry::registry_app::RegistryApp;

/// The applications that own at least one listed window, one entry per
/// process. A helper that answers on the registry with no frame never
/// appears, and a window that opens under a running process adds no app.
pub(crate) fn apps_of(windows: &[ListedWindow], registry_apps: &[RegistryApp]) -> Vec<AppInfo> {
    let owners: BTreeSet<usize> = windows.iter().map(|window| window.app).collect();
    one_entry_per_process(
        owners
            .into_iter()
            .filter_map(|index| registry_apps.get(index))
            .map(|app| info(app, true))
            .collect(),
    )
}

#[cfg(test)]
#[path = "signal_population_tests.rs"]
mod tests;
