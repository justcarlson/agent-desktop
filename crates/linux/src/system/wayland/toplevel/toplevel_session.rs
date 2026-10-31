//! One connection to the bound compositor's Wayland socket that lists its
//! toplevels over both foreign toplevel protocols, maps each handle to a
//! Hyprland address, and activates one through the wlr handle. The
//! connection is peer-checked by `connect_bound`, and every read carries a
//! deadline.

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, EventQueue};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;

use super::chosen_globals::{ChosenGlobals, choose};
use super::mapping_target::MappingTarget;
use super::toplevel_join::join;
use super::toplevel_state::ToplevelState;
use super::toplevel_view::ToplevelView;
use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::hyprland::hyprland_binding::HyprlandBinding;
use crate::system::wayland::dispatch::dispatch_until;
use crate::system::wayland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_mapping_manager_v1::HyprlandToplevelMappingManagerV1;
use crate::system::wayland::bound_connection::connect_bound;
use crate::system::wayland::wayland_fault::WaylandFault;

/// A live toplevel connection to the bound compositor.
pub(crate) struct ToplevelSession {
    connection: Connection,
    queue: EventQueue<ToplevelState>,
    state: ToplevelState,
    mapping: HyprlandToplevelMappingManagerV1,
    seat: Option<WlSeat>,
    requested: (usize, usize),
    socket: String,
}

impl ToplevelSession {
    /// Connects to the binding's Wayland socket, binds the toplevel globals
    /// and waits until every toplevel handle is mapped to an address.
    pub(crate) fn open(
        binding: &HyprlandBinding,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let socket = binding.wayland_socket.display().to_string();
        let connection = connect_bound(binding)?;
        let mut queue = connection.new_event_queue::<ToplevelState>();
        let handle = queue.handle();
        let registry = connection.display().get_registry(&handle, ());
        let _sync = connection.display().sync(&handle, ());
        let mut state = ToplevelState::default();
        dispatch_until(&mut queue, &mut state, |state| state.synced >= 1, deadline)
            .map_err(|fault| fault_error(&socket, fault))?;
        let chosen = choose(&state.globals).map_err(unsupported)?;
        let (mapping, seat) = bind_globals(&registry, &chosen, &queue);
        let mut session = Self {
            connection,
            queue,
            state,
            mapping,
            seat,
            requested: (0, 0),
            socket,
        };
        session.settle(deadline)?;
        Ok(session)
    }

    /// Every toplevel the compositor announced, joined across both
    /// protocols by address.
    pub(crate) fn views(&self) -> Result<Vec<ToplevelView>, AdapterError> {
        let ext: Vec<_> = self
            .state
            .ext
            .iter()
            .map(|(_, record)| record.clone())
            .collect();
        let wlr: Vec<_> = self
            .state
            .wlr
            .iter()
            .map(|(_, record)| record.clone())
            .collect();
        join(&ext, &wlr).ok_or_else(|| {
            AdapterError::internal("Two foreign toplevel handles named one Hyprland window")
                .with_suggestion("Retry once the compositor's window list settles")
        })
    }

    /// The peer-checked connection, for other queues on the same socket.
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The live ext handle whose identifier is `identifier`. A handle the
    /// compositor closed has no identifier left and is never returned.
    pub(crate) fn ext_handle(&self, identifier: &str) -> Option<ExtForeignToplevelHandleV1> {
        self.state
            .ext
            .iter()
            .find(|(_, record)| !record.identifier.is_empty() && record.identifier == identifier)
            .map(|(handle, _)| handle.clone())
    }

    /// Sends `activate` on the one wlr handle mapped to `address`.
    pub(crate) fn activate(&mut self, address: u64) -> Result<(), AdapterError> {
        let views = self.views()?;
        let index = views
            .iter()
            .find(|view| view.address == address)
            .and_then(|view| view.wlr_index)
            .ok_or_else(|| {
                AdapterError::new(
                    ErrorCode::ActionFailed,
                    "The window has no foreign toplevel handle that can activate it",
                )
                .with_suggestion("Run 'list-windows' to confirm the window, then retry")
            })?;
        let seat = self.seat.as_ref().ok_or_else(|| unsupported("wl_seat"))?;
        if let Some((handle, _)) = self.state.wlr.get(index) {
            handle.activate(seat);
        }
        self.queue
            .flush()
            .map_err(|_| fault_error(&self.socket, WaylandFault::Gone))
    }

    /// Requests a mapping for every new handle and waits until the
    /// compositor has answered everything sent so far.
    pub(crate) fn settle(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let handle = self.queue.handle();
        let base = self.state.synced;
        let _announced = self.connection.display().sync(&handle, ());
        dispatch_until(
            &mut self.queue,
            &mut self.state,
            |state| state.synced > base,
            deadline,
        )
        .map_err(|fault| fault_error(&self.socket, fault))?;
        let (ext_seen, wlr_seen) = self.requested;
        for (index, (toplevel, _)) in self.state.ext.iter().enumerate().skip(ext_seen) {
            self.mapping
                .get_window_for_toplevel(toplevel, &handle, MappingTarget::Ext(index));
        }
        for (index, (toplevel, _)) in self.state.wlr.iter().enumerate().skip(wlr_seen) {
            self.mapping
                .get_window_for_toplevel_wlr(toplevel, &handle, MappingTarget::Wlr(index));
        }
        let requested = (self.state.ext.len(), self.state.wlr.len());
        self.requested = requested;
        let _mapped = self.connection.display().sync(&handle, ());
        dispatch_until(
            &mut self.queue,
            &mut self.state,
            |state| state.synced > base + 1 && state.mapped(requested),
            deadline,
        )
        .map_err(|fault| fault_error(&self.socket, fault))
    }
}

fn bind_globals(
    registry: &WlRegistry,
    chosen: &ChosenGlobals,
    queue: &EventQueue<ToplevelState>,
) -> (HyprlandToplevelMappingManagerV1, Option<WlSeat>) {
    let handle = queue.handle();
    let _list: ExtForeignToplevelListV1 =
        registry.bind(chosen.ext_list.name, chosen.ext_list.version, &handle, ());
    let _manager: ZwlrForeignToplevelManagerV1 = registry.bind(
        chosen.wlr_manager.name,
        chosen.wlr_manager.version,
        &handle,
        (),
    );
    let mapping = registry.bind(chosen.mapping.name, chosen.mapping.version, &handle, ());
    let seat = chosen
        .seat
        .as_ref()
        .map(|seat| registry.bind(seat.name, seat.version, &handle, ()));
    (mapping, seat)
}

fn unsupported(interface: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("The compositor does not advertise {interface}, which window focus needs"),
    )
    .with_suggestion("Update Hyprland, or focus the window yourself")
    .with_platform_detail(format!("missing Wayland global {interface}"))
}

fn fault_error(socket: &str, fault: WaylandFault) -> AdapterError {
    match fault {
        WaylandFault::Silent => display_cause_error(&DisplayCause::CompositorSilent {
            socket: socket.to_owned(),
        }),
        WaylandFault::Gone => display_cause_error(&DisplayCause::Missing {
            address: socket.to_owned(),
        }),
    }
}

#[cfg(test)]
#[path = "toplevel_session_tests.rs"]
mod tests;
