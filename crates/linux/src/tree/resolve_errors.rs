use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use crate::system::{listed_app_name, proc_stat, process_facts};

fn settled(message: &str, kind: &str) -> AdapterError {
    AdapterError::stale_ref_because(message).with_details(json!({
        "kind": kind,
        "complete": true,
        "retryable": false,
    }))
}

fn unsettled(message: &str, kind: &str) -> AdapterError {
    AdapterError::stale_ref_because(message).with_details(json!({
        "kind": kind,
        "complete": true,
        "retryable": true,
    }))
}

/// The search read the whole tree and no node answers to the stored evidence.
/// The element may be drawn again, so the answer stays retryable the way the
/// other platforms keep it.
pub(crate) fn no_candidate() -> AdapterError {
    unsettled(
        "Stored ref does not match any live element",
        "resolve_no_candidate",
    )
}

/// The process that owned the ref no longer exists.
pub(crate) fn app_exited() -> AdapterError {
    settled(
        "Stored ref's owning application has exited",
        "resolve_owning_process_exited",
    )
}

/// A missing bus owner does not prove that its process exited.
pub(crate) fn owner_unreachable(pid: u32) -> AdapterError {
    if matches!(process_facts::lookup(pid), process_facts::Lookup::Gone) {
        return app_exited();
    }
    settled(
        "Stored ref's owning application is not reachable on this session's accessibility bus",
        "resolve_owner_unreachable",
    )
    .with_suggestion(
        "Return to the desktop session where this ref was captured, run 'snapshot', then retry with the new ref.",
    )
}

/// The element the ref names was destroyed while its application kept
/// running. The element may be drawn again, so the answer stays retryable the
/// way the other platforms keep it.
pub(crate) fn object_gone() -> AdapterError {
    unsettled(
        "Stored ref's element no longer exists",
        "resolve_target_destroyed",
    )
}

/// The window id now belongs to a different process, or to a process whose
/// identity the ref cannot vouch for.
pub(crate) fn window_replaced() -> AdapterError {
    settled(
        "Stored ref's window now belongs to a different process",
        "resolve_window_replaced",
    )
}

/// The ref carries no process-instance token to verify its window with.
pub(crate) fn window_unverifiable() -> AdapterError {
    settled(
        "Stored ref has no process-instance token to verify its window with",
        "resolve_window_unverifiable",
    )
}

/// The ref carries nothing any live element could be checked against.
pub(crate) fn unverifiable_entry() -> AdapterError {
    settled(
        "Stored ref carries no identifier, no stable text and no usable bounds to match a live element with",
        "resolve_unverifiable_entry",
    )
}

pub(crate) fn ambiguous(candidates: usize) -> AdapterError {
    AdapterError::ambiguous_target("Multiple live elements match the stored identity").with_details(
        json!({
            "kind": "resolve_ambiguous",
            "candidate_count": candidates,
            "complete": true,
            "retryable": false,
        }),
    )
}

/// A bus departure is settled against process liveness, a destroyed object
/// stays retryable, and anything else is unchanged.
pub(crate) fn settle_gone(error: AdapterError, pid: u32) -> AdapterError {
    match error.code {
        ErrorCode::AppNotFound => owner_unreachable(pid),
        ErrorCode::ElementNotFound => object_gone(),
        _ => error,
    }
}

const INCOMPLETE_KIND: &str = "resolution_identity_unknown";

/// Whether the error is the retryable answer for an observation that could not
/// be completed, rather than a settled refusal.
pub(crate) fn is_incomplete(error: &AdapterError) -> bool {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        == Some(INCOMPLETE_KIND)
}

/// A timeout, or an observation left incomplete, on a process that `/proc`
/// shows stopped is `APP_UNRESPONSIVE` naming the application, the answer the
/// scoped listings give for the same state. An incomplete identity on an
/// answering process is a retryable stale ref, never proof of an unresponsive
/// application. Every other error passes through.
pub(crate) fn stopped_aware(mut error: AdapterError, stopped_app: Option<&str>) -> AdapterError {
    match stopped_app {
        Some(name) if error.code == ErrorCode::Timeout || is_incomplete(&error) => {
            AdapterError::app_unresponsive(name)
        }
        None if is_incomplete(&error) => {
            error.code = ErrorCode::StaleRef;
            error
        }
        _ => error,
    }
}

/// The listed name of `pid`'s application when the process is stopped now.
pub(crate) fn stopped_app(pid: u32) -> Option<String> {
    proc_stat::is_stopped(pid)
        .then(|| process_facts::read(pid))
        .flatten()
        .map(|facts| listed_app_name(&facts))
}

#[cfg(test)]
#[path = "resolve_errors_tests.rs"]
mod tests;
