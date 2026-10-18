use agent_desktop_core::{AdapterError, Deadline};

use super::SharedBus;
use super::listed_window::ListedWindow;
use super::process_facts;
use super::registry::Registry;
use super::registry_app::RegistryApp;
use super::window_ops::{is_listable, windows_of};

/// The first selected application that is stopped: its own root did not
/// answer and the process is in a stopped state, while the bus daemon still
/// reports its process id. An application that merely answers slowly, or has
/// no windows, is not stopped.
pub(crate) fn stopped_target(
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    is_stopped: impl Fn(u32) -> bool,
) -> Option<AdapterError> {
    registry
        .apps
        .iter()
        .filter(|app| is_listable(app) && select(app) && !app.answered)
        .find(|app| is_stopped(app.root.pid()))
        .map(|app| AdapterError::app_unresponsive(&app.name))
}

/// The windows of the selected applications for a request that names its
/// target. When the target yields none and it is stopped, that is
/// `APP_UNRESPONSIVE` for the target rather than an empty list that reads as
/// "no window". An empty list from a responsive application stays empty.
pub(crate) fn scoped_windows_of(
    bus: &SharedBus,
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    deadline: Deadline,
) -> Result<Vec<ListedWindow>, AdapterError> {
    let windows = windows_of(bus, registry, &select, deadline)?;
    if windows.is_empty()
        && let Some(error) = stopped_target(registry, &select, process_facts::is_stopped)
    {
        return Err(error);
    }
    Ok(windows)
}

#[cfg(test)]
#[path = "unresponsive_tests.rs"]
mod tests;
