//! The compositor-only windows of a Hyprland session, merged into
//! `list_windows` beside the accessibility frames.

use std::collections::HashSet;

use agent_desktop_core::{AdapterError, Deadline, WindowInfo};

use crate::system::app::app_name::name_matches;
use crate::system::hyprland::bound_request::bound_request;
use crate::system::hyprland::clients::client_parse::parse_active;
use crate::system::hyprland::clients::client_read::read_clients;
use crate::system::hyprland::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::identity::compositor_windows::compositor_windows;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::routed_binding::routed_binding;
use crate::system::process::process_facts::{self, ProcessFacts};
use crate::system::process::process_lookup::Lookup;
use crate::system::wayland::toplevel::toplevel_session::ToplevelSession;

/// The toplevels no accessibility frame represents, when the session is
/// Hyprland; empty on any other route. `represented` is built only on that
/// route and judges a toplevel by its owner's pid and title. `query` keeps
/// those whose class, command name or executable matches.
pub(crate) fn hyprland_windows<R: Fn(u32, &str) -> bool>(
    represented: impl FnOnce() -> Result<R, AdapterError>,
    query: Option<&str>,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    match routed_binding()? {
        Some(binding) => read_windows(&binding, represented()?, query, deadline),
        None => Ok(Vec::new()),
    }
}

/// Whether an accessibility frame already lists a toplevel. A process the
/// listing read is judged by its frames' titles, so a toplevel it exposes
/// no frame for, such as every window of an application whose tree is off,
/// stays listed. A process the listing's filter skipped is judged by
/// registration alone.
pub(crate) fn represented_by(
    frames: HashSet<(u32, String)>,
    read: HashSet<u32>,
    registered: HashSet<u32>,
) -> impl Fn(u32, &str) -> bool {
    move |pid, title| match read.contains(&pid) {
        true => frames.contains(&(pid, title.to_owned())),
        false => registered.contains(&pid),
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
    represented: impl Fn(u32, &str) -> bool,
    query: Option<&str>,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let clients = read_clients(binding, deadline)?;
    let toplevels = ToplevelSession::open(binding, deadline)?.views()?;
    let active = bound_request(binding, "j/activewindow", deadline)?;
    let active = parse_active(&active).ok_or_else(|| {
        AdapterError::internal("The Hyprland active window could not be read")
            .with_suggestion("Retry once the compositor answers")
    })?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let windows = compositor_windows(
        inventory,
        active,
        (binding.pid, binding.start_time),
        represented,
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
