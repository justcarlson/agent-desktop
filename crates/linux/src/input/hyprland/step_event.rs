use super::wheel_axis::WheelAxis;
use crate::input::mouse::wheel_plan::{
    BUTTON_WHEEL_DOWN, BUTTON_WHEEL_LEFT, BUTTON_WHEEL_RIGHT, BUTTON_WHEEL_UP,
};

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

/// The wheel event for an X11 wheel button code of the shared plans: 4 up,
/// 5 down, 6 left, 7 right. `wl_pointer` axes are positive down and right.
pub(crate) fn wheel_of(code: u8) -> Option<(WheelAxis, i32)> {
    match code {
        BUTTON_WHEEL_UP => Some((WheelAxis::Vertical, -1)),
        BUTTON_WHEEL_DOWN => Some((WheelAxis::Vertical, 1)),
        BUTTON_WHEEL_LEFT => Some((WheelAxis::Horizontal, -1)),
        BUTTON_WHEEL_RIGHT => Some((WheelAxis::Horizontal, 1)),
        _ => None,
    }
}

/// The Linux input code for an X11 pointer button code of the shared
/// plans: 1 left, 2 middle, 3 right.
pub(crate) fn linux_button(code: u8) -> Option<u32> {
    match code {
        1 => Some(BTN_LEFT),
        2 => Some(BTN_MIDDLE),
        3 => Some(BTN_RIGHT),
        _ => None,
    }
}

#[cfg(test)]
#[path = "step_event_tests.rs"]
mod tests;
