use std::cmp::Reverse;

use crate::system::registry::listed_window::ListedWindow;
use crate::system::x11::map::{ClientMatch, match_client};
use crate::system::x11::{X11Client, X11State};

/// How far above the bottom of the stacking order the window's one client
/// sits, or `None` when the window maps to no single client in the stack.
fn height_in(window: &ListedWindow, clients: &[X11Client], stack: &[u32]) -> Option<usize> {
    let ClientMatch::One(id) = match_client(&window.frame, clients) else {
        return None;
    };
    stack.iter().rposition(|entry| *entry == id)
}

/// The windows from front to back: the focused window first, then the others
/// from the top of the stacking order to the bottom. A window with no single
/// client in the stack follows those that have one, in the order it was
/// listed; without a stacking order the listing order follows the focused
/// window.
pub(crate) fn front_to_back(
    windows: Vec<ListedWindow>,
    state: &X11State,
    stack: Option<&[u32]>,
) -> Vec<ListedWindow> {
    let clients: &[X11Client] = match state {
        X11State::Ready { clients, .. } => clients,
        _ => &[],
    };
    let mut ranked: Vec<(ListedWindow, Option<usize>)> = windows
        .into_iter()
        .map(|window| {
            let height = stack.and_then(|stack| height_in(&window, clients, stack));
            (window, height)
        })
        .collect();
    ranked.sort_by_key(|(window, height)| {
        (
            !window.info.state.is_focused,
            height.is_none(),
            Reverse(height.unwrap_or_default()),
        )
    });
    ranked.into_iter().map(|(window, _)| window).collect()
}

#[cfg(test)]
#[path = "window_order_tests.rs"]
mod tests;
