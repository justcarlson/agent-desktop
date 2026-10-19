use agent_desktop_core::{Action, ActionResult, AdapterError, ErrorCode};

use super::live_click_tests::{act, find_named, open, state};
use super::toggle_state::toggle;
use crate::tree::role_numbers::{CHECK_BOX, PUSH_BUTTON};
use crate::tree::state_bits::CHECKED;

fn check(target: &super::context::Target<'_>) -> Result<ActionResult, AdapterError> {
    toggle(target, Some(true))
}

#[test]
#[ignore = "live: needs the AT-SPI session, GNOME Settings (Multitasking) and the write engine"]
fn settings_hot_corner_checks_and_unchecks_with_read_back() {
    let session = open("gnome-control-center", &["multitasking"]).expect("settings opens");
    let corner = find_named(&session, CHECK_BOX, "Hot Corner").expect("Hot Corner switch");
    let start = state(&session, &corner, CHECKED).expect("checked state reads");
    let flipped =
        act(&session, &corner, |target| toggle(target, Some(!start))).expect("first write");
    assert_eq!(flipped.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &corner, CHECKED), Some(!start));
    act(&session, &corner, |target| toggle(target, Some(start))).expect("restore");
    assert_eq!(state(&session, &corner, CHECKED), Some(start));
}

#[test]
#[ignore = "live: needs the AT-SPI session, GNOME Settings (Multitasking) and the write engine"]
fn checking_a_checked_switch_writes_nothing() {
    let session = open("gnome-control-center", &["multitasking"]).expect("settings opens");
    let corner = find_named(&session, CHECK_BOX, "Hot Corner").expect("Hot Corner switch");
    if state(&session, &corner, CHECKED) != Some(true) {
        act(&session, &corner, check).expect("bring it to checked");
    }
    let result = act(&session, &corner, check).expect("satisfied");
    assert_eq!(
        result.disposition(),
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn toggle_on_a_push_button_is_not_supported() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    for action in [Action::Toggle, Action::Check, Action::Uncheck] {
        let desired = match action {
            Action::Check => Some(true),
            Action::Uncheck => Some(false),
            _ => None,
        };
        let error = act(&session, &digit, |target| toggle(target, desired))
            .expect_err("a push button cannot carry checked");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
    }
}
