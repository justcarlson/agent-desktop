use std::collections::BTreeMap;

use agent_desktop_core::NodeDescriptor;

use super::descriptors as describe_with;
use crate::tree::name_evidence::name_fields as resolve_names;
use crate::tree::placeholder::read_placeholder;
use crate::tree::role_numbers::{CHECK_BOX, ENTRY, PUSH_BUTTON, TEXT};
use crate::tree::test_support::attributes;
use crate::tree::{Field, LabelOutcome, NodeProperties};

fn descriptors(
    role: &Field<u32>,
    role_name: &Field<String>,
    properties: &NodeProperties,
    attributes: &Field<BTreeMap<String, String>>,
    in_web_document: bool,
) -> NodeDescriptor {
    let placeholder = read_placeholder(properties, role, attributes);
    describe_with(role, role_name, &placeholder, attributes, in_web_document)
}

fn name_fields(
    properties: &NodeProperties,
    role: &Field<u32>,
    attributes: &Field<BTreeMap<String, String>>,
    label: &LabelOutcome,
) -> (
    agent_desktop_core::LocatorField<String>,
    agent_desktop_core::LocatorField<String>,
) {
    let placeholder = read_placeholder(properties, role, attributes);
    resolve_names(properties, role, &placeholder, label)
}

fn properties(name: &str, description: &str) -> NodeProperties {
    NodeProperties {
        name: Field::Present(name.to_owned()),
        description: Field::Present(description.to_owned()),
        child_count: Field::Present(0),
        accessible_id: Field::Absent,
    }
}

fn describe(
    role: u32,
    role_name: &str,
    attrs: &Field<BTreeMap<String, String>>,
    in_web_document: bool,
) -> NodeDescriptor {
    descriptors(
        &Field::Present(role),
        &Field::Present(role_name.to_owned()),
        &properties("", ""),
        attrs,
        in_web_document,
    )
}

#[test]
fn class_is_split_on_whitespace_only_inside_a_web_document() {
    let attrs = attributes(&[("class", "btn  primary")]);
    assert_eq!(
        describe(PUSH_BUTTON, "push button", &attrs, true).dom_classes,
        ["btn", "primary"]
    );
    assert!(
        describe(PUSH_BUTTON, "push button", &attrs, false)
            .dom_classes
            .is_empty()
    );
}

#[test]
fn subrole_is_the_first_xml_role_and_the_one_gtk_pair() {
    let web = attributes(&[("xml-roles", "navigation banner")]);
    assert_eq!(
        describe(110, "landmark", &web, true).subrole.as_deref(),
        Some("navigation")
    );
    let gtk = attributes(&[("toolkit", "GTK")]);
    assert_eq!(
        describe(CHECK_BOX, "switch", &gtk, false)
            .subrole
            .as_deref(),
        Some("switch")
    );
    assert_eq!(describe(CHECK_BOX, "checkbox", &gtk, false).subrole, None);
    assert_eq!(describe(PUSH_BUTTON, "switch", &gtk, false).subrole, None);
    assert_eq!(describe(43, "generic", &gtk, false).subrole, None);
    let gtk3 = attributes(&[("toolkit", "gtk")]);
    assert_eq!(describe(CHECK_BOX, "switch", &gtk3, false).subrole, None);
    let qt = attributes(&[]);
    assert_eq!(describe(CHECK_BOX, "switch", &qt, false).subrole, None);
}

#[test]
fn role_description_is_the_author_attribute() {
    let attrs = attributes(&[("roledescription", "editor")]);
    assert_eq!(
        describe(ENTRY, "entry", &attrs, true)
            .role_description
            .as_deref(),
        Some("editor")
    );
    assert_eq!(
        describe(ENTRY, "entry", &attributes(&[]), true).role_description,
        None
    );
}

#[test]
fn the_descriptor_placeholder_equals_the_name_evidence_placeholder() {
    let cases = [
        (
            ENTRY,
            attributes(&[("placeholder", "Describe"), ("tag", "div")]),
            properties("", ""),
        ),
        (
            ENTRY,
            attributes(&[("placeholder-text", "Search"), ("tag", "input")]),
            properties("", ""),
        ),
        (TEXT, attributes(&[]), properties("", "Type name")),
    ];
    for (role, attrs, props) in cases {
        let role_field = Field::Present(role);
        let descriptor = descriptors(&role_field, &Field::Absent, &props, &attrs, true);
        let (name, _) = name_fields(&props, &role_field, &attrs, &LabelOutcome::Unlabelled);
        let placeholder = descriptor.placeholder.expect("a placeholder is read");
        assert_eq!(name.known(), Some(&placeholder));
    }
    let gtk = attributes(&[("toolkit", "GTK")]);
    let descriptor = descriptors(
        &Field::Present(TEXT),
        &Field::Absent,
        &properties("", "Tooltip"),
        &gtk,
        false,
    );
    assert_eq!(descriptor.placeholder, None);
}
