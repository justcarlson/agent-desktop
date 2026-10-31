//! One open connection to the bound compositor's Wayland socket that holds
//! its foreign toplevel handles. The socket's peer must be the bound
//! compositor process, checked by pid and start time before any byte is
//! read. The toplevel list is the shared `ext_foreign_toplevel_list_v1`,
//! found by its registry entry alone; the Hyprland mapping manager is bound
//! only here, after the binding paired this socket with the instance's IPC
//! socket. The connection stays open so a capture can pass one of its
//! handles to a request on its own queue.

use agent_desktop_core::{AdapterError, Deadline};
use wayland_client::Connection;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

use super::bound_socket::{connect_bound, unadvertised};
use super::mapped_toplevels::MappedToplevels;
use super::mapping_target::MappingTarget;
use super::toplevel_join::join;
use super::toplevel_view::ToplevelView;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::protocol::MAPPING_INTERFACE;
use crate::system::hyprland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_mapping_manager_v1::HyprlandToplevelMappingManagerV1;
use crate::system::wayland::link::Link;
use crate::system::wayland::toplevel_list::{LIST_INTERFACE, advertised_list};
use crate::system::wayland::wayland_fault::WaylandFault;

const NEED: &str = "listing its windows";

/// The bound compositor's toplevels as one connection read them, with
/// their handles still live on that connection.
pub(crate) struct ToplevelReader {
    link: Link<MappedToplevels>,
    state: MappedToplevels,
}

impl ToplevelReader {
    /// Connects to the bound compositor and reads every toplevel and the
    /// Hyprland address the mapping manager gives it.
    pub(crate) fn open(
        binding: &HyprlandBinding,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let socket = binding.wayland_socket.as_path();
        let fault = |fault: WaylandFault| fault.into_error(socket);
        let stream = connect_bound(binding)?;
        let mut state = MappedToplevels::default();
        let mut link = Link::over(stream, &mut state, deadline).map_err(fault)?;
        let list = advertised_list(&state.exchange)
            .cloned()
            .ok_or_else(|| unadvertised(LIST_INTERFACE, NEED))?;
        let mapping = state
            .exchange
            .find(MAPPING_INTERFACE, 1)
            .cloned()
            .ok_or_else(|| unadvertised(MAPPING_INTERFACE, NEED))?;
        let _list: ExtForeignToplevelListV1 = link.bind(&list, 1, ());
        let manager: HyprlandToplevelMappingManagerV1 = link.bind(&mapping, 1, ());
        link.roundtrip(&mut state).map_err(fault)?;
        let handle = link.handle();
        for (index, toplevel) in state.toplevels.iter().enumerate() {
            let _mapping = manager.get_window_for_toplevel(
                &toplevel.handle,
                &handle,
                MappingTarget::Ext(index),
            );
        }
        let requested = state.toplevels.len();
        link.roundtrip(&mut state).map_err(fault)?;
        link.wait(&mut state, |state| state.answered(requested))
            .map_err(fault)?;
        Ok(Self { link, state })
    }

    /// Every toplevel that has an identifier and a Hyprland address, in the
    /// compositor's announce order.
    pub(crate) fn views(&self) -> Result<Vec<ToplevelView>, AdapterError> {
        join(&self.state.records()).ok_or_else(|| {
            AdapterError::internal("Two foreign toplevel handles named one Hyprland window")
                .with_suggestion("Retry once the compositor's window list settles")
        })
    }

    /// The live handle of the toplevel `identifier` names, or `None` when
    /// this read has no such toplevel or the compositor closed it.
    pub(crate) fn ext_handle(&self, identifier: &str) -> Option<ExtForeignToplevelHandleV1> {
        if identifier.is_empty() {
            return None;
        }
        self.state
            .toplevels
            .iter()
            .find(|toplevel| toplevel.identifier == identifier)
            .map(|toplevel| toplevel.handle.clone())
    }

    /// The connection the handles live on.
    pub(crate) fn connection(&self) -> &Connection {
        self.link.connection()
    }
}

#[cfg(test)]
#[path = "toplevel_reader_tests.rs"]
mod tests;
