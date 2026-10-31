//! One held connection to the bound compositor's Wayland socket over which
//! its toplevels are read through `zwlr_foreign_toplevel_manager_v1`, each
//! mapped to its Hyprland address, and one is activated.
//! The manager is the shared wlroots-family protocol, found by its registry
//! entry alone; the mapping manager is what ties a handle to the Hyprland
//! window, and so to its process. Every read waits for at most the
//! session's deadline.

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use wayland_client::Proxy;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;

use super::bound_socket::{connect_bound, unadvertised};
use super::mapped_toplevels::MappedToplevels;
use super::mapping_target::MappingTarget;
use super::wlr_window::WlrWindow;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::protocol::MAPPING_INTERFACE;
use crate::system::hyprland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_mapping_manager_v1::HyprlandToplevelMappingManagerV1;
use crate::system::wayland::link::Link;
use crate::system::wayland::wayland_fault::WaylandFault;
use crate::system::wayland::wlr_toplevel::WlrToplevel;
use crate::system::wayland::wlr_toplevels::{
    SEAT_INTERFACE, WLR_MANAGER_INTERFACE, WLR_MANAGER_VERSION, advertised_manager,
};

const NEED: &str = "activating a window";

/// A held toplevel connection to the bound compositor.
pub(crate) struct ToplevelSession {
    link: Link<MappedToplevels>,
    state: MappedToplevels,
    mapping: HyprlandToplevelMappingManagerV1,
    seat: Option<WlSeat>,
    requested: usize,
    binding: HyprlandBinding,
}

impl ToplevelSession {
    /// Connects to the binding's Wayland socket, binds the toplevel manager
    /// and waits until every announced handle is mapped to an address.
    pub(crate) fn open(
        binding: &HyprlandBinding,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let stream = connect_bound(binding)?;
        let mut state = MappedToplevels::default();
        let socket = binding.wayland_socket.as_path();
        let link = Link::over(stream, &mut state, deadline).map_err(|f| f.into_error(socket))?;
        let manager = advertised_manager(&state.exchange)
            .cloned()
            .ok_or_else(|| unadvertised(WLR_MANAGER_INTERFACE, NEED))?;
        let mapping = state
            .exchange
            .find(MAPPING_INTERFACE, 1)
            .cloned()
            .ok_or_else(|| unadvertised(MAPPING_INTERFACE, NEED))?;
        let seat = state.exchange.find(SEAT_INTERFACE, 1).cloned();
        let _manager: ZwlrForeignToplevelManagerV1 = link.bind(&manager, WLR_MANAGER_VERSION, ());
        let mapping = link.bind(&mapping, 1, ());
        let seat = seat.map(|seat| link.bind(&seat, 1, ()));
        let mut session = Self {
            link,
            state,
            mapping,
            seat,
            requested: 0,
            binding: binding.clone(),
        };
        session.settle()?;
        Ok(session)
    }

    /// Reads every event sent so far, then maps each handle announced since
    /// the last settle and waits for the answers.
    pub(crate) fn settle(&mut self) -> Result<(), AdapterError> {
        self.link
            .roundtrip(&mut self.state)
            .map_err(|fault| self.fault(fault))?;
        let handle = self.link.handle();
        for (index, toplevel) in self.state.wlr.iter().enumerate().skip(self.requested) {
            let target = MappingTarget::Wlr(index);
            let _mapping =
                self.mapping
                    .get_window_for_toplevel_wlr(&toplevel.handle, &handle, target);
        }
        let requested = self.state.wlr.len();
        self.requested = requested;
        self.link
            .roundtrip(&mut self.state)
            .map_err(|fault| self.fault(fault))?;
        self.link
            .wait(&mut self.state, |state| state.wlr_answered(requested))
            .map_err(|fault| self.fault(fault))
    }

    /// Every toplevel the compositor has not closed that maps to an
    /// address, in announce order, as of the last settle.
    pub(crate) fn windows(&self) -> Vec<WlrWindow> {
        self.state
            .wlr
            .iter()
            .enumerate()
            .filter(|(_, toplevel)| !toplevel.closed)
            .filter_map(|(index, toplevel)| {
                let address = self.state.wlr_addresses.get(&index).copied().flatten()?;
                Some(WlrWindow {
                    id: toplevel.handle.id().protocol_id(),
                    address,
                    activated: toplevel.activated,
                })
            })
            .collect()
    }

    /// Sends `activate` on the one live handle mapped to `address`.
    pub(crate) fn activate(&mut self, address: u64) -> Result<(), AdapterError> {
        let seat = self
            .seat
            .clone()
            .ok_or_else(|| unadvertised(SEAT_INTERFACE, NEED))?;
        let toplevel = self.live(|index, _| self.address_of(index) == Some(address));
        let Some(toplevel) = toplevel else {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "The window has no foreign toplevel handle that can activate it",
            )
            .with_suggestion("Run 'list-windows' to confirm the window, then retry"));
        };
        toplevel.handle.activate(&seat);
        self.link.flush().map_err(|fault| self.fault(fault))
    }

    fn address_of(&self, index: usize) -> Option<u64> {
        self.state.wlr_addresses.get(&index).copied().flatten()
    }

    fn live(&self, wanted: impl Fn(usize, &WlrToplevel) -> bool) -> Option<WlrToplevel> {
        self.state
            .wlr
            .iter()
            .enumerate()
            .find(|(index, toplevel)| !toplevel.closed && wanted(*index, toplevel))
            .map(|(_, toplevel)| toplevel.clone())
    }

    fn fault(&self, fault: WaylandFault) -> AdapterError {
        fault.into_error(self.binding.wayland_socket.as_path())
    }
}

#[cfg(test)]
#[path = "toplevel_session_tests.rs"]
mod tests;
