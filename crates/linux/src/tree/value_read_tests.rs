use super::{ValueSource, number_text, truncate_to_boundary, value_source};
use crate::tree::role_numbers::{
    COMBO_BOX, ENTRY, PANEL, PASSWORD_TEXT, PROGRESS_BAR, PUSH_BUTTON, SLIDER, TEXT,
};
use crate::tree::state_bits::EDITABLE;
use crate::tree::test_support::words;
use crate::tree::{AtspiElement, Field, RawNode};

fn node(role: u32, interfaces: &[&str], state_bits: &[u32]) -> RawNode {
    let mut node = RawNode::unreadable(AtspiElement::new(":1.1", "/p", 1), "unused");
    node.traits.role = Field::Present(role);
    node.traits.states = Field::Present(words(state_bits));
    node.traits.interfaces = Field::Present(
        interfaces
            .iter()
            .map(|name| format!("org.a11y.atspi.{name}"))
            .collect(),
    );
    node
}

#[test]
fn a_slider_and_a_progress_bar_with_the_value_interface_read_a_number() {
    assert_eq!(
        value_source(&node(SLIDER, &["Value"], &[])),
        ValueSource::Numeric
    );
    assert_eq!(
        value_source(&node(PROGRESS_BAR, &["Value"], &[])),
        ValueSource::Numeric
    );
}

#[test]
fn a_slider_that_does_not_name_the_value_interface_is_never_read() {
    assert_eq!(value_source(&node(SLIDER, &[], &[])), ValueSource::None);
}

#[test]
fn an_entry_and_an_editable_text_node_read_text() {
    assert_eq!(
        value_source(&node(ENTRY, &["Text"], &[])),
        ValueSource::Text
    );
    assert_eq!(
        value_source(&node(TEXT, &["Text"], &[EDITABLE])),
        ValueSource::Text
    );
    assert_eq!(
        value_source(&node(COMBO_BOX, &["Text"], &[])),
        ValueSource::Text
    );
}

#[test]
fn read_only_text_is_content_and_carries_no_value() {
    assert_eq!(value_source(&node(TEXT, &["Text"], &[])), ValueSource::None);
}

#[test]
fn a_password_field_has_no_value_source() {
    assert_eq!(
        value_source(&node(PASSWORD_TEXT, &["Text"], &[EDITABLE])),
        ValueSource::None
    );
}

#[test]
fn a_role_without_a_value_is_never_read() {
    assert_eq!(
        value_source(&node(PUSH_BUTTON, &["Value", "Text"], &[])),
        ValueSource::None
    );
    assert_eq!(
        value_source(&node(PANEL, &["Value", "Text"], &[])),
        ValueSource::None
    );
}

#[test]
fn a_node_whose_role_read_failed_has_no_value_source() {
    let unreadable = RawNode::unreadable(AtspiElement::new(":1.1", "/p", 1), "failed");
    assert_eq!(value_source(&unreadable), ValueSource::None);
}

#[test]
fn text_longer_than_the_budget_is_cut_on_a_character_boundary() {
    let (kept, cut) = truncate_to_boundary("aé€😀".repeat(3), 10);
    assert!(cut);
    assert!(kept.len() <= 10);
    assert!(kept.starts_with("aé€"));
}

#[test]
fn text_within_the_budget_is_untouched() {
    let (kept, cut) = truncate_to_boundary("héllo".to_owned(), 64);
    assert_eq!((kept.as_str(), cut), ("héllo", false));
}

#[test]
fn a_budget_that_lands_inside_a_multibyte_character_backs_up_to_its_start() {
    let (kept, cut) = truncate_to_boundary("€€".to_owned(), 4);
    assert_eq!((kept.as_str(), cut), ("€", true));
    let (empty, cut) = truncate_to_boundary("€".to_owned(), 2);
    assert_eq!((empty.as_str(), cut), ("", true));
}

#[test]
fn numbers_print_without_a_trailing_fraction_and_non_finite_numbers_are_refused() {
    assert_eq!(number_text(50.0).as_deref(), Some("50"));
    assert_eq!(number_text(0.25).as_deref(), Some("0.25"));
    assert_eq!(number_text(f64::NAN), None);
    assert_eq!(number_text(f64::INFINITY), None);
}
