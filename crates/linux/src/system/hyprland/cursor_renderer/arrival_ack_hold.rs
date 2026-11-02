//! A test hook that delays or withholds a travel's arrival acknowledgement,
//! so a live run can show that dispatch waits for it up to the arrival
//! timeout and no longer.

use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;

use super::renderer_wire::{ok_reply, write_reply};

/// The flag file's name inside the registry directory.
pub(crate) const ARRIVAL_ACK_FLAG: &str = "arrival-ack-for-test";

/// The trace file's name inside the registry directory.
pub(crate) const ARRIVAL_ACK_TRACE: &str = "arrival-ack-trace-for-test";

/// How the flag holds an arrival acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrivalAckHold {
    Delay(Duration),
    Withhold,
}

/// The hold that `text`, the flag's content, names.
pub(crate) fn parse_hold(_text: &str) -> Option<ArrivalAckHold> {
    None
}

/// The hold that the flag in `registry` names, when this user may set it.
pub(crate) fn arrival_ack_hold(_registry: &Path) -> Option<ArrivalAckHold> {
    None
}

/// Acknowledges a travel received at `received` on `reply`.
pub(crate) fn acknowledge_arrival(_registry: &Path, _received: Instant, reply: UnixStream) {
    write_reply(&reply, &ok_reply(json!({})));
}

#[cfg(test)]
#[path = "arrival_ack_hold_tests.rs"]
mod tests;
