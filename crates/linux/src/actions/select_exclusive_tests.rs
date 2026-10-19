use super::*;

#[test]
fn only_a_second_selected_row_needs_the_selection_replaced() {
    assert!(!needs_replacement(None));
    assert!(!needs_replacement(Some(0)));
    assert!(!needs_replacement(Some(1)));
    assert!(needs_replacement(Some(2)));
    assert!(needs_replacement(Some(5)));
}

#[test]
fn a_selection_with_extra_rows_is_never_verified() {
    assert_eq!(exclusive_verdict(Some(1)), Some(true));
    assert_eq!(exclusive_verdict(Some(2)), None);
    assert_eq!(exclusive_verdict(Some(0)), None);
}

#[test]
fn an_unread_count_leaves_the_item_state_to_decide() {
    assert_eq!(exclusive_verdict(None), Some(true));
}
