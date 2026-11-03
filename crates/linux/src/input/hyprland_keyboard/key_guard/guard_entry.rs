//! The guard's entry point. An `agent-desktop` process started with the
//! guard marker reads its payload from standard input, waits on its
//! command's pidfd, and once the command is gone releases what the
//! command's record still holds. A marker for another protocol version is
//! refused.

use std::io::Read;
use std::os::fd::{AsFd, BorrowedFd, FromRawFd, OwnedFd};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use rustix::event::{PollFd, PollFlags, poll};
use rustix::io::Errno;

use super::guard_payload::GuardPayload;
use super::guard_record::read_held;
use super::guard_release::release_held;
use super::guard_spawn::{COMMAND_PIDFD, GUARD_ENV, GUARD_PROTOCOL};
use crate::input::hyprland_keyboard::held_key_journal::{instance_key, journal_dir_for};
use crate::input::hyprland_keyboard::wayland_wire::WaylandWire;
use crate::system::process::identity::Identity;
use crate::system::process::liveness;

const MAX_PAYLOAD_BYTES: u64 = 64 * 1024;
const RELEASE_BUDGET_MS: u64 = 2_000;

/// Runs the key guard when this process was started as one.
pub(crate) fn entry_from_env() -> Option<Result<(), AdapterError>> {
    let marker = std::env::var_os(GUARD_ENV)?;
    if marker != GUARD_PROTOCOL {
        return Some(Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            format!("This build serves key guard protocol {GUARD_PROTOCOL} only"),
        )));
    }
    Some(run())
}

fn run() -> Result<(), AdapterError> {
    let payload = read_payload()?;
    let command = unsafe { OwnedFd::from_raw_fd(COMMAND_PIDFD) };
    wait_for_exit(command.as_fd());
    if !command_gone(&payload) {
        return Err(AdapterError::internal(
            "The key guard woke while its command still runs",
        ));
    }
    let record = journal_dir_for(&payload.binding).and_then(|dir| {
        read_held(
            &dir,
            payload.command_pid,
            &payload.command_instance,
            &instance_key(&payload.binding),
        )
    });
    let deadline = Deadline::detached_after(RELEASE_BUDGET_MS)?;
    let outcome = release_held(record, payload.layout, deadline, |deadline| {
        WaylandWire::open(&payload.binding, deadline)
    })?;
    tracing::debug!(?outcome, "key guard done");
    Ok(())
}

fn read_payload() -> Result<GuardPayload, AdapterError> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_PAYLOAD_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            AdapterError::internal("The key guard could not read its payload")
                .with_platform_detail(error.to_string())
        })?;
    GuardPayload::from_json(&bytes).ok_or_else(|| {
        AdapterError::new(ErrorCode::InvalidArgs, "The key guard payload is invalid")
    })
}

/// Blocks until `pidfd` reports its process gone, or cannot be polled.
pub(crate) fn wait_for_exit(pidfd: BorrowedFd<'_>) {
    let _ = (PollFd::new(&pidfd, PollFlags::IN), poll, Errno::INTR);
}

/// Whether the command the payload names has exited, so a pidfd that could
/// not be polled never releases keys under a live command.
pub(crate) fn command_gone(payload: &GuardPayload) -> bool {
    let _ = (payload, liveness::identity, Identity::Gone);
    false
}

#[cfg(test)]
#[path = "guard_entry_tests.rs"]
mod tests;
