//! The compositor's toplevels over `ext_foreign_toplevel_list_v1`, the
//! staging protocol wlroots-family compositors share. Whether it can be read
//! is decided from the registry alone, with no compositor-specific check. A
//! reader's state holds the handles, exposes them through `HoldsToplevels`
//! and delegates the list and handle events here with
//! `wayland_client::delegate_dispatch!`.

use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::{
    self as ext_handle, ExtForeignToplevelHandleV1,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::{
    self as ext_list, ExtForeignToplevelListV1,
};

use super::exchange::Exchange;
use super::ext_toplevel::ExtToplevel;
use super::global::Global;

/// The interface of the toplevel list global.
pub(crate) const LIST_INTERFACE: &str = "ext_foreign_toplevel_list_v1";

/// The list global, when the compositor advertises one.
pub(crate) fn advertised_list(exchange: &Exchange) -> Option<&Global> {
    exchange.find(LIST_INTERFACE, 1)
}

/// A reader state that collects foreign toplevel handles.
pub(crate) trait HoldsToplevels {
    fn toplevels_mut(&mut self) -> &mut Vec<ExtToplevel>;
}

/// Handles the list and handle events for any reader state.
pub(crate) struct ToplevelHandler;

impl<S> Dispatch<ExtForeignToplevelListV1, (), S> for ToplevelHandler
where
    S: Dispatch<ExtForeignToplevelListV1, ()>
        + Dispatch<ExtForeignToplevelHandleV1, ()>
        + HoldsToplevels
        + 'static,
{
    fn event(
        state: &mut S,
        _: &ExtForeignToplevelListV1,
        event: ext_list::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<S>,
    ) {
        if let ext_list::Event::Toplevel { toplevel } = event {
            state.toplevels_mut().push(ExtToplevel {
                handle: toplevel,
                identifier: String::new(),
            });
        }
    }

    event_created_child!(S, ExtForeignToplevelListV1, [
        ext_list::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}

impl<S> Dispatch<ExtForeignToplevelHandleV1, (), S> for ToplevelHandler
where
    S: Dispatch<ExtForeignToplevelHandleV1, ()> + HoldsToplevels,
{
    fn event(
        state: &mut S,
        handle: &ExtForeignToplevelHandleV1,
        event: ext_handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<S>,
    ) {
        let Some(record) = state
            .toplevels_mut()
            .iter_mut()
            .find(|known| known.handle == *handle)
        else {
            return;
        };
        match event {
            ext_handle::Event::Identifier { identifier } => record.identifier = identifier,
            ext_handle::Event::Closed => record.identifier.clear(),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "toplevel_list_tests.rs"]
mod tests;
