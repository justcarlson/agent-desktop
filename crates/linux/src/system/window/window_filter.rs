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

fn same_geometry(
    frame: &crate::system::x11::FrameFacts,
    client: &crate::system::x11::X11Client,
) -> bool {
    let (Some(actual), Some(native)) = (frame.extents, client.bounds) else {
        return false;
    };
    let edges = client.extents.unwrap_or(crate::system::x11::FrameExtents {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    });
    let Ok(left) = i32::try_from(edges.left) else {
        return false;
    };
    let Ok(right) = i32::try_from(edges.right) else {
        return false;
    };
    let Ok(top) = i32::try_from(edges.top) else {
        return false;
    };
    let Ok(bottom) = i32::try_from(edges.bottom) else {
        return false;
    };
    let outer = crate::tree::Extents {
        x: native.x.saturating_sub(left),
        y: native.y.saturating_sub(top),
        width: native.width.saturating_add(left).saturating_add(right),
        height: native.height.saturating_add(top).saturating_add(bottom),
    };
    [native, outer].iter().any(|bounds| {
        let (l, t, r, b) = client.shadow;
        let content = crate::tree::Extents {
            x: bounds.x.saturating_add(l),
            y: bounds.y.saturating_add(t),
            width: bounds.width.saturating_sub(l).saturating_sub(r),
            height: bounds.height.saturating_sub(t).saturating_sub(b),
        };
        [*bounds, content].iter().any(|expected| {
            actual.width > 0
                && actual.height > 0
                && actual.width == expected.width
                && actual.height == expected.height
                && (!frame.global_position || (actual.x == expected.x && actual.y == expected.y))
        })
    })
}

fn unproven(pid: u32) -> agent_desktop_core::AdapterError {
    agent_desktop_core::AdapterError::new(
        agent_desktop_core::ErrorCode::ActionNotSupported,
        format!(
            "The native window type of accessibility frame owned by pid {pid} could not be proven"
        ),
    )
    .with_disposition(agent_desktop_core::DeliverySemantics::not_delivered())
}

/// Classify a complete, uniform group without choosing a native window identity.
fn native_group(
    frame: &crate::system::x11::FrameFacts,
    frames: &[crate::system::x11::FrameFacts],
    clients: &[crate::system::x11::X11Client],
    user_ids: &std::collections::HashSet<u32>,
    helper_ids: &std::collections::HashSet<u32>,
    complete_process: bool,
) -> Result<bool, agent_desktop_core::AdapterError> {
    if !complete_process {
        return Err(unproven(frame.pid));
    }
    let owned: std::collections::HashSet<_> = clients
        .iter()
        .filter(|client| client.pid == Some(frame.pid))
        .map(|client| client.id)
        .collect();
    if owned.is_empty()
        || owned.len() != frames.iter().filter(|other| other.pid == frame.pid).count()
    {
        return Err(unproven(frame.pid));
    }
    if owned.is_subset(user_ids) {
        return Ok(true);
    }
    if owned.is_subset(helper_ids) {
        return Ok(false);
    }
    let matching = |frame: &crate::system::x11::FrameFacts| -> std::collections::HashSet<u32> {
        clients
            .iter()
            .filter(|client| client.pid == Some(frame.pid) && same_geometry(frame, client))
            .map(|client| client.id)
            .collect()
    };
    let ids = matching(frame);
    let user = !ids.is_empty() && ids.is_subset(user_ids);
    if !user && (ids.is_empty() || !ids.is_subset(helper_ids)) {
        return Err(unproven(frame.pid));
    }
    let mut claims = 0;
    for other in frames.iter().filter(|other| other.pid == frame.pid) {
        let other_ids = matching(other);
        if !ids.is_disjoint(&other_ids) {
            if ids != other_ids {
                return Err(unproven(frame.pid));
            }
            claims += 1;
        }
    }
    (claims == ids.len())
        .then_some(user)
        .ok_or_else(|| unproven(frame.pid))
}

pub(in crate::system) fn without_helpers(
    windows: Vec<listed_window::ListedWindow>,
    deadline: agent_desktop_core::Deadline,
) -> Result<Vec<listed_window::ListedWindow>, agent_desktop_core::AdapterError> {
    use crate::system::display::display_server::{DisplayServer, session_server};
    use crate::system::x11::{
        DisplayReader, X11Source, X11State, control, window_kind::WindowKind,
    };
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
        return Ok(windows
            .into_iter()
            .filter(|window| !helpers.contains(&window.frame.pid))
            .collect());
    }
    let pending = crate::system::x11::start(|| DisplayReader.read());
    let Some(X11State::Ready { clients, .. }) = pending.finish(deadline) else {
        return Err(unproven(windows[0].frame.pid));
    };
    let pids: std::collections::HashSet<_> =
        windows.iter().map(|window| window.frame.pid).collect();
    let mut user_ids = std::collections::HashSet::new();
    let mut helper_ids = std::collections::HashSet::new();
    let mut absent = std::collections::HashSet::new();
    for pid in pids {
        if !clients.iter().any(|client| client.pid == Some(pid))
            && crate::system::x11::client_population::complete(pid, Vec::new(), deadline)
        {
            absent.insert(pid);
            continue;
        }
        let Some(native) = control::toplevels_of(pid, deadline).ready() else {
            return Err(unproven(pid));
        };
        for native in native {
            match native.kind {
                WindowKind::Normal | WindowKind::Dialog | WindowKind::Utility => {
                    user_ids.insert(native.id);
                }
                WindowKind::Menu | WindowKind::Dock | WindowKind::Desktop => {
                    helper_ids.insert(native.id);
                }
                WindowKind::Other => {}
            }
        }
    }
    let mut listed = Vec::new();
    let frames: Vec<_> = windows
        .iter()
        .filter(|window| !window.unshown)
        .map(|window| window.frame.clone())
        .collect();
    let mut complete = std::collections::HashSet::new();
    for frame in &frames {
        let owned: Vec<_> = clients
            .iter()
            .filter(|client| client.pid == Some(frame.pid))
            .map(|client| client.id)
            .collect();
        if !complete.contains(&frame.pid)
            && !owned.is_empty()
            && crate::system::x11::client_population::complete(frame.pid, owned, deadline)
        {
            complete.insert(frame.pid);
        }
    }
    for window in &windows {
        if window.unshown {
            listed.push(window.clone());
            continue;
        }
        if absent.contains(&window.frame.pid) {
            continue;
        }
        match native_group(
            &window.frame,
            &frames,
            &clients,
            &user_ids,
            &helper_ids,
            complete.contains(&window.frame.pid),
        ) {
            Ok(true) => listed.push(window.clone()),
            Ok(false) => {}
            Err(error) => {
                if crate::system::process::protected::is_display_owner(window.frame.pid, deadline)?
                    || control::decoration_matches(
                        window.frame.pid,
                        frames
                            .iter()
                            .filter(|other| other.pid == window.frame.pid)
                            .count(),
                        deadline,
                    )
                {
                    continue;
                }
                return Err(error);
            }
        }
    }
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
