#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline, KeyCombo};

use super::session;
use crate::input::key_plan::plan_combo;
use crate::input::step::Delivery;
use crate::input::text_plan::plan_text;

pub(crate) fn press(
    display: &str,
    combo: &KeyCombo,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let map = session::live_keymap(display, deadline)?;
    let plan = plan_combo(&map, combo)?;
    session::run(display, vec![plan], gate, deadline)
}

pub(crate) fn type_text(
    display: &str,
    text: &str,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let map = session::live_keymap(display, deadline)?;
    let plans = plan_text(&map, text)?;
    session::run(display, plans, gate, deadline)
}

#[cfg(test)]
#[path = "keyboard_send_tests.rs"]
mod tests;
