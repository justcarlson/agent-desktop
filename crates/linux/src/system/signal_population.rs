#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::AppInfo;

use super::listed_window::ListedWindow;
use super::registry_app::RegistryApp;

pub(crate) fn apps_of(_windows: &[ListedWindow], _registry_apps: &[RegistryApp]) -> Vec<AppInfo> {
    Vec::new()
}

#[cfg(test)]
#[path = "signal_population_tests.rs"]
mod tests;
