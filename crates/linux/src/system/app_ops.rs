use std::collections::HashSet;

use agent_desktop_core::{
    AdapterError, AppInfo, AppInventory, AppPresentation, Deadline, DeliverySemantics, ErrorCode,
};
use serde_json::json;

use super::SharedBus;
use super::proc_scan::find_running;
use super::proc_stat;
use super::registry::{Registry, read_registry};
use super::registry_app::RegistryApp;
use super::unresponsive::{settle_matches, stopped_target};
use super::window_ops::windows_of;

const MISSING_CONNECTION: &str = "The process is running but is not on the accessibility bus. Sandboxed apps (snap, Flatpak) cannot reach it unless the bus is shared. A Chromium browser (Chrome, Chromium) needs --force-renderer-accessibility on its command line: ACCESSIBILITY_ENABLED=1 alone puts it on the bus with no windows. An Electron app needs no accessibility flag, but needs --no-sandbox where its chrome-sandbox is not setuid. A Qt app needs QT_ACCESSIBILITY=1. The registry was read once, so an app launched a moment ago may need a retry.";

pub(crate) fn info(app: &RegistryApp, foreground: bool) -> AppInfo {
    AppInfo {
        name: app.name.clone(),
        pid: app.root.pid().into(),
        bundle_id: super::app_name::desktop_id(&app.facts),
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
    Ok(one_entry_per_process(
        registry
            .apps
            .iter()
            .enumerate()
            .filter(|(_, app)| select(app))
            .map(|(index, app)| info(app, windowed.contains(&index)))
            .collect(),
    ))
}

/// Collapses the entries one process has on the registry into one. A Qt
/// application opens two accessibility connections, only one of which owns
/// the windows, and listing both would show a single process twice and make
/// its own name ambiguous. The entry that presents windows decides the
/// presentation.
pub(crate) fn one_entry_per_process(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    let mut merged: Vec<AppInfo> = Vec::with_capacity(apps.len());
    for app in apps {
        match merged
            .iter_mut()
            .find(|seen| seen.pid == app.pid && seen.process_instance == app.process_instance)
        {
            Some(seen) => {
                if app.presentation == Some(AppPresentation::Foreground) {
                    seen.presentation = app.presentation;
                }
            }
            None => merged.push(app),
        }
    }
    merged
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
    if super::shell::is_shell_process(pid) {
        return super::shell::not_this_sessions_shell();
    }
    AdapterError::new(
        ErrorCode::AppNotFound,
        format!("'{query}' is running (pid {pid}) but is not on the accessibility bus"),
    )
    .with_suggestion(MISSING_CONNECTION)
    .with_disposition(DeliverySemantics::not_delivered())
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
    let registry = read_registry(bus, deadline)?;
    let select = |app: &RegistryApp| match bundle_id {
        Some(id) => super::app_name::desktop_id(&app.facts)
            .is_some_and(|desktop_id| desktop_id.eq_ignore_ascii_case(id)),
        None => app.matches(name),
    };
    let matched = infos(bus, &registry, select, deadline)?;
    let matched = settle_matches(matched, || {
        stopped_target(&registry, select, proc_stat::is_stopped)
    })?;
    scoped_outcome(matched, bundle_id.unwrap_or(name), find_running)
}

#[cfg(test)]
#[path = "app_ops_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "app_ops_live_tests.rs"]
mod live_tests;
