//! A Wayland connection to the bound compositor. The socket's peer must be
//! the bound compositor process, checked by pid and start time before any
//! byte is read.

use agent_desktop_core::AdapterError;
use wayland_client::Connection;

use super::socket_peer::connect;
use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::hyprland::hyprland_binding::HyprlandBinding;
use crate::system::process::proc_stat::read_stat;

/// Connects to the binding's Wayland socket and refuses a socket served by
/// any process other than the bound compositor.
pub(crate) fn connect_bound(binding: &HyprlandBinding) -> Result<Connection, AdapterError> {
    let socket = binding.wayland_socket.display().to_string();
    let missing = || {
        display_cause_error(&DisplayCause::Missing {
            address: socket.clone(),
        })
    };
    let (stream, pid) = connect(&binding.wayland_socket).map_err(|_| missing())?;
    let start_time = read_stat(pid).ok().flatten().map(|stat| stat.start_time);
    if pid != binding.pid || start_time != Some(binding.start_time) {
        return Err(display_cause_error(&DisplayCause::HyprlandMismatch {
            detail: format!(
                "Hyprland instance {} was bound to pid {}, but its Wayland socket {socket} is now served by pid {pid}",
                binding.signature, binding.pid
            ),
        }));
    }
    Connection::from_socket(stream).map_err(|_| missing())
}
