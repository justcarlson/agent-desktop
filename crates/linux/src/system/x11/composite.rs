use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;
use x11rb::connection::Connection;
use x11rb::protocol::composite::{ConnectionExt as _, Redirect};
use x11rb::protocol::xproto::{ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

use super::MonitorFault;
use super::drawable::reply_error;
use super::grab::{Raw, read_strips};
use super::grab_error::GrabError;
use super::grab_rect::GrabRect;
use super::map_state::MapState;

const PAINT_WAIT: Duration = Duration::from_millis(1_000);
const PAINT_POLL: Duration = Duration::from_millis(40);

pub(crate) fn is_blank(data: &[u8]) -> bool {
    data.iter().all(|byte| *byte == 0)
}

fn unreadable<E>(_: E) -> GrabError {
    GrabError::Fault(MonitorFault::Unreadable)
}

fn painted(
    connection: &RustConnection,
    pixmap: Window,
    rect: GrabRect,
    visual: u32,
    deadline: Deadline,
) -> Result<Raw, GrabError> {
    let started = Instant::now();
    loop {
        let raw = read_strips(connection, pixmap, rect, deadline, Some(visual))?;
        if !is_blank(&raw.data) {
            return Ok(raw);
        }
        if started.elapsed() >= PAINT_WAIT || deadline.is_expired() {
            return Err(GrabError::Timeout);
        }
        std::thread::sleep(PAINT_POLL.min(deadline.remaining()));
    }
}

/// Reads a mapped window's own pixels through a Composite redirect, so a
/// window another window covers still gives its own paint. The redirect
/// lasts only for this read. A pixmap that is still blank after the wait is
/// refused: it is a window not painted yet, never an image to return.
pub(crate) fn grab_redirected(
    connection: &RustConnection,
    window: Window,
    rect: GrabRect,
    deadline: Deadline,
) -> Result<Raw, GrabError> {
    connection
        .composite_query_version(0, 4)
        .map_err(unreadable)?
        .reply()
        .map_err(unreadable)?;
    let attributes = connection
        .get_window_attributes(window)
        .map_err(unreadable)?
        .reply()
        .map_err(|error| reply_error(connection, window, error))?;
    if MapState::from(attributes.map_state) != MapState::Viewable {
        return Err(GrabError::Unviewable);
    }
    connection
        .composite_redirect_window(window, Redirect::AUTOMATIC)
        .map_err(unreadable)?
        .check()
        .map_err(unreadable)?;
    let pixmap = connection.generate_id().map_err(unreadable);
    let result = pixmap.and_then(|pixmap| {
        let named = connection
            .composite_name_window_pixmap(window, pixmap)
            .map_err(unreadable)?
            .check()
            .map_err(unreadable);
        let raw =
            named.and_then(|()| painted(connection, pixmap, rect, attributes.visual, deadline));
        let _ = connection.free_pixmap(pixmap);
        raw
    });
    let _ = connection.composite_unredirect_window(window, Redirect::AUTOMATIC);
    let _ = connection.flush();
    result
}

#[cfg(test)]
#[path = "composite_tests.rs"]
mod tests;
