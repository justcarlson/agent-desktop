//! Signal cancellation for a Hyprland keyboard delivery. While the delivery
//! holds the input slot, SIGTERM, SIGINT and SIGHUP stop it before its next
//! key-down; the delivery releases its held keys and destroys its virtual
//! keyboard before the signal's own action is taken.

use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use crate::input::delivery::Gate;
use crate::input::x11::progress::Progress;

/// `gate`, refusing every key-down once `progress` is cancelled.
pub(crate) fn stop_on_signal<'a>(
    gate: Gate<'a>,
    progress: &'a Progress,
) -> impl FnMut() -> Result<(), AdapterError> + 'a {
    move || {
        if progress.is_cancelled() {
            return Err(stopped_by_signal());
        }
        gate()?;
        if progress.is_cancelled() {
            return Err(stopped_by_signal());
        }
        Ok(())
    }
}

fn stopped_by_signal() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "A signal stopped the keyboard delivery",
    )
    .with_details(json!({ "cause": "signal" }))
}

#[cfg(test)]
#[path = "keyboard_cancel_tests.rs"]
mod tests;
