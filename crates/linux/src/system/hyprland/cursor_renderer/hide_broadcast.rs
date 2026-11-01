//! Sending `hide` and `show` to every renderer on the compositor at once. A
//! renderer that acknowledges counts as hidden, and so does one whose
//! process is gone, because the compositor removed its surfaces with it. A
//! live renderer that does not acknowledge before the deadline refuses the
//! hide; a refused connection from a live renderer is retried until then,
//! since the renderer may still be starting.

use std::time::{Duration, Instant};

use agent_desktop_core::ErrorCode;

use super::hide_refusal::HideRefusal;
use super::record_class::is_alive_as_recorded;
use super::renderer_paths::RendererPaths;
use super::renderer_record::RendererRecord;
use super::renderer_request::RendererRequest;
use super::renderer_wire::exchange;

/// How long `hide` and `show` may take, for all renderers together.
pub(crate) const HIDE_DEADLINE_MS: u64 = 500;

const RETRY_PAUSE: Duration = Duration::from_millis(10);

/// Sends `request` to every renderer in `renderers` in parallel and returns
/// the pids that acknowledged it.
pub(crate) fn hide_every(
    renderers: &[(RendererPaths, RendererRecord)],
    request: &RendererRequest,
    deadline: Instant,
) -> Result<Vec<u32>, HideRefusal> {
    let answers: Vec<Result<bool, HideRefusal>> = std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(renderers.len());
        for (paths, record) in renderers {
            workers.push(scope.spawn(move || hide_one(paths, record, request, deadline)));
        }
        workers
            .into_iter()
            .map(|worker| {
                worker.join().unwrap_or_else(|_| {
                    Err(HideRefusal::Failed {
                        cause: "hide_worker",
                        detail: "a hide worker stopped".into(),
                    })
                })
            })
            .collect()
    });
    let mut acknowledged = Vec::new();
    for ((_, record), answer) in renderers.iter().zip(answers) {
        if answer? {
            acknowledged.push(record.owner.pid);
        }
    }
    Ok(acknowledged)
}

/// Sends `request` to one renderer. `Ok(true)` when it acknowledged,
/// `Ok(false)` when its process is gone.
pub(crate) fn hide_one(
    paths: &RendererPaths,
    record: &RendererRecord,
    request: &RendererRequest,
    deadline: Instant,
) -> Result<bool, HideRefusal> {
    let pid = record.owner.pid;
    loop {
        let answer = exchange(&paths.socket, request, deadline);
        if !is_alive_as_recorded(record) {
            return Ok(false);
        }
        match answer {
            Ok(Some(_)) => return Ok(true),
            Ok(None) if Instant::now() + RETRY_PAUSE < deadline => std::thread::sleep(RETRY_PAUSE),
            Ok(None) => return Err(HideRefusal::Timeout { pid }),
            Err(error) if error.code == ErrorCode::Timeout => {
                return Err(HideRefusal::Timeout { pid });
            }
            Err(error) => {
                let kind = error
                    .details
                    .as_ref()
                    .and_then(|details| details["kind"].as_str())
                    .unwrap_or("cursor_renderer_failed")
                    .to_owned();
                return Err(HideRefusal::Failed {
                    cause: "renderer_refused",
                    detail: format!("renderer {pid}: {kind}: {}", error.message),
                });
            }
        }
    }
}

/// Sends `request` to every renderer in parallel and ignores the answers: a
/// late or failed `show` never changes a command's result, because each
/// renderer rereads the holders and draws again once none is live.
pub(crate) fn show_every(
    renderers: &[(RendererPaths, RendererRecord)],
    request: &RendererRequest,
    deadline: Instant,
) {
    std::thread::scope(|scope| {
        for (paths, _) in renderers {
            scope.spawn(move || {
                let _ = exchange(&paths.socket, request, deadline);
            });
        }
    });
}

#[cfg(test)]
#[path = "hide_broadcast_tests.rs"]
mod tests;
