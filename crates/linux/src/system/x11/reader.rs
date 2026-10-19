use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, GetPropertyReply, Window};
use x11rb::rust_connection::RustConnection;

use super::snapshot::{decode_cardinals, decode_frame_extents, decode_title};
use super::stacking::{READ_BUDGET, bounded, route};
use super::stacking_read::StackingRead;
use super::{X11Client, X11Source, X11State};
use crate::system::display_server::session_server;
use crate::tree::Extents;
use agent_desktop_core::Deadline;

const MAX_CLIENTS: usize = 256;
const MAX_LIST_LONGS: u32 = 4_096;
const MAX_TITLE_LONGS: u32 = 1_024;

/// Reads window-manager properties from the display named by `DISPLAY`. It
/// connects on every read and keeps nothing: a connection kept between calls
/// would outlive a server that has stopped answering.
pub(crate) struct DisplayReader;

#[path = "stacking_wire.rs"]
mod stacking_wire;

impl super::stacking_source::StackingSource for DisplayReader {
    fn read(&self, target_client: u32) -> StackingRead {
        route(session_server(), |display| {
            let Ok(deadline) = Deadline::from_duration(READ_BUDGET) else {
                return StackingRead::Unavailable;
            };
            bounded(deadline, StackingRead::Unavailable, move || {
                stacking_wire::read_stacking(&display, target_client)
            })
        })
    }

    fn clients(&self) -> X11State {
        let Ok(deadline) = Deadline::from_duration(READ_BUDGET) else {
            return X11State::Unavailable;
        };
        bounded(deadline, X11State::Unavailable, || X11Source::read(&Self))
    }
}

struct Atoms {
    wm_check: Atom,
    active: Atom,
    client_list: Atom,
    pid: Atom,
    name: Atom,
    utf8: Atom,
    frame_extents: Atom,
}

fn display_to_read() -> Option<String> {
    match crate::system::display_server::session_server() {
        crate::system::display_server::DisplayServer::X11(display) => Some(display),
        _ => None,
    }
}

fn atom(connection: &RustConnection, name: &[u8]) -> Option<Atom> {
    Some(connection.intern_atom(false, name).ok()?.reply().ok()?.atom)
}

fn atoms(connection: &RustConnection) -> Option<Atoms> {
    Some(Atoms {
        wm_check: atom(connection, b"_NET_SUPPORTING_WM_CHECK")?,
        active: atom(connection, b"_NET_ACTIVE_WINDOW")?,
        client_list: atom(connection, b"_NET_CLIENT_LIST")?,
        pid: atom(connection, b"_NET_WM_PID")?,
        name: atom(connection, b"_NET_WM_NAME")?,
        utf8: atom(connection, b"UTF8_STRING")?,
        frame_extents: atom(connection, b"_NET_FRAME_EXTENTS")?,
    })
}

fn property(
    connection: &RustConnection,
    window: Window,
    name: Atom,
    kind: impl Into<Atom>,
    longs: u32,
) -> Option<GetPropertyReply> {
    connection
        .get_property(false, window, name, kind, 0, longs)
        .ok()?
        .reply()
        .ok()
}

fn cardinals(reply: Option<GetPropertyReply>) -> Option<Vec<u32>> {
    let reply = reply?;
    decode_cardinals(reply.format, &reply.value)
}

fn bounds(connection: &RustConnection, root: Window, window: Window) -> Option<Extents> {
    let origin = connection
        .translate_coordinates(window, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    let size = connection.get_geometry(window).ok()?.reply().ok()?;
    Some(Extents {
        x: i32::from(origin.dst_x),
        y: i32::from(origin.dst_y),
        width: i32::from(size.width),
        height: i32::from(size.height),
    })
}

fn client(connection: &RustConnection, atoms: &Atoms, root: Window, window: Window) -> X11Client {
    let pid = cardinals(property(
        connection,
        window,
        atoms.pid,
        AtomEnum::CARDINAL,
        1,
    ))
    .and_then(|values| values.first().copied());
    let wide = property(connection, window, atoms.name, atoms.utf8, MAX_TITLE_LONGS);
    let narrow = property(
        connection,
        window,
        AtomEnum::WM_NAME.into(),
        AtomEnum::STRING,
        MAX_TITLE_LONGS,
    );
    let extents = property(
        connection,
        window,
        atoms.frame_extents,
        AtomEnum::CARDINAL,
        4,
    )
    .and_then(|reply| decode_frame_extents(reply.format, &reply.value));
    X11Client {
        id: window,
        pid,
        title: decode_title(
            wide.as_ref()
                .map(|reply| (reply.format, reply.value.as_slice())),
            narrow
                .as_ref()
                .map(|reply| (reply.format, reply.value.as_slice())),
        ),
        extents,
        bounds: bounds(connection, root, window),
    }
}

fn read_display(display: &str) -> X11State {
    let Ok((connection, screen)) = RustConnection::connect(Some(display)) else {
        return X11State::Unavailable;
    };
    let Some(root) = connection
        .setup()
        .roots
        .get(screen)
        .map(|screen| screen.root)
    else {
        return X11State::Unavailable;
    };
    let Some(atoms) = atoms(&connection) else {
        return X11State::Unavailable;
    };
    let manager = cardinals(property(
        &connection,
        root,
        atoms.wm_check,
        AtomEnum::WINDOW,
        1,
    ));
    if manager.is_none_or(|values| values.is_empty()) {
        return X11State::NoWindowManager;
    }
    let active = cardinals(property(
        &connection,
        root,
        atoms.active,
        AtomEnum::WINDOW,
        1,
    ))
    .and_then(|values| values.first().copied())
    .filter(|window| *window != 0);
    let Some(listed) = cardinals(property(
        &connection,
        root,
        atoms.client_list,
        AtomEnum::WINDOW,
        MAX_LIST_LONGS,
    )) else {
        return X11State::Unavailable;
    };
    let clients = listed
        .into_iter()
        .take(MAX_CLIENTS)
        .map(|window| client(&connection, &atoms, root, window))
        .collect();
    X11State::Ready { active, clients }
}

impl X11Source for DisplayReader {
    fn read(&self) -> X11State {
        display_to_read().map_or(X11State::Unavailable, |display| read_display(&display))
    }
}
