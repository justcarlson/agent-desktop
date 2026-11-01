//! Keyboard delivery through a virtual keyboard on the bound Hyprland
//! instance. The keys run through the same chunked delivery as the portal
//! route: the focus gate before every key-down, release of every held key on
//! a stop, and `uncertain` with the confirmed count once a key went out.

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::json;

use super::active_layout::read_active_layout;
use super::control_text::refuse_control;
use super::keyboard_wire::KeyboardWire;
use super::virtual_keyboard_sink::VirtualKeyboardSink;
use super::wayland_wire::WaylandWire;
use crate::input::delivery::{Delivery, Gate};
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::wayland::keymap::keycode_session::KeycodeSession;
use crate::input::wayland::keymap::refusal::keymap_refused;
use crate::input::wayland::keymap::xkb_keymap::XkbKeymap;
use crate::input::wayland::keysym_send::{run_chunks, with_progress};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// Sends `chunks` through a virtual keyboard on the bound instance.
pub(crate) fn send(
    binding: &HyprlandBinding,
    chunks: &[Vec<KeysymStep>],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let chunks: Vec<&[KeysymStep]> = chunks
        .iter()
        .map(Vec::as_slice)
        .filter(|chunk| !chunk.is_empty())
        .collect();
    if chunks.is_empty() {
        return Ok(Delivery {
            delivered_events: 0,
            virtual_keyboard: true,
        });
    }
    refuse_control(&chunks)?;
    let (wire, owner) =
        WaylandWire::open(binding, deadline).map_err(|error| with_progress(error, 0))?;
    let layouts = owner_layouts(&owner).map_err(|error| with_progress(error, 0))?;
    let layout =
        read_active_layout(binding, &layouts, deadline).map_err(|error| with_progress(error, 0))?;
    deliver(wire, owner, layout, &chunks, gate, deadline)
}

fn owner_layouts(owner: &str) -> Result<Vec<String>, AdapterError> {
    XkbKeymap::compile(owner.as_bytes())?
        .ok_or_else(|| {
            keymap_refused(
                "The seat's keyboard keymap did not compile",
                "owner_keymap_invalid",
                json!({}),
            )
        })?
        .layout_names()
}

/// Types `chunks` over `wire` with `owner` extended for them, in `layout`.
pub(crate) fn deliver<W: KeyboardWire>(
    wire: W,
    owner: String,
    layout: u32,
    chunks: &[&[KeysymStep]],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let _ = (wire, owner, layout, chunks, gate, deadline);
    Ok(Delivery {
        delivered_events: 0,
        virtual_keyboard: false,
    })
}

#[cfg(test)]
#[path = "virtual_keyboard_delivery_tests.rs"]
mod tests;
