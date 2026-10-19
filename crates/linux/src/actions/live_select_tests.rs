use agent_desktop_core::ErrorCode;

use super::live_click_tests::{act, find, find_named, open};
use super::select::select;
use crate::tree::role_numbers::{LIST, PUSH_BUTTON};

#[test]
#[ignore = "live: needs the AT-SPI session, GNOME Settings and the write engine"]
fn a_settings_sidebar_row_that_ignores_the_selection_is_a_write_ignored_failure() {
    let session = open("gnome-control-center", &[]).expect("settings opens");
    let list = find(&session, |view| {
        view.role() == Some(LIST)
            && view
                .interfaces()
                .is_some_and(|names| names.iter().any(|name| name == "org.a11y.atspi.Selection"))
    })
    .expect("the sidebar list");
    let refused = act(&session, &list, |target| select(target, "Sound"))
        .expect_err("a navigation row does not change on SelectChild");
    assert_eq!(refused.code, ErrorCode::ActionFailed);
    assert_eq!(refused.details.expect("details")["cause"], "write_ignored");
}

#[test]
#[ignore = "live: needs the AT-SPI session, GNOME Settings and the write engine"]
fn a_value_with_no_match_in_the_sidebar_is_not_found_and_writes_nothing() {
    let session = open("gnome-control-center", &[]).expect("settings opens");
    let list = find(&session, |view| view.role() == Some(LIST)).expect("a list");
    let error = act(&session, &list, |target| {
        select(target, "No Such Panel 7f3a")
    })
    .expect_err("no match");
    assert!(matches!(
        error.code,
        ErrorCode::ElementNotFound | ErrorCode::ActionNotSupported
    ));
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn a_button_with_no_selection_route_is_not_supported() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    let error = act(&session, &digit, |target| select(target, "7")).expect_err("no route");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}
