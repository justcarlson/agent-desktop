use agent_desktop_core::{AdapterError, Deadline, ErrorCode, WindowFilter, WindowInfo};

use super::SharedBus;
use super::window_id;
use super::window_ops::list_windows;

fn not_found(detail: &str) -> AdapterError {
    AdapterError::new(ErrorCode::WindowNotFound, detail)
        .with_suggestion("Run 'list-windows' to see the windows that exist now")
}

/// Checks a stored window against a fresh listing: the process and its
/// start must be the same. The title is not compared because applications
/// retitle their windows, and the id already is the window's address.
pub(crate) fn verify(
    expected: &WindowInfo,
    live: Vec<WindowInfo>,
) -> Result<WindowInfo, AdapterError> {
    let Some(instance) = expected
        .process_instance
        .as_deref()
        .filter(|instance| !instance.is_empty())
    else {
        return Err(AdapterError::new(
            ErrorCode::StaleRef,
            "The stored window has no process-instance token to verify",
        ));
    };
    let found = live
        .into_iter()
        .find(|window| window.id == expected.id)
        .ok_or_else(|| not_found("The window is no longer open"))?;
    if found.pid != expected.pid || found.process_instance.as_deref() != Some(instance) {
        return Err(not_found(
            "The window id now belongs to a different process",
        ));
    }
    Ok(found)
}

pub(crate) fn resolve_window_strict(
    bus: &SharedBus,
    expected: &WindowInfo,
    deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    window_id::decode(&expected.id)?;
    let live = list_windows(bus, &WindowFilter::default(), deadline)?;
    verify(expected, live)
}

#[cfg(test)]
#[path = "window_resolve_tests.rs"]
mod tests;
