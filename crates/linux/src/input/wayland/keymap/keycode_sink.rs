use agent_desktop_core::{AdapterError, Deadline};

/// A keyboard that takes a keymap and evdev key edges, such as a Wayland
/// virtual keyboard. The session uploads the keymap once, before any key.
pub(crate) trait KeycodeSink {
    fn upload_keymap(&mut self, keymap: &str, deadline: Deadline) -> Result<(), AdapterError>;
    fn key(&mut self, code: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError>;
}
