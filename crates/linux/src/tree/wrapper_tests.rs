use agent_desktop_core::LocatorField;

use super::{classify, is_wrapper_shape};
use crate::tree::role_numbers::{
    DOCUMENT_FRAME, DOCUMENT_WEB, FILLER, PANEL, PUSH_BUTTON, SECTION,
};
use crate::tree::value_read::ValueRead;
use crate::tree::{AtspiElement, Field, NodeProperties, ObservedRead, RawNode};

fn properties(name: Field<String>, accessible_id: Field<String>) -> NodeProperties {
    NodeProperties {
        name,
        description: Field::Present(String::new()),
        child_count: Field::Present(1),
        accessible_id,
    }
}

fn blank() -> NodeProperties {
    properties(Field::Present(String::new()), Field::Present(String::new()))
}

fn caps(tokens: &[&str]) -> LocatorField<Vec<String>> {
    LocatorField::Known(tokens.iter().map(|token| (*token).to_owned()).collect())
}

fn shape(role: u32, props: &NodeProperties, tokens: &LocatorField<Vec<String>>) -> bool {
    is_wrapper_shape(&Field::Present(role), props, &ValueRead::absent(), tokens)
}

#[test]
fn a_blank_panel_section_or_filler_with_no_action_is_a_wrapper() {
    for role in [PANEL, SECTION, FILLER] {
        assert!(shape(role, &blank(), &caps(&[])), "role {role}");
    }
}

#[test]
fn only_the_ubiquitous_affordances_leave_a_node_a_wrapper() {
    assert!(shape(
        PANEL,
        &blank(),
        &caps(&["RightClick", "SetFocus", "ScrollTo"])
    ));
}

#[test]
fn a_node_with_a_click_capability_is_not_a_wrapper() {
    assert!(!shape(PANEL, &blank(), &caps(&["RightClick", "Click"])));
}

#[test]
fn a_named_panel_is_not_a_wrapper() {
    let named = properties(Field::Present("Sidebar".to_owned()), Field::Absent);
    assert!(!shape(PANEL, &named, &caps(&[])));
}

#[test]
fn a_panel_with_a_dom_style_identifier_is_not_a_wrapper() {
    let identified = properties(Field::Absent, Field::Present("main-content".to_owned()));
    assert!(!shape(PANEL, &identified, &caps(&[])));
}

#[test]
fn a_panel_with_a_value_is_not_a_wrapper() {
    let valued = ValueRead {
        value: Field::Present("3".to_owned()),
        truncated: false,
    };
    assert!(!is_wrapper_shape(
        &Field::Present(PANEL),
        &blank(),
        &valued,
        &caps(&[])
    ));
}

#[test]
fn a_failed_name_read_never_makes_a_wrapper() {
    let unknown_name = properties(Field::Failed("timed out".to_owned()), Field::Absent);
    assert!(!shape(PANEL, &unknown_name, &caps(&[])));
}

#[test]
fn an_unknown_capability_list_never_makes_a_wrapper() {
    assert!(!shape(PANEL, &blank(), &LocatorField::Unknown));
}

#[test]
fn a_failed_role_read_never_makes_a_wrapper() {
    assert!(!is_wrapper_shape(
        &Field::Failed("x".to_owned()),
        &blank(),
        &ValueRead::absent(),
        &caps(&[])
    ));
}

#[test]
fn a_button_is_never_a_wrapper_however_blank() {
    assert!(!shape(PUSH_BUTTON, &blank(), &caps(&[])));
}

fn read_with(role: u32, actions: Field<Vec<String>>) -> ObservedRead {
    let mut node = RawNode::unreadable(AtspiElement::new(":1.1", "/p", 1), "unused");
    node.traits.role = Field::Present(role);
    node.traits.states = Field::Present(vec![0, 0]);
    node.properties = blank();
    node.actions = actions;
    ObservedRead::plain(node)
}

#[test]
fn classify_reads_the_wrapper_shape_from_the_advertised_actions() {
    let context_menu_only = read_with(PANEL, Field::Present(vec!["showContextMenu".to_owned()]));
    assert!(classify(&context_menu_only).wrapper);
    let default_action = read_with(PANEL, Field::Present(vec!["doDefault".to_owned()]));
    assert!(!classify(&default_action).wrapper);
    let unreadable_actions = read_with(PANEL, Field::Failed("x".to_owned()));
    assert!(!classify(&unreadable_actions).wrapper);
}

#[test]
fn classify_marks_web_documents() {
    assert!(classify(&read_with(DOCUMENT_WEB, Field::Absent)).web_document);
    assert!(classify(&read_with(DOCUMENT_FRAME, Field::Absent)).web_document);
    assert!(!classify(&read_with(PANEL, Field::Absent)).web_document);
}
