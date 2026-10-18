use super::needs_label;
use crate::tree::Field;
use crate::tree::role_numbers::{PANEL, PUSH_BUTTON};
use crate::tree::test_support::read_of;

#[test]
fn a_blank_named_button_needs_its_label_read() {
    assert!(needs_label(&read_of(PUSH_BUTTON, "", &["click"]).node));
}

#[test]
fn a_named_button_does_not() {
    assert!(!needs_label(&read_of(PUSH_BUTTON, "Save", &["click"]).node));
}

#[test]
fn a_blank_container_costs_no_relation_read() {
    assert!(!needs_label(&read_of(PANEL, "", &[]).node));
}

#[test]
fn an_unreadable_name_or_role_is_not_read_blindly() {
    let mut failed_name = read_of(PUSH_BUTTON, "", &["click"]).node;
    failed_name.properties.name = Field::Failed("x".to_owned());
    assert!(!needs_label(&failed_name));
    let mut failed_role = read_of(PUSH_BUTTON, "", &["click"]).node;
    failed_role.traits.role = Field::Failed("x".to_owned());
    assert!(!needs_label(&failed_role));
}
