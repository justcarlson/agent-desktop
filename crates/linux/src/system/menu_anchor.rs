use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity, WindowInfo};

use super::SharedBus;
use super::liveness::{Liveness, liveness};
use super::menu_surface::{popups_at, popups_of};
use super::registry::read_registry_for;
use super::registry_app::RegistryApp;
use super::window_ops::windows_of;
use crate::tree::AtspiElement;

enum Target {
    Window(WindowInfo),
    Root(AtspiElement),
}

fn owned_by(pid: u32) -> impl Fn(&RegistryApp) -> bool {
    move |app| app.root.pid() == pid
}

/// What to read for a process: its windows, each of which also reads the
/// top-level popups of the application. A process with no window is read from
/// its application roots, which is where a popup frame that is not a window
/// lives.
fn targets(roots: Vec<AtspiElement>, windows: Vec<WindowInfo>) -> Vec<Target> {
    if windows.is_empty() {
        roots.into_iter().map(Target::Root).collect()
    } else {
        windows.into_iter().map(Target::Window).collect()
    }
}

fn answer(
    state: Liveness,
    collect: impl FnOnce() -> Result<Vec<Target>, AdapterError>,
    mut read: impl FnMut(&Target) -> Result<bool, AdapterError>,
) -> Result<bool, AdapterError> {
    if state != Liveness::Alive {
        return Ok(false);
    }
    for target in collect()? {
        if read(&target)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_target(bus: &SharedBus, target: &Target, deadline: Deadline) -> Result<bool, AdapterError> {
    let popups = match target {
        Target::Window(window) => popups_of(bus, window, false, deadline)?,
        Target::Root(root) => popups_at(bus, root, false, deadline)?,
    };
    Ok(!popups.is_empty())
}

fn collect_targets(
    bus: &SharedBus,
    pid: u32,
    deadline: Deadline,
) -> Result<Vec<Target>, AdapterError> {
    let registry = read_registry_for(bus, Some(pid), deadline)?;
    let windows = windows_of(bus, &registry, owned_by(pid), deadline)?;
    let roots = registry
        .apps
        .iter()
        .filter(|app| owned_by(pid)(app))
        .map(|app| app.root.clone())
        .collect();
    Ok(targets(
        roots,
        windows.into_iter().map(|window| window.info).collect(),
    ))
}

/// Whether the process has an open menu now. A process that is not alive is
/// read for nothing: its tree answers slowly or not at all, and the wait loop
/// turns that state into its own error.
pub(crate) fn menu_open(
    bus: &SharedBus,
    process: &ProcessIdentity,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let pid = process.pid.get();
    answer(
        liveness(pid, &process.instance),
        || collect_targets(bus, pid, deadline),
        |target| read_target(bus, target, deadline),
    )
}

#[cfg(test)]
#[path = "menu_anchor_tests.rs"]
mod tests;
