#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the planner is not called until the transport is wired"
    )
)]

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, KeyCombo};

use super::key_plan::plan_combo;
use super::keymap::Keymap;
use super::step::Plan;
use super::text_plan::plan_text;

pub(crate) fn ensure_time(deadline: Deadline) -> Result<(), AdapterError> {
    if deadline.is_expired() {
        return Err(deadline
            .timeout_error()
            .with_details(serde_json::json!({ "physical_delivery_started": false }))
            .with_disposition(DeliverySemantics::not_delivered()));
    }
    Ok(())
}

pub(crate) fn combo_plan(
    map: &Keymap,
    combo: &KeyCombo,
    deadline: Deadline,
) -> Result<Plan, AdapterError> {
    ensure_time(deadline)?;
    plan_combo(map, combo)
}

pub(crate) fn text_plans(
    map: &Keymap,
    text: &str,
    deadline: Deadline,
) -> Result<Vec<Plan>, AdapterError> {
    ensure_time(deadline)?;
    plan_text(map, text)
}

#[cfg(test)]
#[path = "keyboard_tests.rs"]
mod tests;
