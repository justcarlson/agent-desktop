use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

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

/// The answer for a scoped listing: the windows, unless a selected
/// application is stopped. A stopped application cannot be read, so its
/// windows are missing from the list; answering with the other selected
/// applications' windows alone would hide it, and a same-name twin that is
/// stopped beside a running one would vanish without a trace.
pub(crate) fn settle_scoped<T>(
    windows: Vec<T>,
    stopped: Option<AdapterError>,
) -> Result<Vec<T>, AdapterError> {
    match stopped {
        Some(error) => Err(error),
        None => Ok(windows),
    }
}

/// The windows of the selected applications for a request that names its
/// target. When a selected application is stopped that is `APP_UNRESPONSIVE`
/// for it, whether or not another selected application has windows, rather
/// than a list that reads as complete. An empty list from a responsive
/// application stays empty.
pub(crate) fn scoped_windows_of(
    bus: &SharedBus,
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    deadline: Deadline,
) -> Result<Vec<ListedWindow>, AdapterError> {
    let windows = windows_of(bus, registry, &select, deadline)?;
    settle_scoped(
        windows,
        stopped_target(registry, &select, process_facts::is_stopped),
    )
}

/// A named target that no registry application and no running process
/// matches is `APP_NOT_FOUND`, the answer `snapshot` and `list-surfaces` give
/// for the same name. A name that only a bus-less process matches stays an
/// empty list, and an application that is registered but shows no window
/// does too.
pub(crate) fn require_known_target(
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    query: &str,
    running: impl FnOnce(&str) -> Option<(u32, String)>,
) -> Result<(), AdapterError> {
    if registry.apps.iter().any(select) || running(query).is_some() {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::AppNotFound,
        format!("No running application matches '{query}'"),
    )
    .with_suggestion("Run 'list-apps' to see running applications, or launch the app first."))
}

#[cfg(test)]
#[path = "unresponsive_tests.rs"]
mod tests;
