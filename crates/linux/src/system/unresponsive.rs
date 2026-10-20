use agent_desktop_core::{AdapterError, AppInfo, Deadline, DeliverySemantics, ErrorCode};

use super::SharedBus;
use super::listed_window::ListedWindow;
use super::proc_stat;
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
    first_stopped(registry, select, is_stopped).map(|app| AdapterError::app_unresponsive(&app.name))
}

/// The application `stopped_target` names, for a caller that needs more of it
/// than its name.
pub(crate) fn first_stopped(
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    is_stopped: impl Fn(u32) -> bool,
) -> Option<&RegistryApp> {
    registry
        .apps
        .iter()
        .filter(|app| is_listable(app) && select(app) && !app.answered)
        .find(|app| is_stopped(app.root.pid()))
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

/// The applications a name matched, unless several matched and one of them is
/// stopped. A stopped application cannot be told from its running same-name
/// twin by anything it answers, so the name is not ambiguous: the stopped one
/// is the thing to tell the caller, as a scoped window listing tells it. A
/// single match is returned whatever its state, so a command on a stopped
/// application still reaches the application.
pub(crate) fn settle_matches(
    matched: Vec<AppInfo>,
    stopped: impl FnOnce() -> Option<AdapterError>,
) -> Result<Vec<AppInfo>, AdapterError> {
    if matched.len() < 2 {
        return Ok(matched);
    }
    match stopped() {
        Some(error) => Err(error),
        None => Ok(matched),
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
    settle_scoped(windows, stopped_target(registry, &select, proc_stat::is_stopped))
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
    .with_suggestion("Run 'list-apps' to see running applications, or launch the app first.")
    .with_disposition(DeliverySemantics::not_delivered()))
}

#[cfg(test)]
#[path = "unresponsive_tests.rs"]
mod tests;
