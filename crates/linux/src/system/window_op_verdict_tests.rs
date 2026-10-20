use super::*;
use crate::tree::Extents;

fn facts(x: i32, y: i32, width: i32, height: i32) -> WindowFacts {
    WindowFacts {
        client: Extents {
            x,
            y,
            width,
            height,
        },
        hidden: false,
        maximized_vertical: false,
        maximized_horizontal: false,
    }
}

fn resize(width: u32, height: u32) -> WindowWrite {
    WindowWrite::MoveResize {
        x: None,
        y: None,
        width: Some(width),
        height: Some(height),
    }
}

fn move_to(x: i32, y: i32) -> WindowWrite {
    WindowWrite::MoveResize {
        x: Some(x),
        y: Some(y),
        width: None,
        height: None,
    }
}

#[test]
fn a_resize_read_back_exactly_is_satisfied() {
    let verdict = judge(&resize(562, 693), &facts(10, 10, 562, 693), true);
    assert_eq!(verdict, Verdict::Satisfied);
}

#[test]
fn a_resize_within_the_tolerance_is_satisfied() {
    let near = facts(10, 10, 562 + POSITION_TOLERANCE, 693 - POSITION_TOLERANCE);
    assert_eq!(judge(&resize(562, 693), &near, true), Verdict::Satisfied);
}

#[test]
fn a_resize_just_outside_the_tolerance_is_not_satisfied() {
    let far = facts(10, 10, 562 + POSITION_TOLERANCE + 1, 693);
    assert_eq!(judge(&resize(562, 693), &far, true), Verdict::Mismatch);
}

#[test]
fn a_mid_transition_read_is_pending_before_the_limit_and_a_mismatch_at_it() {
    let moving = facts(10, 10, 400, 300);
    assert_eq!(judge(&resize(562, 693), &moving, false), Verdict::Pending);
    assert_eq!(judge(&resize(562, 693), &moving, true), Verdict::Mismatch);
}

#[test]
fn a_satisfied_read_is_satisfied_whether_or_not_the_limit_passed() {
    let done = facts(0, 0, 562, 693);
    assert_eq!(judge(&resize(562, 693), &done, false), Verdict::Satisfied);
}

#[test]
fn a_move_is_judged_on_x_and_y_only() {
    let elsewhere_in_size = facts(100, 200, 1, 1);
    assert_eq!(
        judge(&move_to(100, 200), &elsewhere_in_size, true),
        Verdict::Satisfied
    );
    assert_eq!(
        judge(&move_to(100, 200), &facts(100, 260, 1, 1), true),
        Verdict::Mismatch
    );
}

#[test]
fn a_resize_is_judged_on_width_and_height_only() {
    let moved = facts(900, 900, 562, 693);
    assert_eq!(judge(&resize(562, 693), &moved, true), Verdict::Satisfied);
}

#[test]
fn a_negative_coordinate_is_compared_as_a_signed_value() {
    assert_eq!(
        judge(&move_to(-40, -10), &facts(-40, -10, 5, 5), true),
        Verdict::Satisfied
    );
    assert_eq!(
        judge(&move_to(-40, -10), &facts(40, 10, 5, 5), true),
        Verdict::Mismatch
    );
}

#[test]
fn maximize_needs_both_atoms() {
    let mut half = facts(0, 0, 1, 1);
    half.maximized_vertical = true;
    assert_eq!(
        judge(&WindowWrite::SetMaximized(true), &half, true),
        Verdict::Mismatch
    );
    half.maximized_horizontal = true;
    assert_eq!(
        judge(&WindowWrite::SetMaximized(true), &half, true),
        Verdict::Satisfied
    );
}

#[test]
fn minimize_needs_the_hidden_flag() {
    let mut window = facts(0, 0, 1, 1);
    assert_eq!(
        judge(&WindowWrite::Iconify, &window, false),
        Verdict::Pending
    );
    window.hidden = true;
    assert_eq!(
        judge(&WindowWrite::Iconify, &window, false),
        Verdict::Satisfied
    );
}

#[test]
fn restore_from_maximized_needs_neither_maximized_atom() {
    let mut window = facts(0, 0, 1, 1);
    window.maximized_horizontal = true;
    assert_eq!(
        judge(&WindowWrite::SetMaximized(false), &window, true),
        Verdict::Mismatch
    );
    window.maximized_horizontal = false;
    assert_eq!(
        judge(&WindowWrite::SetMaximized(false), &window, true),
        Verdict::Satisfied
    );
}
