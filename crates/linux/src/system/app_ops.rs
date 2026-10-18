use std::collections::HashSet;

use agent_desktop_core::{
    AdapterError, AppInfo, AppInventory, AppPresentation, Deadline, ErrorCode,
};
use serde_json::json;

use super::SharedBus;
use super::app_name::name_matches;
use super::proc_scan::find_running;
use super::registry::{Registry, read_registry};
use super::registry_app::RegistryApp;
use super::window_ops::windows_of;

const MISSING_CONNECTION: &str = "The process is running but is not on the accessibility bus. Sandboxed apps (snap, Flatpak) cannot reach it unless the bus is shared. A Chromium or Electron app needs ACCESSIBILITY_ENABLED=1 in its environment and a Qt app needs QT_ACCESSIBILITY=1. The registry was read once, so an app launched a moment ago may need a retry.";

fn info(app: &RegistryApp, foreground: bool) -> AppInfo {
    AppInfo {
        name: app.name.clone(),
        pid: app.root.pid().into(),
        bundle_id: None,
        process_instance: Some(app.facts.instance.clone()),
        presentation: Some(if foreground {
            AppPresentation::Foreground
        } else {
            AppPresentation::Background
        }),
    }
}

fn infos(
    bus: &SharedBus,
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    deadline: Deadline,
) -> Result<Vec<AppInfo>, AdapterError> {
    let windowed: HashSet<usize> = windows_of(bus, registry, &select, deadline)?
        .into_iter()
        .map(|window| window.app)
        .collect();
    Ok(registry
        .apps
        .iter()
        .enumerate()
        .filter(|(_, app)| select(app))
        .map(|(index, app)| info(app, windowed.contains(&index)))
        .collect())
}

pub(crate) fn inventory(bus: &SharedBus, deadline: Deadline) -> Result<AppInventory, AdapterError> {
    let registry = read_registry(bus, deadline)?;
    let apps = infos(bus, &registry, |_| true, deadline)?;
    Ok(AppInventory {
        apps,
        skipped: registry.skipped,
    })
}

/// The error for a name that no registry application has but a running
/// process does.
pub(crate) fn missing_connection(query: &str, pid: u32, comm: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::AppNotFound,
        format!("'{query}' is running (pid {pid}) but is not on the accessibility bus"),
    )
    .with_suggestion(MISSING_CONNECTION)
    .with_details(json!({
        "kind": "no_accessibility_connection",
        "pid": pid,
        "process": comm,
    }))
}

/// Registry matches, or the named diagnostic when only a process matches, or
/// the ordinary empty answer when nothing does.
pub(crate) fn scoped_outcome(
    matched: Vec<AppInfo>,
    query: &str,
    running: impl FnOnce(&str) -> Option<(u32, String)>,
) -> Result<Vec<AppInfo>, AdapterError> {
    if !matched.is_empty() {
        return Ok(matched);
    }
    match running(query) {
        Some((pid, comm)) => Err(missing_connection(query, pid, &comm)),
        None => Ok(matched),
    }
}

pub(crate) fn scoped(
    bus: &SharedBus,
    name: &str,
    bundle_id: Option<&str>,
    deadline: Deadline,
) -> Result<Vec<AppInfo>, AdapterError> {
    if bundle_id.is_some() {
        return Ok(Vec::new());
    }
    let registry = read_registry(bus, deadline)?;
    let matched = infos(
        bus,
        &registry,
        |app| name_matches(name, &app.name, &app.facts),
        deadline,
    )?;
    scoped_outcome(matched, name, find_running)
}

#[cfg(test)]
#[path = "app_ops_tests.rs"]
mod tests;
