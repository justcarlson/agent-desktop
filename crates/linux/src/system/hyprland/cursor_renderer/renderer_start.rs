//! The lazy start. A caller that finds no renderer answering takes the
//! session's start lock, looks again, and classifies the record: a valid
//! renderer still starting is waited for, another build's live renderer is
//! refused and left alone, and a stale record is removed for its own
//! generation only. Exactly one caller launches.

use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, CursorOverlayControl, Deadline, DeliverySemantics, ErrorCode, FileLock,
};

use super::exe_identity::ExeIdentity;
use super::record_class::RecordClass;
use super::record_read::{RecordRead, read_record, remove_if_generation};
use super::renderer_launcher::RendererLauncher;
use super::renderer_paths::RendererPaths;
use super::renderer_record::RendererRecord;
use super::renderer_target::RendererTarget;

const SOCKET_POLL: Duration = Duration::from_millis(10);

/// What `ensure_running` needs: where, for which compositor, and how long.
pub(crate) struct StartRequest<'a> {
    pub(crate) paths: &'a RendererPaths,
    pub(crate) target: &'a RendererTarget,
    pub(crate) first: &'a CursorOverlayControl,
    pub(crate) deadline: Instant,
}

/// Makes sure a renderer answers on `request.paths.socket`, starting one
/// through `launcher` when none does.
pub(crate) fn ensure_running(
    request: &StartRequest<'_>,
    launcher: &dyn RendererLauncher,
    caller: Option<ExeIdentity>,
) -> Result<(), AdapterError> {
    let paths = request.paths;
    if accepts(paths) {
        return Ok(());
    }
    paths.ensure_dirs()?;
    let _lock = FileLock::acquire(
        &paths.start_lock,
        Deadline::detached_after(millis_left(request.deadline))?,
        "cursor renderer start lock",
    )?;
    if accepts(paths) {
        return Ok(());
    }
    match read_record(&paths.record) {
        RecordRead::Present(record) => match RecordClass::of(record, caller) {
            RecordClass::Valid(_) => return wait_for_socket(paths, request.deadline),
            RecordClass::IncompatibleLive(record) => return Err(incompatible(&record)),
            RecordClass::Stale(record) => {
                remove_if_generation(paths, &record.owner.generation);
            }
        },
        RecordRead::Unreadable => {
            let _ = std::fs::remove_file(&paths.record);
            let _ = std::fs::remove_file(&paths.socket);
        }
        RecordRead::Absent => {
            let _ = std::fs::remove_file(&paths.socket);
        }
    }
    if !launcher.session_active(request.first) {
        return Ok(());
    }
    launcher.launch(request.target, paths, request.first)?;
    wait_for_socket(paths, request.deadline)
}

/// Whether a renderer accepts connections on the session's socket.
pub(crate) fn accepts(paths: &RendererPaths) -> bool {
    UnixStream::connect(&paths.socket).is_ok()
}

fn wait_for_socket(paths: &RendererPaths, deadline: Instant) -> Result<(), AdapterError> {
    while Instant::now() < deadline {
        if accepts(paths) {
            return Ok(());
        }
        std::thread::sleep(SOCKET_POLL);
    }
    Err(
        AdapterError::timeout("The cursor renderer did not start listening in time")
            .with_details(serde_json::json!({"kind": "cursor_renderer_start_timeout"}))
            .with_disposition(DeliverySemantics::not_delivered()),
    )
}

fn millis_left(deadline: Instant) -> u64 {
    u64::try_from(
        deadline
            .saturating_duration_since(Instant::now())
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
    .max(1)
}

fn incompatible(record: &RendererRecord) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Another agent-desktop build's cursor renderer is running for this session",
    )
    .with_suggestion("End the session with the build that started it, or start a new session")
    .with_platform_detail(format!(
        "pid {} runs {}",
        record.owner.pid,
        record.owner.exe.display()
    ))
    .with_details(serde_json::json!({
        "kind": "cursor_renderer_incompatible",
        "pid": record.owner.pid,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "renderer_start_tests.rs"]
mod tests;
