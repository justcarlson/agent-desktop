//! A test hook that delays or withholds a travel's arrival acknowledgement,
//! so a live run can show that dispatch waits for the acknowledgement up to
//! the arrival timeout and no longer. It is honored only for a regular file
//! named [`ARRIVAL_ACK_FLAG`] in the registry directory that the renderer's
//! own user owns; a symlink or another user's file is ignored. The file
//! holds `withhold`, or a delay of at most [`MAX_DELAY_MS`] milliseconds
//! counted from when the renderer received the travel. Each held
//! acknowledgement appends one JSON line to [`ARRIVAL_ACK_TRACE`] with its
//! `CLOCK_MONOTONIC` times, so a run can order it against app events. A
//! withheld acknowledgement keeps its connection open past the arrival
//! timeout and then closes it unanswered. The same user can already stop
//! the renderer, which withholds every acknowledgement, so the flag gives
//! no new way to change what an action delivers.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use agent_desktop_core::CURSOR_ARRIVAL_TIMEOUT_MS;
use rustix::fs::{Mode, OFlags};
use serde_json::{Value, json};

use super::keep_drawn_flag::flag_honored;
use super::renderer_wire::{ok_reply, write_reply};

/// The flag file's name inside the registry directory.
pub(crate) const ARRIVAL_ACK_FLAG: &str = "arrival-ack-for-test";

/// The trace file's name inside the registry directory.
pub(crate) const ARRIVAL_ACK_TRACE: &str = "arrival-ack-trace-for-test";

/// The longest delay the flag may name.
pub(crate) const MAX_DELAY_MS: u64 = 10_000;

const MAX_FLAG_BYTES: u64 = 32;
const WITHHOLD_MS: u64 = 2 * CURSOR_ARRIVAL_TIMEOUT_MS;
const WITHHOLD_WORD: &str = "withhold";

/// How the flag holds an arrival acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrivalAckHold {
    Delay(Duration),
    Withhold,
}

/// The hold that `text`, the flag's content, names.
pub(crate) fn parse_hold(text: &str) -> Option<ArrivalAckHold> {
    let text = text.trim();
    if text == WITHHOLD_WORD {
        return Some(ArrivalAckHold::Withhold);
    }
    let delay_ms: u64 = text.parse().ok()?;
    (delay_ms <= MAX_DELAY_MS).then(|| ArrivalAckHold::Delay(Duration::from_millis(delay_ms)))
}

/// The hold that the flag in `registry` names, when this user may set it.
pub(crate) fn arrival_ack_hold(registry: &Path) -> Option<ArrivalAckHold> {
    let descriptor = rustix::fs::open(
        registry.join(ARRIVAL_ACK_FLAG),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .ok()?;
    let file = File::from(descriptor);
    let metadata = file.metadata().ok()?;
    if !flag_honored(&metadata, rustix::process::getuid().as_raw()) {
        return None;
    }
    let mut text = String::new();
    file.take(MAX_FLAG_BYTES).read_to_string(&mut text).ok()?;
    parse_hold(&text)
}

/// Acknowledges a travel the renderer received at `received` on `reply`:
/// at once, or as the flag in `registry` holds it.
pub(crate) fn acknowledge_arrival(registry: &Path, received: Instant, reply: UnixStream) {
    let Some(hold) = arrival_ack_hold(registry) else {
        write_reply(&reply, &ok_reply(json!({})));
        return;
    };
    let held = HeldAck {
        hold,
        received,
        received_ns: monotonic_ns().saturating_sub(nanos(received.elapsed())),
        trace: registry.join(ARRIVAL_ACK_TRACE),
    };
    let _ = std::thread::Builder::new()
        .name("arrival-ack-hold".into())
        .spawn(move || held.release(reply));
}

struct HeldAck {
    hold: ArrivalAckHold,
    received: Instant,
    received_ns: u64,
    trace: PathBuf,
}

impl HeldAck {
    fn release(self, reply: UnixStream) {
        match self.hold {
            ArrivalAckHold::Delay(delay) => {
                std::thread::sleep(delay.saturating_sub(self.received.elapsed()));
                self.append(json!({
                    "hold": "delay",
                    "delay_ms": nanos(delay) / 1_000_000,
                    "received_ns": self.received_ns,
                    "acked_ns": monotonic_ns(),
                }));
                write_reply(&reply, &ok_reply(json!({})));
            }
            ArrivalAckHold::Withhold => {
                self.append(json!({
                    "hold": WITHHOLD_WORD,
                    "received_ns": self.received_ns,
                    "acked_ns": null,
                }));
                std::thread::sleep(Duration::from_millis(WITHHOLD_MS));
            }
        }
    }

    fn append(&self, line: Value) {
        let opened = OpenOptions::new()
            .append(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.trace);
        if let Ok(mut file) = opened {
            let _ = file.write_all(format!("{line}\n").as_bytes());
        }
    }
}

fn monotonic_ns() -> u64 {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    let seconds = u64::try_from(now.tv_sec).unwrap_or(0);
    let fraction = u64::try_from(now.tv_nsec).unwrap_or(0);
    seconds
        .saturating_mul(1_000_000_000)
        .saturating_add(fraction)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "arrival_ack_hold_tests.rs"]
mod tests;
