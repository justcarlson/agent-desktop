use agent_desktop_core::AdapterError;

use crate::input::wayland::keymap::modifier_state::ModifierState;

/// The requests of one `zwp_virtual_keyboard_v1`, as a sink sends them. The
/// keyboard itself is created by the first `keymap`, so a delivery refused
/// before its keymap never adds a keyboard to the seat.
pub(crate) trait KeyboardWire {
    fn keymap(&mut self, text: &str) -> Result<(), AdapterError>;
    fn key(&mut self, time_ms: u32, code: u32, down: bool) -> Result<(), AdapterError>;
    fn modifiers(&mut self, state: ModifierState) -> Result<(), AdapterError>;
    /// Waits until the compositor has handled every request sent so far.
    fn sync(&mut self) -> Result<(), AdapterError>;
    fn flush(&mut self) -> Result<(), AdapterError>;
    /// Destroys the keyboard, if one was created, and sends the request.
    fn destroy(&mut self) -> Result<(), AdapterError>;
}
