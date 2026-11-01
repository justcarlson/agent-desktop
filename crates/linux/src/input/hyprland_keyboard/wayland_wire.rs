//! The [`KeyboardWire`] over the bound compositor's own Wayland socket. Open
//! reads the seat owner's keymap through a short-lived `wl_keyboard`; the
//! virtual keyboard itself is created at the first keymap, because the
//! compositor makes a live virtual keyboard the seat's keyboard. Dropping the
//! wire destroys it, and the compositor returns to the physical keyboard's
//! keymap.

use std::os::fd::AsFd;

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::json;
use wayland_client::Proxy;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;

use super::keyboard_state::KeyboardState;
use super::keyboard_wire::KeyboardWire;
use super::keymap_fd::{keymap_memfd, read_keymap};
use crate::input::wayland::keymap::modifier_state::ModifierState;
use crate::input::wayland::keymap::refusal::keymap_refused;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::toplevel::bound_socket::{connect_bound, unadvertised};
use crate::system::wayland::link::Link;
use crate::system::wayland::wayland_fault::WaylandFault;
use crate::system::wayland::wlr_toplevels::SEAT_INTERFACE;

const MANAGER_INTERFACE: &str = "zwp_virtual_keyboard_manager_v1";
const NEED: &str = "keyboard input on Hyprland";
const SEAT_VERSION: u32 = 7;
const KEYBOARD_RELEASE_SINCE: u32 = 3;
const XKB_V1: u32 = 1;

/// One connection to the bound compositor that sends a virtual keyboard's
/// requests.
pub(crate) struct WaylandWire {
    link: Link<KeyboardState>,
    state: KeyboardState,
    manager: ZwpVirtualKeyboardManagerV1,
    seat: WlSeat,
    keyboard: Option<ZwpVirtualKeyboardV1>,
    binding: HyprlandBinding,
}

impl WaylandWire {
    /// Connects to the binding's Wayland socket, binds the seat and the
    /// virtual keyboard manager and reads the seat owner's keymap text.
    pub(crate) fn open(
        binding: &HyprlandBinding,
        deadline: Deadline,
    ) -> Result<(Self, String), AdapterError> {
        let stream = connect_bound(binding)?;
        let socket = binding.wayland_socket.as_path();
        let mut state = KeyboardState::default();
        let mut link =
            Link::over(stream, &mut state, deadline).map_err(|f| f.into_error(socket))?;
        let manager = state
            .exchange
            .find(MANAGER_INTERFACE, 1)
            .cloned()
            .ok_or_else(|| unadvertised(MANAGER_INTERFACE, NEED))?;
        let seat = state
            .exchange
            .find(SEAT_INTERFACE, 1)
            .cloned()
            .ok_or_else(|| unadvertised(SEAT_INTERFACE, NEED))?;
        let manager: ZwpVirtualKeyboardManagerV1 = link.bind(&manager, 1, ());
        let seat: WlSeat = link.bind(&seat, SEAT_VERSION, ());
        link.roundtrip(&mut state)
            .map_err(|f| f.into_error(socket))?;
        if !state.has_keyboard {
            return Err(no_owner_keymap("the seat has no keyboard"));
        }
        let owner_keyboard = seat.get_keyboard(&link.handle(), ());
        link.roundtrip(&mut state)
            .map_err(|f| f.into_error(socket))?;
        if owner_keyboard.version() >= KEYBOARD_RELEASE_SINCE {
            owner_keyboard.release();
        }
        let Some((fd, size)) = state.keymap.take() else {
            let detail = if state.other_format {
                "the seat's keymap is not XKB"
            } else {
                "the seat's keyboard announced no keymap"
            };
            return Err(no_owner_keymap(detail));
        };
        let owner = read_keymap(fd, size)?;
        let wire = Self {
            link,
            state,
            manager,
            seat,
            keyboard: None,
            binding: binding.clone(),
        };
        Ok((wire, owner))
    }

    fn fault(&self, fault: WaylandFault) -> AdapterError {
        fault.into_error(self.binding.wayland_socket.as_path())
    }

    fn keyboard(&self) -> Result<&ZwpVirtualKeyboardV1, AdapterError> {
        self.keyboard.as_ref().ok_or_else(|| {
            AdapterError::internal("the virtual keyboard was used before its keymap")
        })
    }
}

fn no_owner_keymap(detail: &str) -> AdapterError {
    keymap_refused(
        "The seat's keyboard keymap could not be read",
        "owner_keymap_unreadable",
        json!({ "detail": detail }),
    )
}

impl KeyboardWire for WaylandWire {
    fn keymap(&mut self, text: &str) -> Result<(), AdapterError> {
        let (fd, size) = keymap_memfd(text)?;
        if self.keyboard.is_none() {
            let created = self
                .manager
                .create_virtual_keyboard(&self.seat, &self.link.handle(), ());
            self.keyboard = Some(created);
        }
        self.keyboard()?.keymap(XKB_V1, fd.as_fd(), size);
        self.link.flush().map_err(|fault| self.fault(fault))
    }

    fn key(&mut self, time_ms: u32, code: u32, down: bool) -> Result<(), AdapterError> {
        self.keyboard()?.key(time_ms, code, u32::from(down));
        Ok(())
    }

    fn modifiers(&mut self, state: ModifierState) -> Result<(), AdapterError> {
        self.keyboard()?
            .modifiers(state.depressed, state.latched, state.locked, state.group);
        Ok(())
    }

    fn sync(&mut self) -> Result<(), AdapterError> {
        self.link
            .roundtrip(&mut self.state)
            .map_err(|fault| self.fault(fault))
    }

    fn flush(&mut self) -> Result<(), AdapterError> {
        self.link.flush().map_err(|fault| self.fault(fault))
    }

    fn destroy(&mut self) -> Result<(), AdapterError> {
        let Some(keyboard) = self.keyboard.take() else {
            return Ok(());
        };
        keyboard.destroy();
        self.link.flush().map_err(|fault| self.fault(fault))
    }
}

impl Drop for WaylandWire {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}
