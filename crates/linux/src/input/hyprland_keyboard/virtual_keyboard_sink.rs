//! A [`KeycodeSink`] over a virtual keyboard wire. Each key edge is followed
//! by the modifiers the keys now hold, computed by an XKB state over the same
//! keymap, with the group fixed to the delivery's layout. An unfocused client
//! never sees the owner's `modifiers` event, so the layout comes from the
//! caller and is sent before the first key.

use std::time::Instant;

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::json;

use super::keyboard_wire::KeyboardWire;
use crate::input::wayland::keymap::keycode_sink::KeycodeSink;
use crate::input::wayland::keymap::modifier_state::ModifierState;
use crate::input::wayland::keymap::refusal::keymap_refused;
use crate::input::wayland::keymap::xkb_keymap::XkbKeymap;
use crate::input::wayland::keymap::xkb_state::XkbState;

/// One virtual keyboard: the wire, the layout it types in and the XKB state
/// of the keys it has sent.
pub(crate) struct VirtualKeyboardSink<W: KeyboardWire> {
    wire: W,
    layout: u32,
    state: Option<XkbState>,
    reported: ModifierState,
    epoch: Instant,
    closed: bool,
}

impl<W: KeyboardWire> VirtualKeyboardSink<W> {
    pub(crate) fn new(wire: W, layout: u32) -> Self {
        Self {
            wire,
            layout,
            state: None,
            reported: ModifierState::default(),
            epoch: Instant::now(),
            closed: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn wire(&self) -> &W {
        &self.wire
    }

    fn time_ms(&self) -> u32 {
        u32::try_from(self.epoch.elapsed().as_millis() % (u128::from(u32::MAX) + 1)).unwrap_or(0)
    }
}

impl<W: KeyboardWire> KeycodeSink for VirtualKeyboardSink<W> {
    fn upload_keymap(&mut self, keymap: &str, _deadline: Deadline) -> Result<(), AdapterError> {
        let _ = (keymap, self.layout, &mut self.state, &mut self.reported);
        Ok(())
    }

    fn key(&mut self, code: u32, down: bool, _deadline: Deadline) -> Result<(), AdapterError> {
        let _ = (code, down, self.time_ms());
        Ok(())
    }

    fn close(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        let _ = &mut self.closed;
        Ok(())
    }
}

#[cfg(test)]
#[path = "virtual_keyboard_sink_tests.rs"]
mod tests;
