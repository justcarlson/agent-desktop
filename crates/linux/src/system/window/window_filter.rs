use super::native_group::{self, unproven};
use crate::system::bus::ensure_budget;
use crate::system::registry::listed_window;
use crate::tree::role_numbers::{DIALOG, FILLER, FRAME, PANEL, WINDOW};
use crate::tree::state_bits::{ACTIVE, ICONIFIED, MODAL, SHOWING, StateBits, VISIBLE};
use crate::tree::{COMPONENT_INTERFACE, Field};

/// Whether a child of an application root is a top-level window: a frame,
/// dialog or window role that implements the component interface, or a panel
/// that carries `MODAL` (GTK 4 presents its shortcuts window that way, and an
/// application root has no panel child otherwise), or a filler that is
/// `SHOWING` and `VISIBLE` (Qt presents a top-level `QWidget` that is not a
/// `QMainWindow` that way, L30-28). GTK 3 names the frame `frame` and
/// GTK 4 names the same numeric role `application`, so only the number is
/// read.
pub(crate) fn is_window_shape(
    role: &Field<u32>,
    interfaces: &Field<Vec<String>>,
    states: &Field<Vec<u32>>,
) -> bool {
    let shaped = match role.present() {
        Some(&(FRAME | DIALOG | WINDOW)) => true,
        Some(&PANEL) => states
            .present()
            .is_some_and(|words| StateBits::new(words).has(MODAL)),
        Some(&FILLER) => states.present().is_some_and(|words| {
            let bits = StateBits::new(words);
            bits.has(SHOWING) && bits.has(VISIBLE)
        }),
        _ => false,
    };
    shaped
        && interfaces
            .present()
            .is_some_and(|list| list.iter().any(|name| name == COMPONENT_INTERFACE))
}

pub(in crate::system) fn without_helpers(
    windows: Vec<listed_window::ListedWindow>,
    deadline: agent_desktop_core::Deadline,
) -> Result<Vec<listed_window::ListedWindow>, agent_desktop_core::AdapterError> {
    use crate::system::display::display_server::{DisplayServer, session_server};
    use crate::system::x11::{
        DisplayReader, X11Source, X11State, control, window_kind::WindowKind,
    };
    ensure_budget(deadline)?;
    if windows.iter().all(|window| window.unshown) {
        return Ok(windows);
    }
    if matches!(session_server(), DisplayServer::Wayland) {
        let pids: std::collections::HashSet<_> =
            windows.iter().map(|window| window.frame.pid).collect();
        let mut helpers = std::collections::HashSet::new();
        for pid in pids {
            if crate::system::process::protected::is_display_owner(pid, deadline)? {
                helpers.insert(pid);
            }
        }
        ensure_budget(deadline)?;
        return Ok(windows
            .into_iter()
            .filter(|window| !helpers.contains(&window.frame.pid))
            .collect());
    }
    let pending = crate::system::x11::start(|| DisplayReader.read());
    let read = pending.finish(deadline);
    ensure_budget(deadline)?;
    let Some(X11State::Ready { mut clients, .. }) = read else {
        return Err(unproven(windows[0].frame.pid));
    };
    let pids: std::collections::HashSet<_> =
        windows.iter().map(|window| window.frame.pid).collect();
    let mut user_ids = std::collections::HashMap::new();
    let mut helper_ids = std::collections::HashSet::new();
    let mut absent = std::collections::HashSet::new();
    let mut complete = std::collections::HashSet::new();
    for pid in pids {
        ensure_budget(deadline)?;
        let managed: Vec<_> = clients
            .iter()
            .filter(|client| client.pid == Some(pid))
            .map(|client| client.id)
            .collect();
        let population =
            crate::system::x11::native_population::read(pid, managed.clone(), deadline);
        ensure_budget(deadline)?;
        if let crate::system::x11::native_population::NativePopulation::Complete { menus } =
            population
        {
            if managed.is_empty() && menus.is_empty() {
                absent.insert(pid);
                continue;
            }
            complete.insert(pid);
            helper_ids.extend(menus.iter().map(|menu| menu.id));
            clients.extend(menus);
        }
        let native = control::toplevels_of(pid, deadline);
        ensure_budget(deadline)?;
        let Some(native) = native.ready() else {
            return Err(unproven(pid));
        };
        let users = user_ids
            .entry(pid)
            .or_insert_with(std::collections::HashSet::new);
        for native in native {
            match native.kind {
                WindowKind::Normal | WindowKind::Dialog | WindowKind::Utility => {
                    users.insert(native.id);
                }
                WindowKind::Menu | WindowKind::Dock | WindowKind::Desktop => {
                    helper_ids.insert(native.id);
                }
                WindowKind::Other => {}
            }
        }
    }
    let mut listed = Vec::new();
    let no_users = std::collections::HashSet::new();
    for window in &windows {
        ensure_budget(deadline)?;
        if window.unshown {
            listed.push(window.clone());
            continue;
        }
        if absent.contains(&window.frame.pid) {
            continue;
        }
        match native_group::classify(
            window,
            &windows,
            &clients,
            user_ids.get(&window.frame.pid).unwrap_or(&no_users),
            &helper_ids,
            complete.contains(&window.frame.pid),
        ) {
            Ok(true) => listed.push(window.clone()),
            Ok(false) => {}
            Err(error) => {
                ensure_budget(deadline)?;
                let helper = crate::system::process::protected::is_display_owner(
                    window.frame.pid,
                    deadline,
                )? || control::decoration_matches(
                    window.frame.pid,
                    windows
                        .iter()
                        .filter(|other| !other.unshown && other.frame.pid == window.frame.pid)
                        .count(),
                    deadline,
                );
                ensure_budget(deadline)?;
                if helper {
                    continue;
                }
                return Err(error);
            }
        }
    }
    ensure_budget(deadline)?;
    Ok(listed)
}

/// What a frame's state set says. `None` is a state read that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameState {
    pub(crate) visible: Option<bool>,
    pub(crate) minimized: Option<bool>,
    pub(crate) active: Option<bool>,
    pub(crate) modal: Option<bool>,
}

pub(crate) fn frame_state(states: &Field<Vec<u32>>) -> FrameState {
    let bits = states.present().map(|words| StateBits::new(words));
    FrameState {
        visible: bits.map(|bits| bits.has(SHOWING)),
        minimized: bits.map(|bits| bits.has(ICONIFIED)),
        active: bits.map(|bits| bits.has(ACTIVE)),
        modal: bits.map(|bits| bits.has(MODAL)),
    }
}

#[cfg(test)]
#[path = "window_filter_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "window_filter_live_tests.rs"]
mod live_tests;

#[cfg(test)]
#[path = "window_filter_deadline_tests.rs"]
mod deadline_tests;
