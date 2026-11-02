//! What a window operation sends, decided from the window's state before
//! anything is sent. An empty plan means the window already is as asked.

use agent_desktop_core::{AdapterError, WindowOp};

use super::client_state::ClientState;
use super::config_dialect::ConfigDialect;
use super::step_goal::StepGoal;
use super::window_command::{
    maximize_command, move_command, resize_command, unset_fullscreen_command, workspace_command,
};
use super::window_op_refusal::refused;
use super::window_step::WindowStep;

/// The special workspace a minimized window waits on, followed by the name
/// of the workspace it came from.
pub(crate) const MINIMIZED_PREFIX: &str = "special:agent-desktop-minimized-";

const SPECIAL_PREFIX: &str = "special:";

const MAXIMIZED: i64 = 1;
const FULLSCREEN: i64 = 2;

/// The steps of `op` on the window in `state`, with `active` the focused
/// window, or the refusal that keeps the request from being sent.
pub(crate) fn plan(
    op: &WindowOp,
    state: &ClientState,
    active: Option<u64>,
    dialect: ConfigDialect,
) -> Result<Vec<WindowStep>, AdapterError> {
    let address = state.address;
    match *op {
        WindowOp::Move { x, y } => {
            placeable(state)?;
            let point = (x.round() as i64, y.round() as i64);
            let goal = StepGoal::At(point.0, point.1);
            if goal.holds(state) {
                return Ok(Vec::new());
            }
            Ok(vec![WindowStep::new(
                move_command(dialect, address, point.0, point.1),
                vec![goal],
            )])
        }
        WindowOp::Resize { width, height } => {
            placeable(state)?;
            let size = (width.round() as i64, height.round() as i64);
            let goal = StepGoal::Size(size.0, size.1);
            if goal.holds(state) {
                return Ok(Vec::new());
            }
            let (x, y) = state.at;
            Ok(vec![
                WindowStep::new(
                    resize_command(dialect, address, size.0, size.1),
                    vec![StepGoal::Resized {
                        wanted: size,
                        from: state.size,
                    }],
                ),
                WindowStep::new(
                    move_command(dialect, address, x, y),
                    vec![StepGoal::At(x, y), goal],
                ),
            ])
        }
        WindowOp::Minimize => not_hidden_by_another(state).and_then(|()| minimize(state, dialect)),
        WindowOp::Maximize => {
            not_hidden_by_another(state).and_then(|()| maximize(state, active, dialect))
        }
        WindowOp::Restore => not_hidden_by_another(state).and_then(|()| restore(state, dialect)),
    }
}

/// The workspace a minimized window came from, read from the special
/// workspace it waits on.
pub(crate) fn minimized_origin(workspace: &str) -> Option<&str> {
    workspace
        .strip_prefix(MINIMIZED_PREFIX)
        .filter(|origin| nameable(origin))
}

fn nameable(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// A window on a special workspace that no minimize of ours names is hidden
/// by another tool. Its state belongs to that tool, so minimize, maximize and
/// restore leave it alone.
fn not_hidden_by_another(state: &ClientState) -> Result<(), AdapterError> {
    if state.workspace.starts_with(SPECIAL_PREFIX) && minimized_origin(&state.workspace).is_none() {
        return Err(refused(
            "foreign_special_workspace",
            "Another tool hid the window on a special workspace",
            "Bring the window back with the tool that hid it, then retry",
        ));
    }
    Ok(())
}

fn placeable(state: &ClientState) -> Result<(), AdapterError> {
    if minimized_origin(&state.workspace).is_some() {
        return Err(minimized());
    }
    if state.fullscreen != 0 {
        return Err(refused(
            "window_maximized",
            "Hyprland does not move or resize a maximized or fullscreen window",
            "Run 'restore' on the window first, then retry",
        ));
    }
    if !state.floating {
        return Err(refused(
            "tiled_window",
            "Hyprland places a tiled window by its layout, so moving or resizing it would change the other windows",
            "Make the window floating yourself, then retry",
        ));
    }
    Ok(())
}

fn minimized() -> AdapterError {
    refused(
        "window_minimized",
        "The window is minimized",
        "Run 'restore' on the window first, then retry",
    )
}

fn minimize(state: &ClientState, dialect: ConfigDialect) -> Result<Vec<WindowStep>, AdapterError> {
    if minimized_origin(&state.workspace).is_some() {
        return Ok(Vec::new());
    }
    if !nameable(&state.workspace) {
        return Err(refused(
            "workspace_name_unsupported",
            "The window's workspace has a name that a minimized window cannot record",
            "Move the window to a numbered workspace, then retry",
        ));
    }
    let parked = format!("{MINIMIZED_PREFIX}{}", state.workspace);
    Ok(vec![WindowStep::new(
        workspace_command(dialect, state.address, &parked),
        vec![StepGoal::Workspace(parked)],
    )])
}

fn maximize(
    state: &ClientState,
    active: Option<u64>,
    dialect: ConfigDialect,
) -> Result<Vec<WindowStep>, AdapterError> {
    if minimized_origin(&state.workspace).is_some() {
        return Err(minimized());
    }
    match state.fullscreen {
        MAXIMIZED => return Ok(Vec::new()),
        FULLSCREEN => {
            return Err(refused(
                "window_fullscreen",
                "The window is fullscreen",
                "Run 'restore' on the window first, then retry",
            ));
        }
        _ => {}
    }
    if dialect == ConfigDialect::Legacy {
        return Err(legacy_focused_only());
    }
    if active.is_some_and(|active| active != state.address) {
        return Err(refused(
            "maximize_takes_focus",
            "Hyprland takes focus from the focused window when it maximizes another window",
            "Run 'focus-window' on the window first, then retry",
        ));
    }
    Ok(vec![WindowStep::new(
        maximize_command(state.address),
        vec![StepGoal::Fullscreen(MAXIMIZED)],
    )])
}

fn restore(state: &ClientState, dialect: ConfigDialect) -> Result<Vec<WindowStep>, AdapterError> {
    if state.fullscreen != 0 && dialect == ConfigDialect::Legacy {
        return Err(legacy_focused_only());
    }
    let mut steps = Vec::new();
    if let Some(origin) = minimized_origin(&state.workspace) {
        steps.push(WindowStep::new(
            workspace_command(dialect, state.address, origin),
            vec![StepGoal::Workspace(origin.to_owned())],
        ));
    }
    if state.fullscreen != 0 {
        steps.push(WindowStep::new(
            unset_fullscreen_command(state.address),
            vec![StepGoal::Fullscreen(0)],
        ));
    }
    Ok(steps)
}

fn legacy_focused_only() -> AdapterError {
    refused(
        "legacy_config_focused_only",
        "A Hyprland legacy config changes maximized state only on the focused window",
        "Change the window's maximized state yourself, or run Hyprland with a Lua config",
    )
}

#[cfg(test)]
#[path = "window_plan_tests.rs"]
mod tests;
