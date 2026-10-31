//! What the bound instance shows: the active window, every monitor's
//! active and special workspace and the cursor, from `j/activewindow`,
//! `j/monitors` and `j/cursorpos`.

use agent_desktop_core::Point;
use serde_json::Value;

use super::client_workspace::ClientWorkspace;
use super::monitor_workspaces::MonitorWorkspaces;

const SPECIAL_PREFIX: &str = "special:";

/// The active window's address, every monitor's workspaces and the cursor
/// position at one read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesktopState {
    pub(crate) active: Option<u64>,
    pub(crate) monitors: Vec<MonitorWorkspaces>,
    pub(crate) cursor: Option<Point>,
}

impl DesktopState {
    /// Whether some monitor shows `workspace`, as its active workspace or as
    /// the special workspace open over it.
    pub(crate) fn shows(&self, workspace: &str) -> bool {
        self.monitors.iter().any(|monitor| {
            monitor.active == workspace || monitor.special.as_deref() == Some(workspace)
        })
    }

    /// The monitor that shows `workspace` when it holds a special workspace
    /// open in front of it. Hyprland keeps that special workspace in front of
    /// an activated window on a regular workspace.
    pub(crate) fn covering_special(
        &self,
        workspace: &ClientWorkspace,
    ) -> Option<&MonitorWorkspaces> {
        if workspace.name.starts_with(SPECIAL_PREFIX) {
            return None;
        }
        let monitor = workspace.monitor?;
        self.monitors
            .iter()
            .find(|shown| shown.id == Some(monitor) && shown.special.is_some())
    }
}

/// Every monitor's workspaces from a `j/monitors` reply, in reply order. A
/// special workspace with an empty name is none.
pub(crate) fn parse_monitors(body: &[u8]) -> Option<Vec<MonitorWorkspaces>> {
    let monitors: Vec<Value> = serde_json::from_slice(body).ok()?;
    monitors.iter().map(monitor_of).collect()
}

fn monitor_of(monitor: &Value) -> Option<MonitorWorkspaces> {
    let name_of = |key: &str| {
        monitor
            .get(key)
            .and_then(|workspace| workspace.get("name"))
            .and_then(Value::as_str)
    };
    Some(MonitorWorkspaces {
        monitor: monitor.get("name")?.as_str()?.to_owned(),
        id: monitor.get("id").and_then(Value::as_i64),
        active: name_of("activeWorkspace")?.to_owned(),
        special: name_of("specialWorkspace")
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
    })
}

/// The cursor position from a `j/cursorpos` reply.
pub(crate) fn parse_cursor(body: &[u8]) -> Option<Point> {
    let cursor: Value = serde_json::from_slice(body).ok()?;
    Some(Point {
        x: cursor.get("x")?.as_f64()?,
        y: cursor.get("y")?.as_f64()?,
    })
}

#[cfg(test)]
#[path = "desktop_state_tests.rs"]
mod tests;
