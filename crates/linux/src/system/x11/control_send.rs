use agent_desktop_core::Deadline;
use x11rb::errors::ReplyError;
use x11rb::protocol::xproto::{Atom, ClientMessageEvent, ConnectionExt, EventMask};
use x11rb::rust_connection::RustConnection;

use super::activate::{request_via, server_time};
use super::activate_error::ActivateError;
use super::client_message::{
    FORMAT_LONGS, MessageData, activate, close, iconify, maximize, move_resize,
};
use super::control::{ControlError, pid_of};
use super::control_display::{atoms, connect};
use super::control_grab::ServerGrab;
use super::control_owner::CloseOwner;
use super::stacking::bounded;
use crate::system::liveness;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WindowWrite {
    MoveResize {
        x: Option<i32>,
        y: Option<i32>,
        width: Option<u32>,
        height: Option<u32>,
    },
    Iconify,
    SetMaximized(bool),
}

/// A request connected, checked against a live window and built, but not yet
/// sent: a worker that outlives its deadline holds this and sends nothing.
struct Ready {
    connection: RustConnection,
    root: u32,
    event: ClientMessageEvent,
    close_owner: Option<(CloseOwner, Atom)>,
}

fn names_of(write: &WindowWrite) -> &'static [&'static [u8]] {
    match write {
        WindowWrite::MoveResize { .. } => &[b"_NET_MOVERESIZE_WINDOW"],
        WindowWrite::Iconify => &[b"WM_CHANGE_STATE"],
        WindowWrite::SetMaximized(_) => &[
            b"_NET_WM_STATE",
            b"_NET_WM_STATE_MAXIMIZED_VERT",
            b"_NET_WM_STATE_MAXIMIZED_HORZ",
        ],
    }
}

fn data_of(write: &WindowWrite, atoms: &[Atom]) -> Option<MessageData> {
    match (write, atoms) {
        (
            WindowWrite::MoveResize {
                x,
                y,
                width,
                height,
            },
            [_],
        ) => Some(move_resize(*x, *y, *width, *height)),
        (WindowWrite::Iconify, [_]) => Some(iconify()),
        (WindowWrite::SetMaximized(on), [_, vertical, horizontal]) => {
            Some(maximize(*on, *vertical, *horizontal))
        }
        _ => None,
    }
}

/// The client message that carries `write` to `window`.
fn event_of(window: u32, write: &WindowWrite, atoms: &[Atom]) -> Option<ClientMessageEvent> {
    let kind = *atoms.first()?;
    Some(message(window, kind, data_of(write, atoms)?))
}

fn classify(error: ReplyError) -> ControlError {
    match error {
        ReplyError::X11Error(_) => ControlError::BadWindow,
        ReplyError::ConnectionError(_) => ControlError::SendFailed,
    }
}

fn live(connection: &RustConnection, window: u32) -> Result<(), ControlError> {
    let cookie = connection
        .get_window_attributes(window)
        .map_err(|_| ControlError::SendFailed)?;
    cookie.reply().map(|_| ()).map_err(classify)
}

fn prepare_write(
    window: u32,
    write: WindowWrite,
) -> impl FnOnce(Deadline) -> Result<Result<Ready, ControlError>, ActivateError> + Send + 'static {
    move |_| {
        let (connection, root) = connect()?;
        if let Err(error) = live(&connection, window) {
            return Ok(Err(error));
        }
        let interned = atoms(&connection, names_of(&write)).ok_or(ActivateError::SendFailed)?;
        let event = event_of(window, &write, &interned).ok_or(ActivateError::SendFailed)?;
        Ok(Ok(Ready {
            connection,
            root,
            event,
            close_owner: None,
        }))
    }
}

fn prepare_close(
    window: u32,
    owner: CloseOwner,
) -> impl FnOnce(Deadline) -> Result<Result<Ready, ControlError>, ActivateError> + Send + 'static {
    move |deadline| {
        let (connection, root) = connect()?;
        if let Err(error) = live(&connection, window) {
            return Ok(Err(error));
        }
        let kind = single(&connection, b"_NET_CLOSE_WINDOW")?;
        let pid_atom = single(&connection, b"_NET_WM_PID")?;
        let stamp = server_time(&connection, root, deadline).ok_or(ActivateError::SendFailed)?;
        Ok(Ok(Ready {
            connection,
            root,
            event: message(window, kind, close(stamp)),
            close_owner: Some((owner, pid_atom)),
        }))
    }
}

/// Runs `send` on a thread the deadline can abandon: a round trip to a server
/// that stopped answering cannot be interrupted. Nothing is written once the
/// deadline has passed, and a send that was started and never confirmed is
/// uncertain, not failed.
fn within_deadline<P: Send + 'static>(
    deadline: Deadline,
    ready: P,
    send: impl FnOnce(P) -> Result<(), ControlError> + Send + 'static,
) -> Result<(), ControlError> {
    bounded(deadline, Err(ControlError::SendFailed), move || {
        if deadline.is_expired() {
            return Err(ControlError::TimedOut);
        }
        send(ready)
    })
}

fn emit_within(deadline: Deadline) -> impl FnOnce(Ready) -> Result<(), ControlError> {
    move |ready| within_deadline(deadline, ready, move |ready| emit(ready, deadline))
}

fn emit(ready: Ready, deadline: Deadline) -> Result<(), ControlError> {
    if let Some((owner, atom)) = &ready.close_owner {
        return owner.send(
            || ServerGrab::acquire(&ready.connection, deadline),
            || pid_of(&ready.connection, ready.event.window, *atom),
            liveness::identity,
            || emit_event(&ready, deadline),
        );
    }
    emit_event(&ready, deadline)
}

fn emit_event(ready: &Ready, deadline: Deadline) -> Result<(), ControlError> {
    if deadline.is_expired() {
        return Err(ControlError::TimedOut);
    }
    let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    ready
        .connection
        .send_event(false, ready.root, mask, ready.event)
        .map_err(|_| ControlError::SendFailed)?
        .check()
        .map_err(classify)
}

/// Prepares under `deadline` and sends from this thread only: a window
/// manager that stopped answering costs the budget and no request is written
/// after the caller was told it failed.
pub(super) fn deliver<P: Send + 'static>(
    deadline: Deadline,
    prepare: impl FnOnce(Deadline) -> Result<Result<P, ControlError>, ActivateError> + Send + 'static,
    send: impl FnOnce(P) -> Result<(), ControlError>,
) -> Result<(), ControlError> {
    let mut outcome = Err(ControlError::SendFailed);
    let prepared = request_via(deadline, prepare, |prepared| {
        outcome = prepared.and_then(send);
        Ok(())
    });
    match prepared {
        Ok(()) => outcome,
        Err(ActivateError::NoDisplay) => Err(ControlError::NoDisplay),
        Err(ActivateError::SendFailed) if deadline.is_expired() => Err(ControlError::TimedOut),
        Err(ActivateError::SendFailed) => Err(ControlError::SendFailed),
    }
}

/// Asks the window manager to apply `write` to `window`. A destroyed window
/// is `BadWindow`, where `wmctrl` answered success.
pub(crate) fn write_window(
    window: u32,
    write: &WindowWrite,
    deadline: Deadline,
) -> Result<(), ControlError> {
    deliver(
        deadline,
        prepare_write(window, write.clone()),
        emit_within(deadline),
    )
}

fn prepare_raise(
    window: u32,
) -> impl FnOnce(Deadline) -> Result<Result<Ready, ControlError>, ActivateError> + Send + 'static {
    move |_| {
        let (connection, root) = connect()?;
        if let Err(error) = live(&connection, window) {
            return Ok(Err(error));
        }
        let kind = single(&connection, b"_NET_ACTIVE_WINDOW")?;
        Ok(Ok(Ready {
            connection,
            root,
            event: message(window, kind, activate()),
            close_owner: None,
        }))
    }
}

fn single(connection: &RustConnection, name: &[u8]) -> Result<Atom, ActivateError> {
    atoms(connection, &[name])
        .and_then(|got| got.first().copied())
        .ok_or(ActivateError::SendFailed)
}

fn message(window: u32, kind: Atom, data: MessageData) -> ClientMessageEvent {
    ClientMessageEvent::new(FORMAT_LONGS, window, kind, data)
}

/// Asks the window manager to activate `window`, which un-minimizes it
/// under EWMH. The caller judges the effect by the hidden flag.
pub(crate) fn raise_window(window: u32, deadline: Deadline) -> Result<(), ControlError> {
    deliver(deadline, prepare_raise(window), emit_within(deadline))
}

/// Asks the window manager to close `window`, the request `wmctrl -c` sends.
pub(super) fn close_window(
    window: u32,
    pid: u32,
    instance: &str,
    deadline: Deadline,
) -> Result<(), ControlError> {
    let owner = CloseOwner {
        pid,
        instance: instance.to_owned(),
    };
    deliver(
        deadline,
        prepare_close(window, owner),
        emit_within(deadline),
    )
}

#[cfg(test)]
#[path = "control_send_tests.rs"]
mod tests;
