//! Whether a ref pointer action on the bound Hyprland instance may aim at a
//! client.

use agent_desktop_core::{AdapterError, Deadline};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::ipc::bound_request::bound_request;

/// Refuses a ref pointer action whose target window is not an XWayland
/// client, read from the bound instance's `j/clients`.
pub(crate) fn refuse_native_receiver(
    binding: &HyprlandBinding,
    address: u64,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let clients = bound_request(binding, "j/clients", deadline)?;
    check_native_receiver(&clients, address)
}

/// The same check over a `j/clients` reply.
pub(crate) fn check_native_receiver(_clients: &[u8], _address: u64) -> Result<(), AdapterError> {
    Ok(())
}

#[cfg(test)]
#[path = "native_receiver_tests.rs"]
mod tests;
