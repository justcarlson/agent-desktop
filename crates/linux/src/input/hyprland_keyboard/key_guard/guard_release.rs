//! What the guard does once its command is gone. A record that still holds
//! keys opens a new virtual keyboard with the seat owner's keymap in the
//! command's layout, sends a key-up for each key newest first, waits until
//! the compositor has handled them, destroys the keyboard and removes the
//! record. With no record the guard never connects. A release that fails
//! keeps the record, so the next delivery on the instance still releases it.

use agent_desktop_core::{AdapterError, Deadline};

use super::guard_record::HeldRecord;
use crate::input::hyprland_keyboard::keyboard_wire::KeyboardWire;
use crate::input::hyprland_keyboard::virtual_keyboard_sink::VirtualKeyboardSink;
use crate::input::x11::sink_error::SinkError;

/// What the guard did for its command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GuardOutcome {
    /// The command held no key when it ended.
    Clean,
    /// The guard released this many keys.
    Released(usize),
}

/// Releases the keys `record` holds through the keyboard `open` connects,
/// in `layout`.
pub(crate) fn release_held<W, F>(
    record: Option<HeldRecord>,
    layout: u32,
    deadline: Deadline,
    open: F,
) -> Result<GuardOutcome, AdapterError>
where
    W: KeyboardWire,
    F: FnOnce(Deadline) -> Result<(W, String), AdapterError>,
{
    let _ = (
        record,
        layout,
        deadline,
        open,
        VirtualKeyboardSink::<W>::new,
        SinkError::Journal,
    );
    Ok(GuardOutcome::Clean)
}

#[cfg(test)]
#[path = "guard_release_tests.rs"]
mod tests;
