//! An IPC request to the bound instance only. The reply must come from the
//! compositor process the binding names, with the same start time, so a
//! restarted compositor or a socket taken over by another process is refused
//! instead of read.

use agent_desktop_core::{AdapterError, Deadline};

use super::hyprland_binding::HyprlandBinding;
use super::ipc::{self, IpcFault};
use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::process::proc_stat::read_stat;

/// Sends `command` to the bound instance and returns the reply body.
pub(crate) fn bound_request(
    binding: &HyprlandBinding,
    command: &str,
    deadline: Deadline,
) -> Result<Vec<u8>, AdapterError> {
    let reply = ipc::request(&binding.ipc_socket, command, deadline)
        .map_err(|fault| fault_error(binding, command, fault))?;
    let start_time = read_stat(reply.pid)
        .ok()
        .flatten()
        .map(|stat| stat.start_time);
    if reply.pid != binding.pid || start_time != Some(binding.start_time) {
        return Err(display_cause_error(&DisplayCause::HyprlandMismatch {
            detail: format!(
                "Hyprland instance {} was bound to pid {}, but its IPC socket {} is now served by pid {}",
                binding.signature,
                binding.pid,
                binding.ipc_socket.display(),
                reply.pid
            ),
        }));
    }
    Ok(reply.body)
}

fn fault_error(binding: &HyprlandBinding, command: &str, fault: IpcFault) -> AdapterError {
    match fault {
        IpcFault::Silent => display_cause_error(&DisplayCause::CompositorSilent {
            socket: binding.ipc_socket.display().to_string(),
        }),
        IpcFault::Gone => display_cause_error(&DisplayCause::Missing {
            address: format!(
                "Hyprland instance {} at {}",
                binding.signature,
                binding.ipc_socket.display()
            ),
        }),
        IpcFault::Oversized => AdapterError::internal(format!(
            "The Hyprland reply to {command} was larger than {} bytes",
            ipc::REPLY_CAP
        )),
    }
}

#[cfg(test)]
#[path = "bound_request_tests.rs"]
mod tests;
