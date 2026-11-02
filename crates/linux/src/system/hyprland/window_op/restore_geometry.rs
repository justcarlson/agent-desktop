//! The steps around a Lua maximize and restore that give a floating window
//! back its place and size. A maximize first records the window's offset
//! from its monitor's origin and its size in a restore tag. A restore that
//! clears the maximize then puts the window at that offset from where the
//! monitor is now, because Hyprland returns it to the absolute place it had,
//! which is off the output once the output moved (L46-3).

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, WindowOp};
use serde_json::json;

use super::client_state::ClientState;
use super::config_dialect::ConfigDialect;
use super::output_origin::{OutputOrigin, origin_of};
use super::restore_record::{RestoreRecord, restore_tags};
use super::step_goal::StepGoal;
use super::window_command::{move_command, resize_command, tag_command};
use super::window_op_refusal::refused;
use super::window_step::WindowStep;

/// Whether a maximize or restore of the window in `state` reads the
/// monitors' origins: a maximize of a floating window, or a restore of a
/// maximized window that carries a restore tag.
pub(crate) fn needs_origins(op: &WindowOp, state: &ClientState) -> bool {
    match op {
        WindowOp::Maximize => state.floating,
        WindowOp::Restore => state.fullscreen != 0 && !restore_tags(&state.marks.tags).is_empty(),
        _ => false,
    }
}

/// The steps a maximize sends before the maximize itself: every stale
/// restore tag removed, then a floating window's place and size recorded.
/// A floating window whose monitor has no origin in `origins` is refused.
pub(crate) fn recorded(
    state: &ClientState,
    origins: &[OutputOrigin],
) -> Result<Vec<WindowStep>, AdapterError> {
    let mut steps = untagged(state);
    if !state.floating {
        return Ok(steps);
    }
    let origin = origin_of(origins, state.marks.monitor).ok_or_else(|| {
        refused(
            "window_state_unreadable",
            "The compositor does not say where the window's monitor is, so a restore could not put the window back",
            "Run a Hyprland release whose 'hyprctl clients -j' and 'hyprctl monitors -j' name each window's monitor and its place",
        )
    })?;
    let record = RestoreRecord {
        offset: (state.at.0 - origin.0, state.at.1 - origin.1),
        size: state.size,
    };
    let tag = record.tag();
    steps.push(WindowStep::new(
        tag_command(state.address, &tag, true),
        vec![StepGoal::Tagged(tag)],
    ));
    Ok(steps)
}

/// The steps a restore sends after the maximize is cleared: a floating
/// window with one readable record goes to the recorded offset from its
/// monitor's origin, at the recorded size, and every restore tag then goes.
/// `held` is the window just before the maximize was cleared, on the
/// monitor it filled; `after` is the window once it was cleared.
pub(crate) fn placed(
    held: &ClientState,
    after: &ClientState,
    origins: &[OutputOrigin],
) -> Result<Vec<WindowStep>, AdapterError> {
    let tags = restore_tags(&after.marks.tags);
    let record = match tags.as_slice() {
        [tag] if after.floating => RestoreRecord::parse(tag),
        _ => None,
    };
    let mut steps = Vec::new();
    if let Some(record) = record {
        let origin = origin_of(origins, held.marks.monitor).ok_or_else(origin_lost)?;
        steps = moved_back(after, &record, origin);
    }
    steps.extend(untagged(after));
    Ok(steps)
}

fn moved_back(after: &ClientState, record: &RestoreRecord, origin: (i64, i64)) -> Vec<WindowStep> {
    let address = after.address;
    let at = (origin.0 + record.offset.0, origin.1 + record.offset.1);
    let (width, height) = record.size;
    let goals = vec![StepGoal::At(at.0, at.1), StepGoal::Size(width, height)];
    if goals.iter().all(|goal| goal.holds(after)) {
        return Vec::new();
    }
    let mut steps = Vec::new();
    if !StepGoal::Size(width, height).holds(after) {
        steps.push(WindowStep::new(
            resize_command(ConfigDialect::Lua, address, width, height),
            vec![StepGoal::Resized {
                wanted: record.size,
                from: after.size,
            }],
        ));
    }
    steps.push(WindowStep::new(
        move_command(ConfigDialect::Lua, address, at.0, at.1),
        goals,
    ));
    steps
}

fn untagged(state: &ClientState) -> Vec<WindowStep> {
    restore_tags(&state.marks.tags)
        .into_iter()
        .map(|tag| {
            WindowStep::new(
                tag_command(state.address, tag, false),
                vec![StepGoal::Untagged(tag.to_owned())],
            )
        })
        .collect()
}

fn origin_lost() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The window was restored, but its monitor's place is no longer known, so it stays where Hyprland put it",
    )
    .with_suggestion("Run 'list-windows' to see the window as it is now, then move it")
    .with_details(json!({ "cause": "window_state_unreadable", "physical_delivery_started": true }))
    .with_disposition(DeliverySemantics::delivered_unverified())
}

#[cfg(test)]
#[path = "restore_geometry_tests.rs"]
mod tests;
