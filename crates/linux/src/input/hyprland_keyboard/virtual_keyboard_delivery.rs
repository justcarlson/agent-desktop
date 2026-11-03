//! Keyboard delivery through a virtual keyboard on the bound Hyprland
//! instance. The keys run through the same chunked delivery as the portal
//! route: the focus gate before every key-down, release of every held key on
//! a stop, and `uncertain` with the confirmed count once a key went out.
//! A signal stops the delivery before its next key-down, and the held keys
//! are released before the signal's own action is taken. A SIGKILL leaves
//! no chance to release, so held keys are journaled, the CLI's key guard
//! releases them when the command dies, and the next delivery on the
//! instance releases whatever a dead one still left.

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::json;

use super::active_layout::read_active_layout;
use super::control_text::refuse_control;
use super::held_key_journal::HeldKeyJournal;
use super::key_guard::guard_spawn::start_once;
use super::keyboard_cancel::stop_on_signal;
use super::keyboard_wire::KeyboardWire;
use super::virtual_keyboard_sink::VirtualKeyboardSink;
use super::wayland_wire::WaylandWire;
use crate::input::delivery::{Delivery, Gate};
use crate::input::hyprland::pointer_cancel::with_release_on_signal;
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
    let slot = crate::input::x11::worker_slot::shared();
    with_release_on_signal(&slot, deadline, |progress| {
        let mut gate = stop_on_signal(gate, progress);
        let (wire, owner) =
            WaylandWire::open(binding, deadline).map_err(|error| with_progress(error, 0))?;
        let layouts = owner_layouts(&owner).map_err(|error| with_progress(error, 0))?;
        let layout = read_active_layout(binding, &layouts, deadline)
            .map_err(|error| with_progress(error, 0))?;
        let journal = HeldKeyJournal::in_runtime_dir(binding);
        if journal.is_some() {
            start_once(binding, layout);
        }
        let sink = VirtualKeyboardSink::new(wire, layout).journaled(journal);
        deliver(sink, owner, &chunks, &mut gate, deadline)
    })
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

/// Types `chunks` through `sink` with `owner` extended for them, in the
/// sink's layout.
pub(crate) fn deliver<W: KeyboardWire>(
    sink: VirtualKeyboardSink<W>,
    owner: String,
    chunks: &[&[KeysymStep]],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let layout = sink.layout();
    let mut session = KeycodeSession::new(sink, owner, layout, chunks);
    let delivery = run_chunks(&mut session, chunks, gate, deadline)?;
    Ok(Delivery {
        virtual_keyboard: true,
        ..delivery
    })
}

#[cfg(test)]
#[path = "virtual_keyboard_delivery_tests.rs"]
mod tests;
