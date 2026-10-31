//! Which foreign toplevels belong to a process, by the Hyprland client each
//! one maps to.

use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::wlr_window::WlrWindow;
use crate::system::x11::control::control_error::ControlError;
use crate::system::x11::window::toplevel::Toplevel;
use crate::system::x11::window::window_kind::WindowKind;

/// Every toplevel of `pid`, as the close flow reads windows. The protocol
/// carries no window type, so a toplevel with a parent, as a dialog has,
/// counts as transient and the close flow treats it as a dialog.
pub(crate) fn owned_toplevels(
    windows: &[WlrWindow],
    clients: &[HyprClient],
    pid: u32,
) -> Vec<Toplevel> {
    windows
        .iter()
        .filter(|window| owner_of(window.address, clients) == Some(pid))
        .map(|window| Toplevel {
            id: window.id,
            title: window.title.clone(),
            kind: WindowKind::Normal,
            transient: window.child,
        })
        .collect()
}

/// Whether the toplevel with handle id `id` may receive a close request for
/// `pid`. A handle that is gone is `BadWindow`, which the close flow skips;
/// one now owned by another process is `StaleWindow`.
pub(crate) fn check_owner(
    id: u32,
    windows: &[WlrWindow],
    clients: &[HyprClient],
    pid: u32,
) -> Result<(), ControlError> {
    let window = windows
        .iter()
        .find(|window| window.id == id)
        .ok_or(ControlError::BadWindow)?;
    match owner_of(window.address, clients) {
        Some(owner) if owner == pid => Ok(()),
        Some(_) => Err(ControlError::StaleWindow),
        None => Err(ControlError::BadWindow),
    }
}

fn owner_of(address: u64, clients: &[HyprClient]) -> Option<u32> {
    clients
        .iter()
        .find(|client| client.address == address)
        .map(|client| client.pid)
}

#[cfg(test)]
#[path = "owned_toplevels_tests.rs"]
mod tests;
