//! The event handlers of a toplevel connection. Each handler only records
//! what the compositor said; the session decides what it means.

use wayland_client::protocol::{wl_callback, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::{
    self as ext_handle, ExtForeignToplevelHandleV1,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::{
    self as ext_list, ExtForeignToplevelListV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    self as wlr_handle, ZwlrForeignToplevelHandleV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{
    self as wlr_manager, ZwlrForeignToplevelManagerV1,
};

use super::advertised_global::AdvertisedGlobal;
use super::mapping_target::MappingTarget;
use super::toplevel_state::ToplevelState;
use crate::system::wayland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_mapping_manager_v1::HyprlandToplevelMappingManagerV1;
use crate::system::wayland::protocol::hyprland_toplevel_mapping::hyprland_toplevel_window_mapping_handle_v1::{
    self as mapping_handle, HyprlandToplevelWindowMappingHandleV1,
};

const ACTIVATED_STATE: u32 = 2;

impl Dispatch<wl_registry::WlRegistry, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.globals.push(AdvertisedGlobal {
                name,
                interface,
                version,
            });
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.synced += 1;
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for ToplevelState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<HyprlandToplevelMappingManagerV1, ()> for ToplevelState {
    fn event(
        _: &mut Self,
        _: &HyprlandToplevelMappingManagerV1,
        _: <HyprlandToplevelMappingManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        _: &ExtForeignToplevelListV1,
        event: ext_list::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_list::Event::Toplevel { toplevel } = event {
            state.ext.push((toplevel, Default::default()));
        }
    }

    event_created_child!(ToplevelState, ExtForeignToplevelListV1, [
        ext_list::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        handle: &ExtForeignToplevelHandleV1,
        event: ext_handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some((_, record)) = state.ext.iter_mut().find(|(known, _)| known == handle) else {
            return;
        };
        match event {
            ext_handle::Event::Identifier { identifier } => record.identifier = identifier,
            ext_handle::Event::Closed => record.identifier.clear(),
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: wlr_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wlr_manager::Event::Toplevel { toplevel } = event {
            state.wlr.push((toplevel, Default::default()));
        }
    }

    event_created_child!(ToplevelState, ZwlrForeignToplevelManagerV1, [
        wlr_manager::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: wlr_handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some((_, record)) = state.wlr.iter_mut().find(|(known, _)| known == handle) else {
            return;
        };
        match event {
            wlr_handle::Event::State { state } => record.activated = holds_activated(&state),
            wlr_handle::Event::Closed => record.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<HyprlandToplevelWindowMappingHandleV1, MappingTarget> for ToplevelState {
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
        let slot = match *target {
            MappingTarget::Ext(index) => state
                .ext
                .get_mut(index)
                .map(|(_, record)| (&mut record.address, &mut record.answered)),
            MappingTarget::Wlr(index) => state
                .wlr
                .get_mut(index)
                .map(|(_, record)| (&mut record.address, &mut record.answered)),
        };
        if let Some((slot_address, answered)) = slot {
            *slot_address = address;
            *answered = true;
        }
    }
}

/// Whether a wlr state array, native-endian `u32` values, holds `activated`.
pub(crate) fn holds_activated(state: &[u8]) -> bool {
    state
        .chunks_exact(4)
        .filter_map(|chunk| chunk.try_into().ok())
        .any(|bytes: [u8; 4]| u32::from_ne_bytes(bytes) == ACTIVATED_STATE)
}

#[cfg(test)]
#[path = "toplevel_dispatch_tests.rs"]
mod tests;
