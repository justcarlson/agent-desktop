//! The desktop changes a focus caused beyond the focus itself, found by
//! comparing what each monitor showed before and after. A monitor that is
//! present on one side only was added or removed, which a focus does not
//! do, so it is not diffed.

use agent_desktop_core::FocusSideEffect;

use super::desktop_state::DesktopState;
use super::monitor_workspaces::MonitorWorkspaces;

/// Every workspace switch, special workspace closed and special workspace
/// revealed between `before` and `after`, monitor by monitor, then the
/// cursor move when both reads saw the cursor.
pub(crate) fn side_effects(before: &DesktopState, after: &DesktopState) -> Vec<FocusSideEffect> {
    let mut effects = Vec::new();
    for now in &after.monitors {
        if let Some(was) = before.monitors.iter().find(|m| m.monitor == now.monitor) {
            monitor_effects(was, now, &mut effects);
        }
    }
    if let (Some(from), Some(to)) = (&before.cursor, &after.cursor)
        && from != to
    {
        effects.push(FocusSideEffect::CursorMoved {
            from: from.clone(),
            to: to.clone(),
        });
    }
    effects
}

fn monitor_effects(
    was: &MonitorWorkspaces,
    now: &MonitorWorkspaces,
    out: &mut Vec<FocusSideEffect>,
) {
    if was.active != now.active {
        out.push(FocusSideEffect::WorkspaceSwitched {
            monitor: now.monitor.clone(),
            from: was.active.clone(),
            to: now.active.clone(),
        });
    }
    if was.special == now.special {
        return;
    }
    if let Some(workspace) = &was.special {
        out.push(FocusSideEffect::SpecialWorkspaceClosed {
            monitor: now.monitor.clone(),
            workspace: workspace.clone(),
        });
    }
    if let Some(workspace) = &now.special {
        out.push(FocusSideEffect::SpecialWorkspaceRevealed {
            monitor: now.monitor.clone(),
            workspace: workspace.clone(),
        });
    }
}

#[cfg(test)]
#[path = "side_effects_tests.rs"]
mod tests;
