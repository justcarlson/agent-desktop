use agent_desktop_core::{AdapterError, Deadline, ErrorCode, WindowInfo};

use super::accessible_frames::{AccessibleFrames, frame_evidence};
use super::compositor_identity::require_toplevel;
use super::compositor_listing::hyprland_windows;
use super::window_id;
use super::window_ops::listed_of_process;
use crate::system::hyprland::identity::compositor_window_id::{is_compositor_id, is_well_formed};
use crate::system::hyprland::identity::window_evidence::WindowEvidence;
use crate::system::process::liveness::{Liveness, liveness};
use crate::system::registry::listed_window::ListedWindow;
use crate::system::{SharedBus, shell};

fn not_found(detail: &str) -> AdapterError {
    AdapterError::new(ErrorCode::WindowNotFound, detail)
        .with_suggestion("Run 'list-windows' to see the windows that exist now")
}

/// Checks a stored window against a fresh listing: the process and its
/// start must be the same. The title is not compared because applications
/// retitle their windows, and the id already is the window's address. The
/// listing shows a stopped process's windows, recovered from the display
/// server, so its window may still be there; resolving it is
/// `APP_UNRESPONSIVE` for its application either way, never a closed window.
pub(crate) fn verify(
    expected: &WindowInfo,
    live: Vec<WindowInfo>,
    process: Liveness,
) -> Result<WindowInfo, AdapterError> {
    if process == Liveness::Stopped {
        return Err(AdapterError::app_unresponsive(&expected.app));
    }
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
    let Some(found) = live.into_iter().find(|window| window.id == expected.id) else {
        return Err(not_found("The window is no longer open"));
    };
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
    if is_compositor_id(&expected.id) {
        let live = || hyprland_windows(|| Ok(AccessibleFrames::default()), None, deadline);
        return resolve_compositor(expected, live);
    }
    window_id::decode(&expected.id)?;
    if shell::is_shell_window(expected) {
        shell::window_gate::verify(bus, &expected.id, deadline)?;
        return Ok(expected.clone());
    }
    let listed = listed_of_process(bus, expected.pid.get(), deadline)?;
    resolve_accessible(expected, listed, process_of(expected), |evidence| {
        require_toplevel(evidence, deadline)
    })
}

/// Verifies a stored accessibility window against its process's fresh
/// windows, then lets `identity` refuse the window found. On a bound
/// Hyprland, `identity` requires exactly one compositor toplevel.
pub(crate) fn resolve_accessible(
    expected: &WindowInfo,
    listed: Vec<ListedWindow>,
    process: Liveness,
    identity: impl FnOnce(&WindowEvidence) -> Result<(), AdapterError>,
) -> Result<WindowInfo, AdapterError> {
    let evidence = listed
        .iter()
        .find(|window| window.info.id == expected.id)
        .map(frame_evidence);
    let found = verify(
        expected,
        listed.into_iter().map(|window| window.info).collect(),
        process,
    )?;
    if let Some(evidence) = evidence {
        identity(&evidence)?;
    }
    Ok(found)
}

/// Resolves a compositor-only window against the bound compositor's
/// listing. Its id names the compositor process and start that issued it,
/// so after a compositor restart the old id is no longer listed. A
/// malformed id is refused before the compositor is read.
pub(crate) fn resolve_compositor(
    expected: &WindowInfo,
    live: impl FnOnce() -> Result<Vec<WindowInfo>, AdapterError>,
) -> Result<WindowInfo, AdapterError> {
    if !is_well_formed(&expected.id) {
        return Err(window_id::invalid(&expected.id));
    }
    verify(expected, live()?, process_of(expected))
}

fn process_of(expected: &WindowInfo) -> Liveness {
    expected
        .process_instance
        .as_deref()
        .filter(|instance| !instance.is_empty())
        .map_or(Liveness::Alive, |instance| {
            liveness(expected.pid.get(), instance)
        })
}

#[cfg(test)]
#[path = "window_resolve_tests.rs"]
mod tests;
