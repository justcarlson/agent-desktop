//! The compositor's toplevels over `zwlr_foreign_toplevel_manager_v1`, the
//! wlroots-family protocol that lets a client activate or close another
//! client's toplevel. Whether it can be used is decided from the registry
//! alone, with no compositor-specific check. A reader's state holds the
//! handles, exposes them through `HoldsWlrToplevels` and delegates the
//! manager, handle and seat events here with
//! `wayland_client::delegate_dispatch!`.

use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    self as wlr_handle, ZwlrForeignToplevelHandleV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{
    self as wlr_manager, ZwlrForeignToplevelManagerV1,
};

use super::exchange::Exchange;
use super::global::Global;
use super::wlr_toplevel::WlrToplevel;

/// The interface of the toplevel manager global.
pub(crate) const WLR_MANAGER_INTERFACE: &str = "zwlr_foreign_toplevel_manager_v1";
/// The manager version that reports each toplevel's parent.
pub(crate) const WLR_MANAGER_VERSION: u32 = 3;
/// The interface of the seat an activation names.
pub(crate) const SEAT_INTERFACE: &str = "wl_seat";

const ACTIVATED: u32 = 2;

/// The manager global, when the compositor advertises one.
pub(crate) fn advertised_manager(exchange: &Exchange) -> Option<&Global> {
    exchange.find(WLR_MANAGER_INTERFACE, 1)
}

/// Whether a `state` event's array holds the activated state. The array is
/// a list of native-endian 32-bit values.
pub(crate) fn holds_activated(state: &[u8]) -> bool {
    state
        .chunks_exact(4)
        .filter_map(|word| word.try_into().ok())
        .any(|word| u32::from_ne_bytes(word) == ACTIVATED)
}

/// A reader state that collects wlr foreign toplevel handles.
pub(crate) trait HoldsWlrToplevels {
    fn wlr_toplevels_mut(&mut self) -> &mut Vec<WlrToplevel>;
}

/// Handles the manager, handle and seat events for any reader state.
pub(crate) struct WlrToplevelHandler;

impl<S> Dispatch<ZwlrForeignToplevelManagerV1, (), S> for WlrToplevelHandler
where
    S: Dispatch<ZwlrForeignToplevelManagerV1, ()>
        + Dispatch<ZwlrForeignToplevelHandleV1, ()>
        + HoldsWlrToplevels
        + 'static,
{
    fn event(
        state: &mut S,
        _: &ZwlrForeignToplevelManagerV1,
        event: wlr_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<S>,
    ) {
        if let wlr_manager::Event::Toplevel { toplevel } = event {
            state.wlr_toplevels_mut().push(WlrToplevel {
                handle: toplevel,
                activated: false,
                closed: false,
            });
        }
    }

    event_created_child!(S, ZwlrForeignToplevelManagerV1, [
        wlr_manager::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ())
    ]);
}

impl<S> Dispatch<ZwlrForeignToplevelHandleV1, (), S> for WlrToplevelHandler
where
    S: Dispatch<ZwlrForeignToplevelHandleV1, ()> + HoldsWlrToplevels,
{
    fn event(
        state: &mut S,
        handle: &ZwlrForeignToplevelHandleV1,
        event: wlr_handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<S>,
    ) {
        let Some(record) = state
            .wlr_toplevels_mut()
            .iter_mut()
            .find(|known| known.handle == *handle)
        else {
            return;
        };
        match event {
            wlr_handle::Event::State { state } => record.activated = holds_activated(&state),
            wlr_handle::Event::Closed => record.closed = true,
            _ => {}
        }
    }
}

impl<S> Dispatch<WlSeat, (), S> for WlrToplevelHandler
where
    S: Dispatch<WlSeat, ()>,
{
    fn event(_: &mut S, _: &WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<S>) {
    }
}

#[cfg(test)]
#[path = "wlr_toplevels_tests.rs"]
mod tests;
