use agent_desktop_core::{AdapterError, Deadline, WindowFilter, WindowInfo};

use super::SharedBus;
use super::focus::decide_focus;
use super::listed_window::ListedWindow;
use super::proc_scan::find_running;
use super::registry::{Registry, read_registry};
use super::registry_app::RegistryApp;
use super::registry_frames::read_frames;
use super::unresponsive::{require_known_target, scoped_windows_of};
use super::window_filter::is_helper_app;
use super::window_listing::collect;
use super::window_position::place_windows;
use super::x11::{self, DisplayReader, FrameFacts, X11Source, X11State};

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
    let chosen: Vec<(usize, &RegistryApp)> = registry
        .apps
        .iter()
        .enumerate()
        .filter(|(_, app)| is_listable(app) && select(app))
        .collect();
    let roots: Vec<_> = chosen.iter().map(|(_, app)| app.root.clone()).collect();
    let frames = read_frames(bus, &roots, deadline)?;
    Ok(collect(&chosen, &frames))
}

fn focused_index(listed: &[ListedWindow], state: &X11State) -> Option<usize> {
    let candidates: Vec<usize> = listed
        .iter()
        .enumerate()
        .filter(|(_, window)| window.active)
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let frames: Vec<FrameFacts> = listed.iter().map(|window| window.frame.clone()).collect();
    decide_focus(&candidates, x11::evidence(state, &frames))
}

/// Lists top-level windows, marking the one the sources agree is focused.
pub(crate) fn list_windows(
    bus: &SharedBus,
    filter: &WindowFilter,
    deadline: Deadline,
) -> Result<Vec<WindowInfo>, AdapterError> {
    let pending = x11::start(|| DisplayReader.read());
    let registry = read_registry(bus, deadline)?;
    let select = |app: &RegistryApp| filter.app.as_deref().is_none_or(|query| app.matches(query));
    let mut listed = if let Some(query) = filter.app.as_deref() {
        require_known_target(&registry, select, query, find_running)?;
        scoped_windows_of(bus, &registry, select, deadline)?
    } else {
        windows_of(bus, &registry, select, deadline)?
    };
    let needs_position = listed.iter().any(|window| !window.frame.global_position);
    let state = if needs_position || listed.iter().any(|window| window.active) {
        pending.finish(deadline).unwrap_or(X11State::Unavailable)
    } else {
        X11State::Unavailable
    };
    place_windows(&mut listed, &state);
    let focused = focused_index(&listed, &state);
    if let Some(index) = focused {
        listed[index].info.state.is_focused = true;
    }
    Ok(listed
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !filter.focused_only || Some(*index) == focused)
        .map(|(_, window)| window.info)
        .collect())
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
