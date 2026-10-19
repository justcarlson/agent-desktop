use x11rb::protocol::xproto::ConnectionExt;
use x11rb::rust_connection::RustConnection;

use super::MonitorFault;
use super::grab_error::GrabError;

pub(crate) fn selection_name(screen: usize) -> String {
    format!("_NET_WM_CM_S{screen}")
}

pub(crate) fn is_compositing(owner: u32) -> bool {
    owner != x11rb::NONE
}

fn unreadable<E>(_: E) -> GrabError {
    GrabError::Fault(MonitorFault::Unreadable)
}

pub(crate) fn read(connection: &RustConnection, screen: usize) -> Result<bool, GrabError> {
    let atom = connection
        .intern_atom(false, selection_name(screen).as_bytes())
        .map_err(unreadable)?
        .reply()
        .map_err(unreadable)?
        .atom;
    let owner = connection
        .get_selection_owner(atom)
        .map_err(unreadable)?
        .reply()
        .map_err(unreadable)?
        .owner;
    Ok(is_compositing(owner))
}

#[cfg(test)]
#[path = "compositing_tests.rs"]
mod tests;
