use agent_desktop_core::{AdapterError, Deadline, WindowFilter, WindowInfo};

use super::SharedBus;
use super::listed_window::ListedWindow;
use super::proc_scan::find_running;
use super::proc_stat;
use super::registry::{Registry, read_registry, read_registry_for};
use super::registry_app::RegistryApp;
use super::signal_inventory::{Pass, read_selected, run_pass_with};
use super::unresponsive::{require_known_target, settle_scoped, stopped_target};
use super::window_filter::is_helper_app;
use super::window_position::place_windows;
use super::x11::desktops::{self, Desktops};
use super::x11::{self, DisplayReader, X11Source, X11State};

pub(super) fn is_listable(app: &RegistryApp) -> bool {
    !is_helper_app(
        app.registry_name.as_deref().unwrap_or(&app.name),
        app.facts.exe_basename.as_deref(),
    )
}

/// The windows of the applications `select` accepts, in registry order.
pub(crate) fn windows_of(
    bus: &SharedBus,
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    deadline: Deadline,
) -> Result<Vec<ListedWindow>, AdapterError> {
    read_selected(bus, registry, select, deadline).map(|(windows, _)| windows)
}

/// Places the windows of a pass, trims the client-side shadows the toolkit
/// drew, marks the minimized ones, those on another desktop and the focused
/// one, and returns the index of the focused window.
fn finish_listing(pass: &mut Pass, state: &X11State, desktops: Option<&Desktops>) -> Option<usize> {
    place_windows(&mut pass.windows, state, desktops);
    if let Some(index) = pass.focused {
        pass.windows[index].info.state.is_focused = true;
    }
    pass.focused
}

/// Lists top-level windows, marking the one the sources agree is focused.
pub(crate) fn list_windows(
    bus: &SharedBus,
    filter: &WindowFilter,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let pending = x11::start(|| DisplayReader.read());
    let desktops = x11::start(desktops::read);
    let registry = read_registry(bus, deadline)?;
    let select = |app: &RegistryApp| filter.app.as_deref().is_none_or(|query| app.matches(query));
    let stopped = match filter.app.as_deref() {
        Some(query) => {
            require_known_target(&registry, select, query, find_running)?;
            stopped_target(&registry, select, proc_stat::is_stopped)
        }
        None => None,
    };
    let finish = |deadline| pending.finish(deadline).unwrap_or(X11State::Unavailable);
    let (mut pass, state) = run_pass_with(bus, &registry, select, finish, deadline)?;
    pass.windows = settle_scoped(pass.windows, stopped)?;
    let focused = finish_listing(
        &mut pass,
        &state,
        desktops.finish(deadline).flatten().as_ref(),
    );
    Ok(pass
        .windows
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !filter.focused_only || Some(*index) == focused)
        .map(|(_, window)| window.info)
        .collect())
}

/// The windows of one process, placed as `list_windows` places them. Only
/// that process is read, so an unrelated stopped application costs nothing.
pub(crate) fn windows_of_process(
    bus: &SharedBus,
    pid: u32,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let pending = x11::start(|| DisplayReader.read());
    let desktops = x11::start(desktops::read);
    let registry = read_registry_for(bus, Some(pid), deadline)?;
    let select = |app: &RegistryApp| app.root.pid() == pid;
    let finish = |deadline| pending.finish(deadline).unwrap_or(X11State::Unavailable);
    let (mut pass, state) = run_pass_with(bus, &registry, select, finish, deadline)?;
    finish_listing(
        &mut pass,
        &state,
        desktops.finish(deadline).flatten().as_ref(),
    );
    Ok(pass.windows.into_iter().map(|window| window.info).collect())
}

/// The focused window, or none when the available sources do not agree.
pub(crate) fn focused_window(
    bus: &SharedBus,
    deadline: Deadline,
) -> Result<Option<WindowInfo>, AdapterError> {
    let filter = WindowFilter {
        focused_only: true,
        app: None,
    };
    Ok(list_windows(bus, &filter, deadline)?.into_iter().next())
}

#[cfg(test)]
#[path = "window_ops_tests.rs"]
mod tests;
