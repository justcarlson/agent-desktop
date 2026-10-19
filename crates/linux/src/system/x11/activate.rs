use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt, CreateWindowAux, EventMask, PropMode,
    WindowClass,
};
use x11rb::rust_connection::RustConnection;

use super::activate_error::ActivateError;
use super::stacking::bounded;

/// The source indication of the request: 2 says a pager or automation tool
/// is asking, the form `xdotool windowactivate` sends. Openbox and mutter
/// accepted 0, 1 and 2 alike when they honoured the request at all (`L27-3`,
/// `L27-10`).
pub(crate) const ACTIVATION_SOURCE: u32 = 2;

/// How long a window manager gets to make the window active before the
/// request counts as ignored. Openbox answered within 2 to 4 ms (`L27-3`).
pub(crate) const ACTIVATION_GRACE: Duration = Duration::from_millis(2_000);

const FORMAT_LONGS: u8 = 32;
const MESSAGE_LONGS: usize = 5;
const SERVER_TIME_BUDGET: Duration = Duration::from_millis(1_000);

/// The five data longs of a `_NET_ACTIVE_WINDOW` request: the source
/// indication, the time of the user action, and the window that is active
/// now, which 0 leaves unnamed.
pub(crate) fn activation_data(source: u32, timestamp: u32) -> [u32; MESSAGE_LONGS] {
    [source, timestamp, 0, 0, 0]
}

/// The client message that asks for `client` to become active.
pub(crate) fn activation_event(
    client: u32,
    active_window: Atom,
    source: u32,
    timestamp: u32,
) -> ClientMessageEvent {
    ClientMessageEvent::new(
        FORMAT_LONGS,
        client,
        active_window,
        activation_data(source, timestamp),
    )
}

fn connect() -> Result<(RustConnection, u32), ActivateError> {
    let display = match crate::system::display_server::session_server() {
        crate::system::display_server::DisplayServer::X11(display) => display,
        _ => return Err(ActivateError::NoDisplay),
    };
    let (connection, screen) =
        RustConnection::connect(Some(&display)).map_err(|_| ActivateError::NoDisplay)?;
    let root = connection
        .setup()
        .roots
        .get(screen)
        .map(|screen| screen.root)
        .ok_or(ActivateError::NoDisplay)?;
    Ok((connection, root))
}

fn atom(connection: &RustConnection, name: &[u8]) -> Result<Atom, ActivateError> {
    connection
        .intern_atom(false, name)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|reply| reply.atom)
        .ok_or(ActivateError::SendFailed)
}

/// Asks the window manager to make `client` the active window.
pub(crate) fn activate(client: u32, source: u32, timestamp: u32) -> Result<(), ActivateError> {
    let (connection, root) = connect()?;
    let active = atom(&connection, b"_NET_ACTIVE_WINDOW")?;
    let message = activation_event(client, active, source, timestamp);
    let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    connection
        .send_event(false, root, mask, message)
        .map_err(|_| ActivateError::SendFailed)?;
    connection
        .get_input_focus()
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|_| ())
        .ok_or(ActivateError::SendFailed)
}

/// The display server's current time, the stamp a request made now should
/// carry. A server only tells its time through an event, so a property on a
/// window made for the purpose is touched and the notification read.
pub(crate) fn server_time() -> Option<u32> {
    let (connection, root) = connect().ok()?;
    let property = atom(&connection, b"WM_CLASS").ok()?;
    let window = connection.generate_id().ok()?;
    connection
        .create_window(
            0,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .ok()?;
    connection
        .change_property(
            PropMode::APPEND,
            window,
            property,
            AtomEnum::STRING,
            8,
            0,
            &[],
        )
        .ok()?;
    connection.flush().ok()?;
    let stamp = loop {
        if let Event::PropertyNotify(event) = connection.wait_for_event().ok()? {
            break Some(event.time);
        }
    };
    let _ = connection.destroy_window(window);
    let _ = connection.flush();
    stamp
}

/// The window the manager says is active, or `None` when it names none or
/// cannot be asked.
pub(crate) fn active_window() -> Option<u32> {
    let (connection, root) = connect().ok()?;
    let active = atom(&connection, b"_NET_ACTIVE_WINDOW").ok()?;
    let reply = connection
        .get_property(false, root, active, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let bytes: [u8; 4] = reply.value.get(..4)?.try_into().ok()?;
    Some(u32::from_ne_bytes(bytes)).filter(|window| *window != 0)
}

fn deadline_of(budget: Duration) -> Option<agent_desktop_core::Deadline> {
    agent_desktop_core::Deadline::from_duration(budget).ok()
}

/// Sends the request with a real server time, bounded: a server that has
/// stopped answering costs the budget, never the caller's whole deadline.
pub(crate) fn request_activation(client: u32) -> Result<(), ActivateError> {
    let deadline = deadline_of(SERVER_TIME_BUDGET).ok_or(ActivateError::SendFailed)?;
    bounded(deadline, Err(ActivateError::SendFailed), move || {
        let stamp = server_time().ok_or(ActivateError::SendFailed)?;
        activate(client, ACTIVATION_SOURCE, stamp)
    })
}

/// `active_window` bounded by `budget`.
pub(crate) fn active_window_within(budget: Duration) -> Option<u32> {
    let deadline = deadline_of(budget)?;
    bounded(deadline, None, active_window)
}

#[cfg(test)]
#[path = "activate_tests.rs"]
mod tests;
