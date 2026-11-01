use super::wheel_axis::WheelAxis;

/// The wheel event for an X11 wheel button code of the shared plans: 4 up,
/// 5 down, 6 left, 7 right. Hyprland's vertical axis is positive down.
/// Its horizontal axis, as GTK3 reports it, is positive left.
pub(crate) fn wheel_of(code: u8) -> Option<(WheelAxis, i32)> {
    let _ = code;
    None
}

/// The Linux input code for an X11 pointer button code of the shared
/// plans: 1 left, 2 middle, 3 right.
pub(crate) fn linux_button(code: u8) -> Option<u32> {
    let _ = code;
    None
}

#[cfg(test)]
#[path = "step_event_tests.rs"]
mod tests;
