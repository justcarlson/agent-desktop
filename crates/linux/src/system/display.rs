use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, ErrorCode, Rect};

use super::display_mutter::{self, LogicalMonitor};
use super::display_server::{Cause, DisplayServer, Unavailable, session_server};
use super::x11::{self, MonitorFault, MonitorReader, MonitorSource, X11Monitor};

fn wayland_missing() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Displays cannot be listed on this Wayland session",
    )
    .with_suggestion(
        "Run under GNOME, whose compositor answers the Mutter DisplayConfig \
         interface; no other Wayland display source is measured",
    )
}

pub(crate) fn unreachable(why: Unavailable) -> AdapterError {
    let (message, suggestion) = match why.cause {
        Cause::NotConfigured => (
            "No display server is configured in this session".to_owned(),
            "Run inside a desktop session with DISPLAY or WAYLAND_DISPLAY set",
        ),
        Cause::Missing => (
            format!(
                "The session's display server is not running: {}",
                why.detail
            ),
            "Start the display server, or correct DISPLAY and WAYLAND_DISPLAY",
        ),
        Cause::Refused => (
            format!(
                "The session's display server refused the connection: {}",
                why.detail
            ),
            "Check that the display server accepts this user, or correct DISPLAY and WAYLAND_DISPLAY",
        ),
    };
    AdapterError::new(ErrorCode::PlatformNotSupported, message)
        .with_suggestion(suggestion)
        .with_details(serde_json::json!({
            "display_unavailable": why.cause.as_str(),
            "detail": why.detail,
        }))
}

fn unreadable() -> AdapterError {
    AdapterError::internal("The display server's monitor list could not be read")
        .with_suggestion("Retry once the X session answers")
}

fn denied(display: &str, reason: &str) -> AdapterError {
    unreachable(Unavailable {
        cause: Cause::Refused,
        detail: format!("DISPLAY={display} refused this client: {reason}"),
    })
}

fn not_answering(display: &str, detail: &str) -> AdapterError {
    unreachable(Unavailable {
        cause: Cause::Missing,
        detail: format!("DISPLAY={display} did not answer: {detail}"),
    })
}

fn silent() -> AdapterError {
    AdapterError::timeout("The display server did not answer in time")
        .with_suggestion("Retry, or check that DISPLAY names a server that is running")
}

/// The answer for a monitor read that failed or never finished: a refusal and
/// an address nobody answers at are permanent and say so, a read that ran out
/// of time is a timeout, and only a reply that could not be used is the
/// generic read failure.
pub(crate) fn fault_error(display: &str, fault: Option<MonitorFault>) -> AdapterError {
    match fault {
        Some(MonitorFault::Denied(reason)) => denied(display, &reason),
        Some(MonitorFault::Unreachable(detail)) => not_answering(display, &detail),
        Some(MonitorFault::Unreadable) => unreadable(),
        None => silent(),
    }
}

fn scale_of(monitor: &X11Monitor, logical: &[LogicalMonitor]) -> f64 {
    logical
        .iter()
        .find(|candidate| {
            candidate.connectors.iter().any(|connector| {
                connector == &monitor.name || monitor.connectors.contains(connector)
            })
        })
        .map(|candidate| candidate.scale)
        .unwrap_or(1.0)
}

fn primaries_first(displays: &mut [DisplayInfo]) {
    displays.sort_by(|left, right| {
        right
            .is_primary
            .cmp(&left.is_primary)
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn assemble(monitors: Vec<X11Monitor>, logical: &[LogicalMonitor]) -> Vec<DisplayInfo> {
    let mut displays: Vec<DisplayInfo> = monitors
        .into_iter()
        .map(|monitor| {
            let scale = scale_of(&monitor, logical);
            DisplayInfo {
                id: monitor.name,
                bounds: Rect::from(monitor.bounds),
                is_primary: monitor.primary,
                scale,
            }
        })
        .collect();
    if !displays.iter().any(|display| display.is_primary)
        && let Some(first) = displays.first_mut()
    {
        first.is_primary = true;
    }
    primaries_first(&mut displays);
    displays
}

fn from_mutter(logical: Vec<LogicalMonitor>) -> Vec<DisplayInfo> {
    let mut displays: Vec<DisplayInfo> = logical
        .into_iter()
        .map(|monitor| DisplayInfo {
            id: monitor.connectors.join("+"),
            bounds: Rect {
                x: f64::from(monitor.x),
                y: f64::from(monitor.y),
                width: f64::from(monitor.width),
                height: f64::from(monitor.height),
            },
            is_primary: monitor.primary,
            scale: monitor.scale,
        })
        .collect();
    primaries_first(&mut displays);
    displays
}

/// Decides the display answer from the measured inputs, pure over plain
/// records so every rule is testable without a session.
pub(crate) fn decide(
    server: DisplayServer,
    monitors: Option<Vec<X11Monitor>>,
    mutter: Option<Vec<LogicalMonitor>>,
) -> Result<Vec<DisplayInfo>, AdapterError> {
    match server {
        DisplayServer::Wayland => match mutter {
            Some(logical) => Ok(from_mutter(logical)),
            None => Err(wayland_missing()),
        },
        DisplayServer::X11(_) => match monitors {
            Some(list) => Ok(assemble(list, mutter.as_deref().unwrap_or_default())),
            None => Err(unreadable()),
        },
        DisplayServer::Unavailable(why) => Err(unreachable(why)),
    }
}

/// The X11 answer. Mutter only supplies scales there, so a Mutter that
/// failed or ran out of time leaves every scale at one, and the RandR
/// monitors are still reported.
pub(crate) fn decide_x11(
    server: DisplayServer,
    monitors: Result<Vec<X11Monitor>, AdapterError>,
    mutter: Result<Option<Vec<LogicalMonitor>>, AdapterError>,
) -> Result<Vec<DisplayInfo>, AdapterError> {
    decide(server, Some(monitors?), mutter.unwrap_or(None))
}

/// The displays of a Wayland session as Mutter lists them, with no look at the
/// environment: a caller that already chose the Wayland route asks here.
pub(crate) fn list_wayland_displays(deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
    decide(
        DisplayServer::Wayland,
        None,
        display_mutter::logical_monitors(deadline)?,
    )
}

/// Lists the session's displays. On X11 the monitor list comes from RandR
/// and Mutter's `DisplayConfig` supplies each monitor's scale where it
/// answers; on Wayland Mutter is the only measured source and a session
/// without it is `PLATFORM_NOT_SUPPORTED`.
pub(crate) fn list_displays(deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
    match session_server() {
        DisplayServer::Wayland => list_wayland_displays(deadline),
        DisplayServer::X11(display) => {
            let pending = x11::start(|| MonitorReader.read());
            let mutter =
                display_mutter::logical_monitors_within(deadline, display_mutter::OPTIONAL_SLICE);
            let monitors = match pending.finish(deadline) {
                Some(Ok(list)) => Ok(list),
                Some(Err(fault)) => Err(fault_error(&display, Some(fault))),
                None => Err(fault_error(&display, None)),
            };
            decide_x11(DisplayServer::X11(display), monitors, mutter)
        }
        server @ DisplayServer::Unavailable(_) => decide(server, None, None),
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
