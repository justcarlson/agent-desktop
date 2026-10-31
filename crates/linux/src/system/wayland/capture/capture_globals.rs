//! The globals a capture binds, chosen from the registry. A global below
//! the version a capture needs counts as missing, and the first missing one
//! is named. Each global is bound at the lower of its advertised version
//! and the version this client speaks.

use super::output_capture_globals::OutputCaptureGlobals;
use crate::system::wayland::exchange::Exchange;
use crate::system::wayland::global::Global;

pub(crate) const SHM: (&str, u32) = ("wl_shm", 1);
pub(crate) const SCREENCOPY: (&str, u32) = ("zwlr_screencopy_manager_v1", 3);
pub(crate) const OUTPUT: (&str, u32) = ("wl_output", 4);

pub(crate) fn output(exchange: &Exchange) -> Result<OutputCaptureGlobals, &'static str> {
    let shm = at_least(exchange, SHM)?;
    let screencopy = at_least(exchange, SCREENCOPY)?;
    let outputs: Vec<Global> = exchange
        .all(OUTPUT.0)
        .filter(|global| global.version >= OUTPUT.1)
        .cloned()
        .collect();
    if outputs.is_empty() {
        return Err(OUTPUT.0);
    }
    Ok(OutputCaptureGlobals {
        shm,
        screencopy,
        outputs,
    })
}

pub(crate) fn at_least(
    exchange: &Exchange,
    (interface, needs): (&'static str, u32),
) -> Result<Global, &'static str> {
    exchange.find(interface, needs).cloned().ok_or(interface)
}

#[cfg(test)]
#[path = "capture_globals_tests.rs"]
mod tests;
