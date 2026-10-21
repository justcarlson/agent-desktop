use super::{PopupWindow, popup_candidate, unique_popup};
use crate::tree::Extents;

fn bounds(width: i32, height: i32) -> Extents {
    Extents {
        x: 0,
        y: 0,
        width,
        height,
    }
}

#[test]
fn a_popup_origin_requires_the_owning_process_and_its_override_redirect_window() {
    assert!(popup_candidate(
        42,
        (280, 190),
        Some(42),
        bounds(280, 190),
        true
    ));
    assert!(!popup_candidate(
        42,
        (280, 190),
        Some(42),
        bounds(280, 190),
        false
    ));
    assert!(!popup_candidate(
        42,
        (280, 190),
        Some(43),
        bounds(280, 190),
        true
    ));
    assert!(!popup_candidate(
        42,
        (280, 190),
        Some(42),
        bounds(281, 190),
        true
    ));
}

#[test]
fn a_popup_is_aimable_only_when_its_override_redirect_origin_is_unique() {
    let one = PopupWindow {
        id: 7,
        bounds: bounds(280, 190),
    };
    let other = PopupWindow {
        id: 8,
        bounds: bounds(280, 190),
    };
    assert_eq!(unique_popup(&[one]), Some(one));
    assert_eq!(unique_popup(&[]), None);
    assert_eq!(unique_popup(&[one, other]), None);
}
