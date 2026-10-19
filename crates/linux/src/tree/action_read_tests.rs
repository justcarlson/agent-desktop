use super::{NodeView, corroboration_of};
use crate::system::NAME_HAS_NO_OWNER;
use crate::tree::absence::{Liveness, UNKNOWN_INTERFACE, UNKNOWN_OBJECT};
use crate::tree::node_evidence::{NodeEvidence, evidence_for};
use crate::tree::role_numbers::{PASSWORD_TEXT, PUSH_BUTTON, TOGGLE_BUTTON};
use crate::tree::state_bits::{CHECKED, EXPANDED, FOCUSED, SHOWING};
use crate::tree::test_support::{client_timeout, method_error, read_of, words};
use crate::tree::{Field, ObservedRead};

fn verdict(error: Option<&zbus::Error>) -> String {
    format!("{:?}", corroboration_of(error))
}

fn view_of(read: ObservedRead) -> NodeView {
    let evidence = evidence_for(&read, &read.node.element);
    NodeView::new(NodeEvidence {
        read,
        evidence,
        liveness: Liveness::Alive,
    })
}

fn with_bits(role: u32, bits: &[u32]) -> NodeView {
    let mut read = read_of(role, "Wrap", &["click"]);
    read.node.traits.states = Field::Present(words(bits));
    view_of(read)
}

#[test]
fn the_view_reports_what_the_one_read_holds() {
    let view = with_bits(PUSH_BUTTON, &[SHOWING]);
    assert_eq!(view.role(), Some(PUSH_BUTTON));
    assert_eq!(view.name(), Some("Wrap"));
    assert_eq!(view.action_names(), Some(&["click".to_owned()][..]));
    assert_eq!(view.state(SHOWING), Some(true));
    assert_eq!(view.state(CHECKED), Some(false));
    assert_eq!(view.child_count(), Some(0));
    assert!(!view.secure());
}

#[test]
fn a_failed_state_read_is_none_not_false() {
    let mut read = read_of(PUSH_BUTTON, "n", &[]);
    read.node.traits.states = Field::Failed("x".to_owned());
    let view = view_of(read);
    assert_eq!(view.state(CHECKED), None);
    assert_eq!(view.tokens(), None);
}

#[test]
fn the_action_names_are_none_only_when_the_read_failed() {
    let mut absent = read_of(PUSH_BUTTON, "n", &[]);
    absent.node.actions = Field::Absent;
    assert_eq!(view_of(absent).action_names(), Some(&[][..]));
    let mut failed = read_of(PUSH_BUTTON, "n", &[]);
    failed.node.actions = Field::Failed("x".to_owned());
    assert_eq!(view_of(failed).action_names(), None);
}

#[test]
fn a_secure_field_gives_no_value_whatever_was_read() {
    let mut read = read_of(PASSWORD_TEXT, "Password", &[]);
    read.value.value = Field::Present("hunter2".to_owned());
    let view = view_of(read);
    assert!(view.secure());
    assert_eq!(view.value(), None);
    assert_eq!(view.observation().changed_since(&view.observation()), None);
}

#[test]
fn an_ordinary_field_gives_its_value() {
    let mut read = read_of(PUSH_BUTTON, "n", &[]);
    read.value.value = Field::Present("42".to_owned());
    assert_eq!(view_of(read).value(), Some("42"));
}

#[test]
fn a_changed_checked_state_is_a_change() {
    let before = with_bits(TOGGLE_BUTTON, &[]).observation();
    let after = with_bits(TOGGLE_BUTTON, &[CHECKED]).observation();
    assert_eq!(after.changed_since(&before), Some(true));
}

#[test]
fn a_changed_expanded_state_is_a_change() {
    let before = with_bits(PUSH_BUTTON, &[]).observation();
    let after = with_bits(PUSH_BUTTON, &[EXPANDED]).observation();
    assert_eq!(after.changed_since(&before), Some(true));
}

#[test]
fn focus_alone_is_not_a_change() {
    let before = with_bits(PUSH_BUTTON, &[SHOWING]).observation();
    let after = with_bits(PUSH_BUTTON, &[SHOWING, FOCUSED]).observation();
    assert_eq!(after.changed_since(&before), Some(false));
}

#[test]
fn a_changed_value_or_name_or_child_count_is_a_change() {
    let base = read_of(PUSH_BUTTON, "n", &[]);
    let before = view_of(base.clone()).observation();
    let mut valued = base.clone();
    valued.value.value = Field::Present("1".to_owned());
    assert_eq!(
        view_of(valued).observation().changed_since(&before),
        Some(true)
    );
    let mut renamed = base.clone();
    renamed.node.properties.name = Field::Present("m".to_owned());
    assert_eq!(
        view_of(renamed).observation().changed_since(&before),
        Some(true)
    );
    let mut grown = base;
    grown.node.properties.child_count = Field::Present(2);
    assert_eq!(
        view_of(grown).observation().changed_since(&before),
        Some(true)
    );
}

#[test]
fn an_unread_part_leaves_no_change_undecided() {
    let before = with_bits(PUSH_BUTTON, &[]).observation();
    let mut read = read_of(PUSH_BUTTON, "Wrap", &["click"]);
    read.node.traits.states = Field::Failed("x".to_owned());
    let unread = view_of(read).observation();
    assert_eq!(unread.changed_since(&before), None);
}

#[test]
fn a_difference_in_a_read_part_decides_even_when_another_is_unread() {
    let before = with_bits(PUSH_BUTTON, &[]).observation();
    let mut read = read_of(PUSH_BUTTON, "Wrap", &["click"]);
    read.node.traits.states = Field::Failed("x".to_owned());
    read.node.properties.name = Field::Present("Other".to_owned());
    assert_eq!(
        view_of(read).observation().changed_since(&before),
        Some(true)
    );
}

#[test]
fn a_role_answer_is_alive_and_a_missing_object_or_application_is_dead() {
    assert_eq!(verdict(None), "Alive");
    for name in [UNKNOWN_OBJECT, UNKNOWN_INTERFACE, NAME_HAS_NO_OWNER] {
        assert_eq!(verdict(Some(&method_error(name))), "Dead", "{name}");
    }
}

#[test]
fn a_timeout_is_not_a_death_and_another_failure_is_not_a_reading() {
    assert_eq!(verdict(Some(&client_timeout())), "TimedOut");
    let other = method_error("org.freedesktop.DBus.Error.Failed");
    assert_eq!(verdict(Some(&other)), "NotRead");
}
