use agent_desktop_core::ErrorCode;

use super::disclosure::set_expanded;
use super::live_click_tests::{act, find, find_named, open, state};
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::state_bits::{EXPANDABLE, EXPANDED};

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn the_calculator_main_menu_expands_and_collapses_through_its_inner_toggle() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let menu = find_named(&session, PUSH_BUTTON, "Main Menu").expect("main menu button");
    let opened = act(&session, &menu, |target| set_expanded(target, true)).expect("expands");
    assert_eq!(opened.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &menu, EXPANDED), Some(true));
    let closed = act(&session, &menu, |target| set_expanded(target, false)).expect("collapses");
    assert_eq!(closed.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &menu, EXPANDED), Some(false));
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn collapsing_a_collapsed_menu_button_writes_nothing() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let menu = find_named(&session, PUSH_BUTTON, "Main Menu").expect("main menu button");
    let result = act(&session, &menu, |target| set_expanded(target, false)).expect("satisfied");
    assert_eq!(
        result.disposition(),
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn a_digit_button_is_a_leaf_and_is_never_expanded() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    let error = act(&session, &digit, |target| set_expanded(target, true)).expect_err("a leaf");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
#[ignore = "live: needs the AT-SPI session, Files (nautilus) with a folder tree and the write engine"]
fn a_files_tree_row_expands_and_collapses_in_both_directions() {
    let session = open("nautilus", &["--new-window", "/usr"]).expect("files opens");
    let Ok(row) = find(&session, |view| {
        view.state(EXPANDABLE) == Some(true) && view.state(EXPANDED) == Some(false)
    }) else {
        eprintln!("no collapsed expandable tree row in this Files layout; absence stated");
        return;
    };
    let opened = act(&session, &row, |target| set_expanded(target, true)).expect("expands");
    assert_eq!(opened.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &row, EXPANDED), Some(true));
    let closed = act(&session, &row, |target| set_expanded(target, false)).expect("collapses");
    assert_eq!(closed.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &row, EXPANDED), Some(false));
}
