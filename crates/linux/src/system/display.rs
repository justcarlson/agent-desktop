use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, ErrorCode, Rect};

use super::display_mutter::{self, LogicalMonitor};
use super::x11::{self, DisplayServer, MonitorReader, MonitorSource, X11Monitor, session_server};

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

fn headless() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "No display server is running in this session",
    )
    .with_suggestion("Run inside a desktop session with DISPLAY or WAYLAND_DISPLAY set")
}

fn unreadable() -> AdapterError {
    AdapterError::internal("The display server's monitor list could not be read")
        .with_suggestion("Retry once the X session answers")
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
                bounds: Rect {
                    x: f64::from(monitor.bounds.x),
                    y: f64::from(monitor.bounds.y),
                    width: f64::from(monitor.bounds.width),
                    height: f64::from(monitor.bounds.height),
                },
                is_primary: monitor.primary,
                scale,
            }
        })
        .collect();
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
        DisplayServer::None => Err(headless()),
    }
}

/// Lists the session's displays. On X11 the monitor list comes from RandR
/// and Mutter's `DisplayConfig` supplies each monitor's scale where it
/// answers; on Wayland Mutter is the only measured source and a session
/// without it is `PLATFORM_NOT_SUPPORTED`.
pub(crate) fn list_displays(deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
    match session_server() {
        DisplayServer::Wayland => decide(
            DisplayServer::Wayland,
            None,
            display_mutter::logical_monitors(deadline)?,
        ),
        server @ DisplayServer::X11(_) => {
            let pending = x11::start(|| MonitorReader.read());
            let mutter = display_mutter::logical_monitors(deadline)?;
            decide(server, pending.finish(deadline).flatten(), mutter)
        }
        DisplayServer::None => decide(DisplayServer::None, None, None),
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
