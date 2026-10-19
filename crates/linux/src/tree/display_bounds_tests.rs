use super::*;

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

const SCREEN: Extents = Extents {
    x: 0,
    y: 0,
    width: 1280,
    height: 800,
};

fn off(node: Extents) -> bool {
    off_every_display(Some(node), Some(&[SCREEN]))
}

#[test]
fn a_rectangle_wholly_left_of_and_above_the_screen_is_off_it() {
    assert!(off(rect(-308, -156, 100, 40)));
    assert!(off(rect(-100, 10, 100, 40)));
    assert!(off(rect(1280, 10, 100, 40)));
    assert!(off(rect(10, 800, 100, 40)));
}

#[test]
fn a_rectangle_that_shares_any_area_with_the_screen_is_not_off_it() {
    assert!(!off(rect(1260, 10, 100, 40)));
    assert!(!off(rect(-50, -20, 100, 40)));
    assert!(!off(rect(10, 10, 100, 40)));
}

#[test]
fn a_rectangle_on_any_one_of_two_displays_is_not_off() {
    let right = rect(1280, 0, 800, 600);
    assert!(!off_every_display(
        Some(rect(1500, 10, 50, 20)),
        Some(&[SCREEN, right])
    ));
    assert!(off_every_display(
        Some(rect(2200, 10, 50, 20)),
        Some(&[SCREEN, right])
    ));
}

#[test]
fn nothing_read_is_not_evidence_of_being_off() {
    assert!(!off_every_display(None, Some(&[SCREEN])));
    assert!(!off_every_display(Some(rect(-500, 0, 10, 10)), None));
    assert!(!off_every_display(Some(rect(-500, 0, 10, 10)), Some(&[])));
    assert!(!off(rect(-500, 0, 0, 10)));
    assert!(!off(rect(i32::MIN, i32::MIN, 10, 10)));
}
