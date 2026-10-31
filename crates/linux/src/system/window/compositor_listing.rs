//! The compositor-only windows of a Hyprland session, merged into
//! `list_windows` beside the accessibility frames.

use agent_desktop_core::{AdapterError, Deadline, WindowInfo};

use super::accessible_frames::AccessibleFrames;
use crate::system::app::app_name::name_matches;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::bind::routed_binding::routed_binding;
use crate::system::hyprland::clients::client_read::{read_clients, read_desktop};
use crate::system::hyprland::identity::compositor_windows::compositor_windows;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::toplevel::toplevel_read::read_toplevels;
use crate::system::process::process_facts::{self, ProcessFacts};
use crate::system::process::process_lookup::Lookup;

/// The toplevels no accessibility frame represents, when the session is
/// Hyprland; empty on any other route. `frames` is read only on that route.
/// `query` keeps those whose class, command name or executable matches.
pub(crate) fn hyprland_windows(
    frames: impl FnOnce() -> Result<AccessibleFrames, AdapterError>,
    query: Option<&str>,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    match routed_binding()? {
        Some(binding) => read_windows(&binding, &frames()?, query, deadline),
        None => Ok(Vec::new()),
    }
}

/// A name that no accessibility application or process answers to still
/// names the compositor-only toplevels whose class matches it. With none,
/// the name stays `missing`.
pub(crate) fn unregistered_target(
    missing: AdapterError,
    compositor: impl FnOnce() -> Result<Vec<WindowInfo>, AdapterError>,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let windows = compositor()?;
    match windows.is_empty() {
        true => Err(missing),
        false => Ok(windows),
    }
}

fn read_windows(
    binding: &HyprlandBinding,
    frames: &AccessibleFrames,
    query: Option<&str>,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let clients = read_clients(binding, deadline)?;
    let toplevels = read_toplevels(binding, deadline)?;
    let desktop = read_desktop(binding, deadline)?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let represented = frames.represented(inventory);
    let windows = compositor_windows(
        inventory,
        &desktop,
        (binding.pid, binding.start_time),
        |client| represented.contains(&client.address),
        |pid| facts_of(pid).map(|facts| facts.instance),
    );
    Ok(match query {
        Some(query) => windows
            .into_iter()
            .filter(|window| matches(query, window))
            .collect(),
        None => windows,
    })
}

fn facts_of(pid: u32) -> Option<ProcessFacts> {
    match process_facts::lookup(pid) {
        Lookup::Found(facts) => Some(facts),
        Lookup::Gone | Lookup::Unreadable => None,
    }
}

fn matches(query: &str, window: &WindowInfo) -> bool {
    let aliases: Vec<String> = facts_of(window.pid.get())
        .map(|facts| {
            std::iter::once(facts.comm)
                .chain(facts.exe_basename)
                .collect()
        })
        .unwrap_or_default();
    name_matches(query, &window.app, &aliases)
}

#[cfg(test)]
#[path = "compositor_listing_tests.rs"]
mod tests;
