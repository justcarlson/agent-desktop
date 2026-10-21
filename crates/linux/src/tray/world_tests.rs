use agent_desktop_core::ErrorCode;

use super::*;

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn the_one_panel_node_with_the_title_is_chosen() {
    assert_eq!(
        choose_panel_node(&names(&["A", "Dropbox", "B"]), "Dropbox").ok(),
        Some(1)
    );
}

#[test]
fn a_repeated_title_is_ambiguous_and_never_guessed_by_position() {
    let error = choose_panel_node(&names(&["Twin", "Twin"]), "Twin").expect_err("ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn a_title_with_no_panel_node_is_not_found() {
    let error = choose_panel_node(&names(&["A"]), "B").expect_err("missing");
    assert_eq!(error.code, ErrorCode::ElementNotFound);
}

#[test]
fn two_registered_items_with_one_title_are_ambiguous_even_with_one_panel_node() {
    let error =
        ensure_title_is_single(&names(&["Twin", "Twin", "Other"]), "Twin").expect_err("ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn a_title_registered_once_is_single() {
    assert!(ensure_title_is_single(&names(&["Twin", "Other"]), "Twin").is_ok());
}
