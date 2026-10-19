use super::{is_compositing, selection_name};

#[test]
fn an_owner_of_zero_is_not_compositing_and_any_other_owner_is() {
    assert!(!is_compositing(0));
    assert!(is_compositing(0x0040_0001));
}

#[test]
fn the_selection_is_named_after_the_screen() {
    assert_eq!(selection_name(0), "_NET_WM_CM_S0");
    assert_eq!(selection_name(2), "_NET_WM_CM_S2");
}
