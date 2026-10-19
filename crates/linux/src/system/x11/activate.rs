use std::sync::Arc;
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
const CONFIRM_BUDGET: Duration = Duration::from_millis(500);
const EVENT_POLL: Duration = Duration::from_millis(2);

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

/// A connection ready to send: the root, the atom and the server time were
/// all read before the caller's deadline.
pub(crate) struct Prepared {
    connection: Arc<RustConnection>,
    root: u32,
    active: Atom,
    timestamp: u32,
}

/// The display server's current time, the stamp a request made now should
/// carry. A server only tells its time through an event, so a property on a
/// window made for the purpose is touched and the notification polled for
/// until `deadline`.
fn server_time(
    connection: &RustConnection,
    root: u32,
    deadline: agent_desktop_core::Deadline,
) -> Option<u32> {
    let property = atom(connection, b"WM_CLASS").ok()?;
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
        match connection.poll_for_event() {
            Ok(Some(Event::PropertyNotify(event))) => break Some(event.time),
            Ok(Some(_)) => {}
            Ok(None) if deadline.is_expired() => break None,
            Ok(None) => std::thread::sleep(EVENT_POLL.min(deadline.remaining())),
            Err(_) => break None,
        }
    };
    let _ = connection.destroy_window(window);
    let _ = connection.flush();
    stamp
}

/// Connects and reads everything a request needs. Nothing is sent: a worker
/// that outlives its deadline holds a connection and no request.
pub(crate) fn prepare(deadline: agent_desktop_core::Deadline) -> Result<Prepared, ActivateError> {
    let (connection, root) = connect()?;
    let active = atom(&connection, b"_NET_ACTIVE_WINDOW")?;
    let timestamp = server_time(&connection, root, deadline).ok_or(ActivateError::SendFailed)?;
    Ok(Prepared {
        connection: Arc::new(connection),
        root,
        active,
        timestamp,
    })
}

/// Asks the window manager to make `client` the active window, from the
/// caller's thread. The request is written here; a round trip then makes sure
/// the display server has taken it before the connection can close, because a
/// window manager can drop a request whose sender has already gone. That wait
/// runs on a worker that holds the connection, so a server that stops
/// answering costs the confirmation budget and the request still stands.
pub(crate) fn send(prepared: &Prepared, client: u32, source: u32) -> Result<(), ActivateError> {
    let message = activation_event(client, prepared.active, source, prepared.timestamp);
    let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    prepared
        .connection
        .send_event(false, prepared.root, mask, message)
        .map_err(|_| ActivateError::SendFailed)?;
    prepared
        .connection
        .flush()
        .map_err(|_| ActivateError::SendFailed)?;
    let connection = Arc::clone(&prepared.connection);
    if let Some(deadline) = deadline_of(CONFIRM_BUDGET) {
        let _ = bounded(deadline, None, move || {
            connection
                .get_input_focus()
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|_| ())
        });
    }
    Ok(())
}

/// The window the manager says is active, or `None` when it names none or
/// cannot be asked.
fn active_window() -> Option<u32> {
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

/// Prepares under the budget and then sends on this thread, so a server that
/// stopped answering costs the budget and no request is ever written after the
/// caller was told it failed.
pub(crate) fn request_activation(client: u32) -> Result<(), ActivateError> {
    let deadline = deadline_of(SERVER_TIME_BUDGET).ok_or(ActivateError::SendFailed)?;
    request_via(deadline, prepare, |prepared| {
        send(&prepared, client, ACTIVATION_SOURCE)
    })
}

/// The two phases apart: `prepare` runs on a worker the deadline can abandon,
/// `send` only on the calling thread and only with what `prepare` returned in
/// time.
pub(crate) fn request_via<P: Send + 'static>(
    deadline: agent_desktop_core::Deadline,
    prepare: impl FnOnce(agent_desktop_core::Deadline) -> Result<P, ActivateError> + Send + 'static,
    send: impl FnOnce(P) -> Result<(), ActivateError>,
) -> Result<(), ActivateError> {
    let prepared = bounded(deadline, Err(ActivateError::SendFailed), move || {
        prepare(deadline)
    })?;
    send(prepared)
}

/// `active_window` bounded by `budget`.
pub(crate) fn active_window_within(budget: Duration) -> Option<u32> {
    let deadline = deadline_of(budget)?;
    bounded(deadline, None, active_window)
}

#[cfg(test)]
#[path = "activate_tests.rs"]
mod tests;
