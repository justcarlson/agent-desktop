use super::*;
use crate::input::mouse::wheel_plan::{
    BUTTON_WHEEL_DOWN, BUTTON_WHEEL_LEFT, BUTTON_WHEEL_RIGHT, BUTTON_WHEEL_UP,
};

#[test]
fn wheel_codes_map_to_the_measured_axis_signs() {
    assert_eq!(wheel_of(BUTTON_WHEEL_UP), Some((WheelAxis::Vertical, -1)));
    assert_eq!(wheel_of(BUTTON_WHEEL_DOWN), Some((WheelAxis::Vertical, 1)));
    assert_eq!(
        wheel_of(BUTTON_WHEEL_LEFT),
        Some((WheelAxis::Horizontal, 1))
    );
    assert_eq!(
        wheel_of(BUTTON_WHEEL_RIGHT),
        Some((WheelAxis::Horizontal, -1))
    );
    assert_eq!(wheel_of(1), None);
}

#[test]
fn pointer_buttons_map_to_linux_codes() {
    assert_eq!(linux_button(1), Some(0x110));
    assert_eq!(linux_button(2), Some(0x112));
    assert_eq!(linux_button(3), Some(0x111));
    assert_eq!(linux_button(4), None);
    assert_eq!(linux_button(8), None);
}
