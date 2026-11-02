//! Signal cancellation for a Hyprland keyboard delivery. While the delivery
//! holds the input slot, SIGTERM, SIGINT and SIGHUP stop it before its next
//! key-down; the delivery releases its held keys and destroys its virtual
//! keyboard before the signal's own action is taken.

use agent_desktop_core::AdapterError;

use crate::input::delivery::Gate;
use crate::input::x11::progress::Progress;

/// `gate`, refusing every key-down once `progress` is cancelled.
pub(crate) fn stop_on_signal<'a>(
    gate: Gate<'a>,
    _progress: &'a Progress,
) -> impl FnMut() -> Result<(), AdapterError> + 'a {
    move || gate()
}

#[cfg(test)]
#[path = "keyboard_cancel_tests.rs"]
mod tests;
