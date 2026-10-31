//! The reader state for the bound compositor's toplevels: the registry, the
//! foreign toplevel handles of both protocols, and the Hyprland address the
//! mapping manager answered for each handle, keyed by the handle's announce
//! index.

use std::collections::HashMap;

use wayland_client::protocol::{wl_callback, wl_registry, wl_seat::WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_dispatch};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;

use super::mapping_target::MappingTarget;

use crate::system::hyprland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_mapping_manager_v1::{
    self as mapping_manager, HyprlandToplevelMappingManagerV1,
};
use crate::system::hyprland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_window_mapping_handle_v1::{
    self as mapping_handle, HyprlandToplevelWindowMappingHandleV1,
};
use crate::system::wayland::exchange::{Exchange, ExchangeHandler, HoldsExchange};
use crate::system::wayland::ext_toplevel::ExtToplevel;
use crate::system::wayland::toplevel_list::{HoldsToplevels, ToplevelHandler};
use crate::system::wayland::wlr_toplevel::WlrToplevel;
use crate::system::wayland::wlr_toplevels::{HoldsWlrToplevels, WlrToplevelHandler};

/// What one connection collected. An index absent from `addresses` or
/// `wlr_addresses` has no answer yet; `None` is a handle the compositor
/// could not map.
#[derive(Default)]
pub(crate) struct MappedToplevels {
    pub(crate) exchange: Exchange,
    pub(crate) toplevels: Vec<ExtToplevel>,
    pub(crate) addresses: HashMap<usize, Option<u64>>,
    pub(crate) wlr: Vec<WlrToplevel>,
    pub(crate) wlr_addresses: HashMap<usize, Option<u64>>,
}

impl MappedToplevels {
    /// Whether every `ext` handle up to `requested` has its mapping answer.
    pub(crate) fn answered(&self, requested: usize) -> bool {
        (0..requested).all(|index| self.addresses.contains_key(&index))
    }

    /// Whether every wlr handle up to `requested` has its mapping answer.
    pub(crate) fn wlr_answered(&self, requested: usize) -> bool {
        (0..requested).all(|index| self.wlr_addresses.contains_key(&index))
    }

    /// Each handle's identifier and mapped address, in announce order.
    pub(crate) fn records(&self) -> Vec<(String, Option<u64>)> {
        self.toplevels
            .iter()
            .enumerate()
            .map(|(index, toplevel)| {
                let address = self.addresses.get(&index).copied().flatten();
                (toplevel.identifier.clone(), address)
            })
            .collect()
    }
}

impl HoldsExchange for MappedToplevels {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

impl HoldsToplevels for MappedToplevels {
    fn toplevels_mut(&mut self) -> &mut Vec<ExtToplevel> {
        &mut self.toplevels
    }
}

impl HoldsWlrToplevels for MappedToplevels {
    fn wlr_toplevels_mut(&mut self) -> &mut Vec<WlrToplevel> {
        &mut self.wlr
    }
}

delegate_dispatch!(MappedToplevels: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(MappedToplevels: [wl_callback::WlCallback: u32] => ExchangeHandler);
delegate_dispatch!(MappedToplevels: [ExtForeignToplevelListV1: ()] => ToplevelHandler);
delegate_dispatch!(MappedToplevels: [ExtForeignToplevelHandleV1: ()] => ToplevelHandler);
delegate_dispatch!(MappedToplevels: [ZwlrForeignToplevelManagerV1: ()] => WlrToplevelHandler);
delegate_dispatch!(MappedToplevels: [ZwlrForeignToplevelHandleV1: ()] => WlrToplevelHandler);
delegate_dispatch!(MappedToplevels: [WlSeat: ()] => WlrToplevelHandler);

impl Dispatch<HyprlandToplevelMappingManagerV1, ()> for MappedToplevels {
    fn event(
        _: &mut Self,
        _: &HyprlandToplevelMappingManagerV1,
        _: mapping_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<HyprlandToplevelWindowMappingHandleV1, MappingTarget> for MappedToplevels {
    fn event(
        state: &mut Self,
        _: &HyprlandToplevelWindowMappingHandleV1,
        event: mapping_handle::Event,
        target: &MappingTarget,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let address = match event {
            mapping_handle::Event::WindowAddress {
                address_hi,
                address,
            } => Some((u64::from(address_hi) << 32) | u64::from(address)),
            mapping_handle::Event::Failed => None,
        };
        match *target {
            MappingTarget::Ext(index) => state.addresses.insert(index, address),
            MappingTarget::Wlr(index) => state.wlr_addresses.insert(index, address),
        };
    }
}
