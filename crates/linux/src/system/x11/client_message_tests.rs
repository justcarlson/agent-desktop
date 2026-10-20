use super::*;

const X: u32 = 1 << 8;
const Y: u32 = 1 << 9;
const WIDTH: u32 = 1 << 10;
const HEIGHT: u32 = 1 << 11;
const GRAVITY_MASK: u32 = 0xFF;
const SOURCE_MASK: u32 = 0xF << 12;

fn field_flags(flags: u32) -> u32 {
    flags & (X | Y | WIDTH | HEIGHT)
}

#[test]
fn a_move_flags_x_and_y_only() {
    let data = move_resize(Some(40), Some(-30), None, None);
    assert_eq!(field_flags(data[0]), X | Y);
    assert_eq!(data[1], 40);
    assert_eq!(data[2], u32::from_ne_bytes((-30_i32).to_ne_bytes()));
    assert_eq!((data[3], data[4]), (0, 0));
}

#[test]
fn a_resize_flags_width_and_height_only() {
    let data = move_resize(None, None, Some(562), Some(693));
    assert_eq!(field_flags(data[0]), WIDTH | HEIGHT);
    assert_eq!(data, [data[0], 0, 0, 562, 693]);
}

#[test]
fn a_request_that_places_the_window_carries_static_gravity() {
    for data in [
        move_resize(Some(1), None, None, None),
        move_resize(None, Some(1), Some(5), Some(5)),
    ] {
        assert_eq!(data[0] & GRAVITY_MASK, 10);
    }
}

#[test]
fn a_request_that_only_resizes_carries_the_gravity_of_the_window_itself() {
    let data = move_resize(None, None, Some(5), Some(5));
    assert_eq!(data[0] & GRAVITY_MASK, 0);
}

#[test]
fn the_request_carries_the_source_indication() {
    let data = move_resize(Some(1), None, None, None);
    assert_eq!((data[0] & SOURCE_MASK) >> 12, SOURCE_TOOL);
}

#[test]
fn a_request_that_changes_nothing_flags_nothing() {
    assert_eq!(field_flags(move_resize(None, None, None, None)[0]), 0);
}

#[test]
fn maximize_adds_both_atoms_in_one_message() {
    assert_eq!(maximize(true, 71, 72), [1, 71, 72, SOURCE_TOOL, 0]);
}

#[test]
fn restore_removes_both_atoms_in_one_message() {
    assert_eq!(maximize(false, 71, 72), [0, 71, 72, SOURCE_TOOL, 0]);
}

#[test]
fn iconify_asks_for_the_iconic_state() {
    assert_eq!(iconify(), [3, 0, 0, 0, 0]);
}

#[test]
fn close_carries_the_timestamp_and_the_source() {
    assert_eq!(close(123_456), [123_456, SOURCE_TOOL, 0, 0, 0]);
}

#[test]
fn activation_names_the_tool_source_and_timestamp_zero() {
    assert_eq!(activate(), [SOURCE_TOOL, 0, 0, 0, 0]);
}
