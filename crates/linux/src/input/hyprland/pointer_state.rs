//! What a virtual pointer link is told: the registry, its round trips and
//! the name each bound output announced, keyed by its registry name.

use wayland_client::protocol::{wl_callback, wl_output, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_dispatch, delegate_noop};
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

use crate::system::wayland::exchange::{Exchange, ExchangeHandler, HoldsExchange};

/// The registry, round trips and output names of one pointer link.
#[derive(Debug, Default)]
pub(crate) struct PointerState {
    pub(crate) exchange: Exchange,
    pub(crate) outputs: Vec<(u32, String)>,
}

impl HoldsExchange for PointerState {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

impl Dispatch<wl_output::WlOutput, u32> for PointerState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        registry_name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            state.outputs.push((*registry_name, name));
        }
    }
}

delegate_dispatch!(PointerState: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(PointerState: [wl_callback::WlCallback: u32] => ExchangeHandler);
delegate_noop!(PointerState: ignore wl_seat::WlSeat);
delegate_noop!(PointerState: ignore ZwlrVirtualPointerManagerV1);
delegate_noop!(PointerState: ignore ZwlrVirtualPointerV1);
