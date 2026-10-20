use agent_desktop_core::{AdapterError, Deadline, WindowFilter, WindowInfo};

use super::SharedBus;
use super::focus::decide_focus;
use super::listed_window::ListedWindow;
use super::proc_scan::find_running;
use super::proc_stat;
use super::registry::{Registry, read_registry};
use super::registry_app::RegistryApp;
use super::registry_frames::read_frames;
use super::stopped_windows::of_stopped_apps;
use super::unresponsive::{require_known_target, scoped_windows_of};
use super::window_filter::is_helper_app;
use super::window_ids::WindowIds;
use super::window_listing::collect;
use super::window_position::{mark_minimized, mark_other_desktop, place_windows, trim_shadows};
use super::x11::desktops;
use super::x11::map::{ClientMatch, match_client};
use super::x11::{self, DisplayReader, FrameFacts, X11Evidence, X11Source, X11State};

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
        .filter(|(_, app)| {
            is_listable(app) && select(app) && !proc_stat::is_stopped(app.root.pid())
        })
        .collect();
    let roots: Vec<_> = chosen.iter().map(|(_, app)| app.root.clone()).collect();
    let frames = read_frames(bus, &roots, deadline)?;
    Ok(collect(&chosen, &frames))
}

/// Keeps the id of each listed window against its X11 client, for the day its
/// process is stopped and can no longer be asked.
fn remember_ids(listed: &[ListedWindow], state: &X11State, ids: Option<&WindowIds>) {
    let (Some(ids), X11State::Ready { clients, .. }) = (ids, state) else {
        return;
    };
    for window in listed {
        let (ClientMatch::One(client), Some(instance)) = (
            match_client(&window.frame, clients),
            window.info.process_instance.as_deref(),
        ) else {
            continue;
        };
        ids.remember(client, instance, &window.info.id);
    }
}

/// The window the sources agree is focused. A window counts when its frame
/// carries `ACTIVE`; so does a modal or dialog frame its toolkit never
/// reports active when the display server names its client the active
/// window and input focus lies in the client or one of its descendants.
pub(super) fn focused_index(
    listed: &[ListedWindow],
    state: &X11State,
    input_focus_in: impl Fn(u32) -> bool,
) -> Option<usize> {
    let mut candidates: Vec<usize> = listed
        .iter()
        .enumerate()
        .filter(|(_, window)| window.active)
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() && !listed.iter().any(|window| window.modal || window.dialog) {
        return None;
    }
    let frames: Vec<FrameFacts> = listed.iter().map(|window| window.frame.clone()).collect();
    let evidence = x11::evidence(state, &frames);
    if let X11Evidence::Frame(named) = evidence
        && (listed[named].modal || listed[named].dialog)
        && !candidates.contains(&named)
        && let X11State::Ready {
            active: Some(client),
            ..
        } = state
        && input_focus_in(*client)
    {
        candidates.push(named);
    }
    decide_focus(&candidates, evidence)
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
    let mut listed = if let Some(query) = filter.app.as_deref() {
        require_known_target(&registry, select, query, find_running)?;
        scoped_windows_of(bus, &registry, select, deadline)?
    } else {
        windows_of(bus, &registry, select, deadline)?
    };
    let state = pending.finish(deadline).unwrap_or(X11State::Unavailable);
    if filter.app.is_none() {
        let ids = WindowIds::in_runtime_dir();
        remember_ids(&listed, &state, ids.as_ref());
        listed.extend(of_stopped_apps(
            &registry,
            select,
            proc_stat::is_stopped,
            &state,
            |client, instance| ids.as_ref()?.recall(client, instance),
        ));
    }
    place_windows(&mut listed, &state);
    trim_shadows(&mut listed, &state);
    mark_minimized(&mut listed, &state);
    mark_other_desktop(
        &mut listed,
        &state,
        desktops.finish(deadline).flatten().as_ref(),
    );
    let focused = focused_index(&listed, &state, |client| {
        super::focus_world::focus_send::input_focus_in(client, deadline)
    });
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

#[cfg(test)]
#[path = "window_ops_tests.rs"]
mod tests;
