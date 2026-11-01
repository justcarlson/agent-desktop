//! The reader state for one virtual keyboard connection: the registry, the
//! seat's capabilities and the keymap the seat's keyboard announced.

use std::os::fd::OwnedFd;

use wayland_client::protocol::wl_keyboard::{self, KeymapFormat, WlKeyboard};
use wayland_client::protocol::wl_seat::{self, Capability, WlSeat};
use wayland_client::protocol::{wl_callback, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, delegate_dispatch};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::{
    self as manager, ZwpVirtualKeyboardManagerV1,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::{
    self as virtual_keyboard, ZwpVirtualKeyboardV1,
};

use crate::system::wayland::exchange::{Exchange, ExchangeHandler, HoldsExchange};

/// What the connection collected. `keymap` is the last XKB keymap the seat's
/// keyboard announced, with its size; `other_format` records a keymap in a
/// format other than XKB v1.
#[derive(Default)]
pub(crate) struct KeyboardState {
    pub(crate) exchange: Exchange,
    pub(crate) has_keyboard: bool,
    pub(crate) keymap: Option<(OwnedFd, u32)>,
    pub(crate) other_format: bool,
}

impl HoldsExchange for KeyboardState {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

delegate_dispatch!(KeyboardState: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(KeyboardState: [wl_callback::WlCallback: u32] => ExchangeHandler);

impl Dispatch<WlSeat, ()> for KeyboardState {
    fn event(
        state: &mut Self,
        _: &WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            state.has_keyboard = capabilities.contains(Capability::Keyboard);
        }
    }
}

impl Dispatch<WlKeyboard, ()> for KeyboardState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Keymap { format, fd, size } = event {
            if format == WEnum::Value(KeymapFormat::XkbV1) {
                state.keymap = Some((fd, size));
                state.other_format = false;
            } else {
                state.keymap = None;
                state.other_format = true;
            }
        }
    }
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for KeyboardState {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardManagerV1,
        _: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for KeyboardState {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardV1,
        _: virtual_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
