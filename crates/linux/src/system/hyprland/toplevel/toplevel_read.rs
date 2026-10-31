//! Reads the bound compositor's toplevels over one connection to its Wayland
//! socket. The socket's peer must be the bound compositor process, checked
//! by pid and start time before any byte is read. The toplevel list is the
//! shared `ext_foreign_toplevel_list_v1`, found by its registry entry alone;
//! the Hyprland mapping manager is bound only here, after the binding paired
//! this socket with the instance's IPC socket.

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

use super::mapped_toplevels::MappedToplevels;
use super::toplevel_join::join;
use super::toplevel_view::ToplevelView;
use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::protocol::MAPPING_INTERFACE;
use crate::system::hyprland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_mapping_manager_v1::HyprlandToplevelMappingManagerV1;
use crate::system::process::proc_stat::read_stat;
use crate::system::wayland::link::Link;
use crate::system::wayland::socket_peer::connect_peer;
use crate::system::wayland::toplevel_list::{LIST_INTERFACE, advertised_list};
use crate::system::wayland::wayland_fault::WaylandFault;

/// Every toplevel of the bound compositor that has an identifier and a
/// Hyprland address, in the compositor's announce order.
pub(crate) fn read_toplevels(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Vec<ToplevelView>, AdapterError> {
    let socket = binding.wayland_socket.as_path();
    let fault = |fault: WaylandFault| fault.into_error(socket);
    let (stream, pid) = connect_peer(socket).map_err(fault)?;
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
    let mut state = MappedToplevels::default();
    let mut link = Link::over(stream, &mut state, deadline).map_err(fault)?;
    let list = advertised_list(&state.exchange)
        .cloned()
        .ok_or_else(|| unadvertised(LIST_INTERFACE))?;
    let mapping = state
        .exchange
        .find(MAPPING_INTERFACE, 1)
        .cloned()
        .ok_or_else(|| unadvertised(MAPPING_INTERFACE))?;
    let _list: ExtForeignToplevelListV1 = link.bind(&list, 1, ());
    let manager: HyprlandToplevelMappingManagerV1 = link.bind(&mapping, 1, ());
    link.roundtrip(&mut state).map_err(fault)?;
    let handle = link.handle();
    for (index, toplevel) in state.toplevels.iter().enumerate() {
        let _mapping = manager.get_window_for_toplevel(&toplevel.handle, &handle, index);
    }
    let requested = state.toplevels.len();
    link.roundtrip(&mut state).map_err(fault)?;
    link.wait(&mut state, |state| state.answered(requested))
        .map_err(fault)?;
    join(&state.records()).ok_or_else(|| {
        AdapterError::internal("Two foreign toplevel handles named one Hyprland window")
            .with_suggestion("Retry once the compositor's window list settles")
    })
}

fn unadvertised(interface: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        format!("The compositor does not advertise {interface}, which listing its windows needs"),
    )
    .with_suggestion("Update Hyprland to a release that advertises it")
    .with_platform_detail(format!("missing Wayland global {interface}"))
}

#[cfg(test)]
#[path = "toplevel_read_tests.rs"]
mod tests;
