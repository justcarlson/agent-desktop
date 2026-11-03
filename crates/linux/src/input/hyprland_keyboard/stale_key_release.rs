//! How a delivery releases keys a dead delivery left held: a key-up for each,
//! newest first, through the delivery's own virtual keyboard, confirmed by a
//! round trip before the records are removed.

use agent_desktop_core::AdapterError;

use super::keyboard_wire::KeyboardWire;

/// Sends a key-up for each of `codes` over `wire` and waits until the
/// compositor has handled them.
pub(crate) fn release_stale_keys<W: KeyboardWire>(
    wire: &mut W,
    time_ms: u32,
    codes: &[u32],
) -> Result<(), AdapterError> {
    for code in codes.iter().rev() {
        tracing::debug!(code, "releasing a key a dead delivery left held");
        wire.key(time_ms, *code, false)?;
    }
    wire.flush()?;
    wire.sync()
}
