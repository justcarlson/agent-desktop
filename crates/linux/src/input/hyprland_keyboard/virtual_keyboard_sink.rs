//! A [`KeycodeSink`] over a virtual keyboard wire. Each key edge is followed
//! by the modifiers the keys now hold, computed by an XKB state over the same
//! keymap, with the group fixed to the delivery's layout. An unfocused client
//! never sees the owner's `modifiers` event, so the layout comes from the
//! caller and is sent before the first key. Each key-down is journaled
//! before it is sent, and the keys a dead delivery left held are released
//! right after the keymap, before any key of this delivery.

use std::time::Instant;

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::json;

use super::held_key_journal::HeldKeyJournal;
use super::held_keys::HeldKeys;
use super::keyboard_wire::KeyboardWire;
use super::stale_key_release::release_stale_keys;
use crate::input::wayland::keymap::keycode_sink::KeycodeSink;
use crate::input::wayland::keymap::modifier_state::ModifierState;
use crate::input::wayland::keymap::refusal::keymap_refused;
use crate::input::wayland::keymap::xkb_keymap::XkbKeymap;
use crate::input::wayland::keymap::xkb_state::XkbState;

/// One virtual keyboard: the wire, the layout it types in, the XKB state of
/// the keys it has sent and the keys it holds.
pub(crate) struct VirtualKeyboardSink<W: KeyboardWire> {
    wire: W,
    layout: u32,
    state: Option<XkbState>,
    reported: ModifierState,
    epoch: Instant,
    closed: bool,
    held: HeldKeys,
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
            held: HeldKeys::default(),
        }
    }

    /// The sink, keeping its held keys in `journal`.
    pub(crate) fn journaled(self, journal: Option<HeldKeyJournal>) -> Self {
        Self {
            held: HeldKeys::new(journal),
            ..self
        }
    }

    pub(crate) fn layout(&self) -> u32 {
        self.layout
    }

    /// Sends a key-up for each of `codes`, keys another virtual keyboard
    /// left held, and waits until the compositor has handled them.
    pub(crate) fn release_keys(&mut self, codes: &[u32]) -> Result<(), AdapterError> {
        let _ = (codes, release_stale_keys::<W>);
        Ok(())
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
        let compiled = XkbKeymap::compile(keymap.as_bytes())?.ok_or_else(|| {
            keymap_refused(
                "The extended keyboard keymap did not compile",
                "extended_keymap_invalid",
                json!({}),
            )
        })?;
        let state = XkbState::new(&compiled, self.layout)?;
        let modifiers = state.modifiers()?;
        self.wire.keymap(keymap)?;
        self.wire.modifiers(modifiers)?;
        self.wire.sync()?;
        self.reported = modifiers;
        self.state = Some(state);
        let time = self.time_ms();
        self.held.release_stale(&mut self.wire, time)
    }

    fn key(&mut self, code: u32, down: bool, _deadline: Deadline) -> Result<(), AdapterError> {
        let time = self.time_ms();
        let state = self.state.as_mut().ok_or_else(|| {
            AdapterError::internal("the virtual keyboard sent a key before its keymap")
        })?;
        if down {
            self.held.press(code)?;
        }
        let modifiers = state.update_key(code, down)?;
        tracing::debug!(code, down, time, "virtual keyboard key edge");
        self.wire.key(time, code, down)?;
        if modifiers != self.reported {
            self.wire.modifiers(modifiers)?;
            self.reported = modifiers;
        }
        self.wire.flush()?;
        if !down {
            self.held.release(code);
        }
        Ok(())
    }

    fn close(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        tracing::debug!("virtual keyboard destroyed");
        self.wire.destroy()
    }
}

#[cfg(test)]
#[path = "virtual_keyboard_sink_tests.rs"]
mod tests;
