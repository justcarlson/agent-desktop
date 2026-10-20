use agent_desktop_core::Rect;

use super::listed_window::ListedWindow;
use super::x11::desktops::Desktops;
use super::x11::map::{ClientMatch, match_client};
use super::x11::{FrameFacts, X11Client, X11State};
use crate::tree::Extents;

/// Where the single X11 client of a window sits, or `None` when the window
/// maps to no client or to several.
pub(crate) fn frame_origin(frame: &FrameFacts, clients: &[X11Client]) -> Option<(i32, i32)> {
    let ClientMatch::One(id) = match_client(frame, clients) else {
        return None;
    };
    let client = clients.iter().find(|client| client.id == id)?;
    let bounds = client.bounds?;
    Some((
        bounds.x.checked_add(client.shadow.0)?,
        bounds.y.checked_add(client.shadow.1)?,
    ))
}

fn at(origin: (i32, i32), size: Extents) -> Rect {
    Rect::from(Extents {
        x: origin.0,
        y: origin.1,
        width: size.width,
        height: size.height,
    })
}

/// Gives a window of a toolkit that reports no global position the place its
/// X11 client occupies, keeping the size the toolkit reported. The window
/// stays without bounds when the display server did not answer or the window
/// maps to no single client: the toolkit's own position is `[0, 0]` there and
/// is never a position.
pub(crate) fn place_windows(listed: &mut [ListedWindow], state: &X11State) {
    let clients: &[X11Client] = match state {
        X11State::Ready { clients, .. } => clients,
        _ => &[],
    };
    for window in listed
        .iter_mut()
        .filter(|window| !window.frame.global_position)
    {
        window.info.bounds = window
            .frame
            .extents
            .zip(frame_origin(&window.frame, clients))
            .map(|(size, origin)| at(origin, size));
    }
}

/// Marks the windows whose single X11 client the window manager keeps on a
/// desktop other than the one shown as not visible: the display server has
/// them unmapped, whatever the toolkit says.
pub(crate) fn mark_other_desktop(
    listed: &mut [ListedWindow],
    state: &X11State,
    desktops: Option<&Desktops>,
) {
    let (X11State::Ready { clients, .. }, Some(desktops)) = (state, desktops) else {
        return;
    };
    for window in listed {
        let ClientMatch::One(id) = match_client(&window.frame, clients) else {
            continue;
        };
        if desktops.hides(id) {
            window.info.state.visible = Some(false);
        }
    }
}

/// Marks the windows whose single X11 client the window manager reports hidden
/// as minimized and not visible, as a toolkit that reports no iconified state
/// leaves them. A window the display server cannot place keeps what the
/// toolkit said.
pub(crate) fn mark_minimized(listed: &mut [ListedWindow], state: &X11State) {
    let X11State::Ready { clients, .. } = state else {
        return;
    };
    for window in listed {
        let ClientMatch::One(id) = match_client(&window.frame, clients) else {
            continue;
        };
        if clients
            .iter()
            .any(|client| client.id == id && client.hidden)
        {
            window.info.state.minimized = Some(true);
            window.info.state.visible = Some(false);
        }
    }
}

#[cfg(test)]
#[path = "window_position_tests.rs"]
mod tests;
