//! The bound compositor's Wayland socket, connected only after its peer is
//! proven to be the bound compositor process by pid and start time, before
//! any byte is read.

use std::os::unix::net::UnixStream;

use agent_desktop_core::{AdapterError, ErrorCode};

use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::process::proc_stat::read_stat;
use crate::system::wayland::socket_peer::connect_peer;

/// A stream to the binding's Wayland socket whose peer is the bound
/// compositor process.
pub(crate) fn connect_bound(binding: &HyprlandBinding) -> Result<UnixStream, AdapterError> {
    let socket = binding.wayland_socket.as_path();
    let (stream, pid) = connect_peer(socket).map_err(|fault| fault.into_error(socket))?;
    let start_time = read_stat(pid).ok().flatten().map(|stat| stat.start_time);
    if pid != binding.pid || start_time != Some(binding.start_time) {
        return Err(display_cause_error(&DisplayCause::HyprlandMismatch {
            detail: format!(
                "Hyprland instance {} was bound to pid {}, but its Wayland socket {} is now served by pid {pid}",
                binding.signature,
                binding.pid,
                socket.display()
            ),
        }));
    }
    Ok(stream)
}

/// The refusal for a compositor that does not advertise `interface`, which
/// `need` names the use of.
pub(crate) fn unadvertised(interface: &str, need: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        format!("The compositor does not advertise {interface}, which {need} needs"),
    )
    .with_suggestion("Update Hyprland to a release that advertises it")
    .with_platform_detail(format!("missing Wayland global {interface}"))
}
