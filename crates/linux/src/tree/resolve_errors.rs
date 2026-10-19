use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use crate::system::{listed_app_name, process_facts};

fn settled(message: &str, kind: &str) -> AdapterError {
    AdapterError::stale_ref_because(message).with_details(json!({
        "kind": kind,
        "complete": true,
        "retryable": false,
    }))
}

/// The search read the whole tree and no node answers to the stored evidence.
pub(crate) fn no_candidate() -> AdapterError {
    settled(
        "Stored ref does not match any live element",
        "resolve_no_candidate",
    )
}

/// The application that owned the ref has left the accessibility bus.
pub(crate) fn app_exited() -> AdapterError {
    settled(
        "Stored ref's owning application has exited",
        "resolve_owning_process_exited",
    )
}

/// The element the ref names was destroyed while its application kept running.
pub(crate) fn object_gone() -> AdapterError {
    settled(
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

/// A read error mapped to the answer a ref gets: an application that left the
/// bus and a destroyed object are settled, anything else is unchanged.
pub(crate) fn settle_gone(error: AdapterError) -> AdapterError {
    match error.code {
        ErrorCode::AppNotFound => app_exited(),
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
/// scoped listings give for the same state; without a stopped process both
/// stay as they were. Every other error passes through.
pub(crate) fn stopped_aware(error: AdapterError, stopped_app: Option<&str>) -> AdapterError {
    match stopped_app {
        Some(name) if error.code == ErrorCode::Timeout || is_incomplete(&error) => {
            AdapterError::app_unresponsive(name)
        }
        _ => error,
    }
}

/// The listed name of `pid`'s application when the process is stopped now.
pub(crate) fn stopped_app(pid: u32) -> Option<String> {
    process_facts::is_stopped(pid)
        .then(|| process_facts::read(pid))
        .flatten()
        .map(|facts| listed_app_name(&facts))
}

#[cfg(test)]
#[path = "resolve_errors_tests.rs"]
mod tests;
