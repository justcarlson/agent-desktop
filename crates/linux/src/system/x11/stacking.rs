use std::time::Duration;

use agent_desktop_core::Deadline;

use crate::system::display_server::DisplayServer;

use super::map::{ClientMatch, match_client};
use super::stacking_read::StackingRead;
use super::stacking_source::StackingSource;
use super::window_x11_state::WindowX11State;
use super::{FrameFacts, X11State, start};

/// How long one stacking read may take on the real display. A read is a
/// handful of pipelined round trips, 2 to 4 ms in `L27-3`.
pub(super) const READ_BUDGET: Duration = Duration::from_millis(1_500);

/// Runs a display-server read on its own thread and gives up with `fallback`
/// when `deadline` passes first: a round trip to a server that has stopped
/// answering cannot be interrupted, only abandoned.
pub(super) fn bounded<T: Send + 'static>(
    deadline: Deadline,
    fallback: T,
    read: impl FnOnce() -> T + Send + 'static,
) -> T {
    start(read).finish(deadline).unwrap_or(fallback)
}

/// Chooses where a stacking read goes from what the shared detector says
/// about the session. Wayland and a missing display answer without a
/// connection being opened: a Wayland desktop's windows are not X11 evidence
/// even when Xwayland is up.
pub(super) fn route(
    server: DisplayServer,
    read: impl FnOnce(String) -> StackingRead,
) -> StackingRead {
    match server {
        DisplayServer::X11(display) => read(display),
        DisplayServer::Wayland => StackingRead::NotX11,
        DisplayServer::Unavailable(..) => StackingRead::Unavailable,
    }
}

/// The X11 client of a listed window and its state, or `None` when the
/// session is not X11, no manager answers, or the window maps to no single
/// client.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the live placement read is its caller")
)]
pub(crate) fn frame_window_state(
    source: &dyn StackingSource,
    deadline: Deadline,
    frame: &FrameFacts,
) -> Option<WindowX11State> {
    let X11State::Ready { clients, .. } = source.clients() else {
        return None;
    };
    let ClientMatch::One(client_id) = match_client(frame, &clients) else {
        return None;
    };
    if deadline.is_expired() {
        return None;
    }
    let StackingRead::Ready(snapshot) = source.read(client_id) else {
        return None;
    };
    let window = snapshot
        .stack
        .iter()
        .find(|window| window.id == client_id)?;
    Some(WindowX11State {
        client_id,
        hidden: window.flags.hidden,
        client_origin: (window.client.x, window.client.y),
    })
}

#[cfg(test)]
#[path = "stacking_tests.rs"]
mod tests;
