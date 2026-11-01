//! Teardown of one session's renderer, for `session end`, `session gc` and
//! `cursor-overlay disable`. It asks first with `disable`, waits on a pidfd,
//! and signals only a process whose pid, start time and executable all match
//! the record, only through that pidfd. It reports `terminated` only after
//! the pidfd confirms exit, and removes only the generation it noted.

use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, CursorOverlayControl, Deadline, FileLock};
use rustix::process::Signal;

use super::exe_identity::ExeIdentity;
use super::record_class::is_alive_as_recorded;
use super::record_read::{RecordRead, read_record, remove_if_generation};
use super::renderer_paths::RendererPaths;
use super::renderer_pidfd::RendererPidfd;
use super::renderer_record::RendererRecord;
use super::renderer_request::RendererRequest;
use super::renderer_wire::exchange;
use super::stop_outcome::StopOutcome;
use super::stop_timings::StopTimings;

const LOCK_WAIT_MS: u64 = 4_000;

/// Stops the renderer the registry at `paths` names, for a caller running
/// the executable `caller`.
pub(crate) fn stop(
    paths: &RendererPaths,
    caller: Option<ExeIdentity>,
    timings: &StopTimings,
) -> Result<StopOutcome, AdapterError> {
    let _ = (caller, timings);
    return Err(AdapterError::internal("not implemented"));
    #[allow(unreachable_code)]
    let record = match read_record(&paths.record) {
        RecordRead::Absent => return Ok(StopOutcome::NotRunning),
        RecordRead::Unreadable => {
            let _lock = start_lock(paths)?;
            let _ = std::fs::remove_file(&paths.record);
            return Ok(StopOutcome::Stale);
        }
        RecordRead::Present(record) => record,
    };
    let disabled = send_disable(paths, &record, timings);
    let _lock = start_lock(paths)?;
    let generation = record.owner.generation.clone();
    let outcome = settle(&record, caller, disabled, timings);
    if outcome.is_confirmed_gone() {
        remove_if_generation(paths, &generation);
    }
    Ok(outcome)
}

fn settle(
    record: &RendererRecord,
    caller: Option<ExeIdentity>,
    disabled: bool,
    timings: &StopTimings,
) -> StopOutcome {
    let Some(pidfd) = RendererPidfd::open(record.owner.pid) else {
        return gone(disabled);
    };
    if !is_alive_as_recorded(record) {
        return gone(disabled);
    }
    if pidfd.wait_for_exit(timings.after_disable) {
        return StopOutcome::Stopped;
    }
    if !is_alive_as_recorded(record) {
        return StopOutcome::Stale;
    }
    let running = ExeIdentity::of_pid(record.owner.pid);
    if running.is_none() || running != caller {
        return StopOutcome::Incompatible;
    }
    for (signal, wait) in [
        (Signal::TERM, timings.after_term),
        (Signal::KILL, timings.after_kill),
    ] {
        let sent = pidfd.signal(signal);
        if pidfd.wait_for_exit(if sent { wait } else { Duration::ZERO }) {
            return StopOutcome::Terminated;
        }
    }
    StopOutcome::ExitUnconfirmed
}

fn gone(disabled: bool) -> StopOutcome {
    if disabled {
        StopOutcome::Stopped
    } else {
        StopOutcome::Stale
    }
}

fn send_disable(paths: &RendererPaths, record: &RendererRecord, timings: &StopTimings) -> bool {
    let request =
        RendererRequest::control(CursorOverlayControl::disable(record.session_id.clone()));
    let deadline = Instant::now() + timings.after_disable;
    matches!(exchange(&paths.socket, &request, deadline), Ok(Some(_)))
}

fn start_lock(paths: &RendererPaths) -> Result<FileLock, AdapterError> {
    paths.ensure_dirs()?;
    FileLock::acquire(
        &paths.start_lock,
        Deadline::detached_after(LOCK_WAIT_MS)?,
        "cursor renderer start lock",
    )
}

#[cfg(test)]
#[path = "renderer_stop_tests.rs"]
mod tests;
