use agent_desktop_core::{Deadline, DisplayInfo};

use crate::input::delivery::Gate;

/// One Hyprland pointer run: the outputs points map onto, the caller's
/// gate before each plan, the focus gate before each button press and
/// wheel step, and the deadline.
pub(crate) struct PointerRun<'a> {
    pub(crate) displays: &'a [DisplayInfo],
    pub(crate) plan_gate: Gate<'a>,
    pub(crate) step_gate: Gate<'a>,
    pub(crate) deadline: Deadline,
}
