use std::time::Duration;

use super::keymap::KeymapEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    Key { code: u8, down: bool },
    Button { code: u8, down: bool },
    MoveTo { x: i32, y: i32 },
    Pause(Duration),
    Keymap(KeymapEdit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) steps: Vec<Step>,
    pub(crate) release_origin: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delivery {
    pub(crate) delivered_events: u32,
    pub(crate) mechanism: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeysymStep {
    pub(crate) keysym: u32,
    pub(crate) down: bool,
}

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
