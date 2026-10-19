use agent_desktop_core::Rect;

use super::listed_window::ListedWindow;
use super::x11::map::{ClientMatch, match_client};
use super::x11::{X11Client, X11State};
use crate::tree::Extents;

fn client_origin(window: &ListedWindow, clients: &[X11Client]) -> Option<(i32, i32)> {
    let ClientMatch::One(id) = match_client(&window.frame, clients) else {
        return None;
    };
    let bounds = clients.iter().find(|client| client.id == id)?.bounds?;
    Some((bounds.x, bounds.y))
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
            .zip(client_origin(window, clients))
            .map(|(size, origin)| at(origin, size));
    }
}

#[cfg(test)]
#[path = "window_position_tests.rs"]
mod tests;
