use agent_desktop_core::roles::INTERACTIVE_ROLES;
use agent_desktop_core::{LocatorField, Role};

use super::{resolve_role, role_of};
use crate::tree::role_table::{ROLE_COUNT, ROLE_TABLE};
use crate::tree::state_bits::{EDITABLE, HAS_POPUP, SENSITIVE};
use crate::tree::test_support::{attributes, words};
use crate::tree::{Field, NodeTraits};

const UNPRODUCED_INTERACTIVE_ROLES: [&str; 1] = ["colorwell"];

const NUMBERS_BY_ROLE: &[(&str, &[u32])] = &[
    ("window", &[23, 69, 89]),
    ("dialog", &[16, 19, 22, 36]),
    ("alert", &[2, 101]),
    ("application", &[75]),
    (
        "group",
        &[
            5, 9, 14, 17, 20, 21, 24, 25, 28, 30, 39, 46, 53, 68, 71, 72, 77, 78, 85, 99, 104, 105,
            106, 107, 128,
        ],
    ),
    ("statictext", &[1, 29, 81, 116, 119, 120, 125, 126, 127]),
    ("image", &[3, 26, 27, 80, 100]),
    ("button", &[43]),
    ("menubutton", &[129]),
    ("switch", &[62]),
    ("checkbox", &[7]),
    ("menuitem", &[8, 35, 45, 59]),
    ("combobox", &[11, 76]),
    ("datefield", &[12]),
    ("slider", &[15, 51]),
    ("incrementor", &[52]),
    ("dockitem", &[13]),
    ("column", &[10, 57]),
    ("cell", &[47, 56, 58]),
    ("row", &[90]),
    ("table", &[55]),
    ("outline", &[65, 66]),
    ("treeitem", &[91]),
    ("list", &[31, 121]),
    ("listbox", &[98]),
    ("option", &[32]),
    ("menu", &[33, 34, 41]),
    ("tab", &[37]),
    ("tablist", &[38]),
    ("textfield", &[40, 79]),
    ("document", &[60, 61, 82, 92, 93, 94, 96]),
    ("webarea", &[95]),
    ("page", &[84]),
    ("paragraph", &[73]),
    ("heading", &[83]),
    ("link", &[88]),
    ("form", &[87]),
    ("region", &[110]),
    ("separator", &[50]),
    ("scrollbar", &[48]),
    ("scrollarea", &[49]),
    ("toolbar", &[63]),
    ("tooltip", &[64]),
    ("status", &[54, 102]),
    ("progressbar", &[42]),
    ("levelindicator", &[103, 114]),
    ("ruler", &[74]),
    ("article", &[109]),
    ("definition", &[108, 123]),
    ("term", &[122]),
    ("note", &[97, 124]),
    ("log", &[111]),
    ("marquee", &[112]),
    ("math", &[113, 117, 118]),
    ("timer", &[115]),
    ("radiobutton", &[44]),
    ("unknown", &[0, 4, 6, 18, 67, 70, 86]),
];

fn states(bits: &[u32]) -> Field<Vec<u32>> {
    Field::Present(words(bits))
}

fn interfaces(names: &[&str]) -> Field<Vec<String>> {
    Field::Present(names.iter().map(|name| (*name).to_owned()).collect())
}

fn plain(number: u32) -> Role {
    role_of(
        number,
        &states(&[SENSITIVE]),
        &interfaces(&[]),
        &attributes(&[]),
    )
}

#[test]
fn every_role_number_maps_to_a_canonical_role_and_larger_ones_to_unknown() {
    assert_eq!(ROLE_TABLE.len(), ROLE_COUNT);
    for number in 0..ROLE_COUNT as u32 {
        let role = plain(number);
        assert!(Role::is_canonical(role.as_str()), "number {number}");
    }
    for number in [130, 131, 1000, u32::MAX] {
        assert_eq!(plain(number), Role::Unknown, "number {number}");
    }
}

#[test]
fn the_table_matches_the_independent_grouping_for_every_number() {
    let mut seen = [false; ROLE_COUNT];
    for (token, numbers) in NUMBERS_BY_ROLE {
        for number in *numbers {
            assert!(!seen[*number as usize], "number {number} is listed twice");
            seen[*number as usize] = true;
            assert_eq!(
                ROLE_TABLE[*number as usize],
                Role::from_token(token),
                "number {number} should map to {token}"
            );
        }
    }
    let unlisted: Vec<usize> = (0..ROLE_COUNT).filter(|number| !seen[*number]).collect();
    assert!(
        unlisted.is_empty(),
        "numbers with no expected role: {unlisted:?}"
    );
}

#[test]
fn every_interactive_role_is_produced_or_listed_as_unproduced() {
    let mut produced: Vec<&str> = ROLE_TABLE.iter().map(|role| role.as_str()).collect();
    produced.extend([Role::TextField, Role::MenuButton, Role::ListBox].map(Role::as_str));
    for interactive in INTERACTIVE_ROLES {
        let covered = produced.contains(interactive);
        let listed = UNPRODUCED_INTERACTIVE_ROLES.contains(interactive);
        assert!(
            covered != listed,
            "{interactive}: produced {covered}, listed {listed}"
        );
    }
}

#[test]
fn the_gtk4_window_and_the_panel_have_their_roles() {
    assert_eq!(plain(23), Role::Window);
    assert_eq!(plain(39), Role::Group);
}

#[test]
fn a_list_with_selection_is_a_listbox_otherwise_a_list() {
    let with = interfaces(&["org.a11y.atspi.Accessible", "org.a11y.atspi.Selection"]);
    let without = interfaces(&["org.a11y.atspi.Accessible"]);
    let failed = Field::Failed("gone".to_owned());
    let none = attributes(&[]);
    assert_eq!(role_of(31, &states(&[]), &with, &none), Role::ListBox);
    assert_eq!(role_of(31, &states(&[]), &without, &none), Role::List);
    assert_eq!(role_of(31, &states(&[]), &failed, &none), Role::List);
}

#[test]
fn text_is_a_textfield_only_when_editable() {
    let none = attributes(&[]);
    let none_if = interfaces(&[]);
    for number in [60, 61] {
        assert_eq!(
            role_of(number, &states(&[EDITABLE]), &none_if, &none),
            Role::TextField
        );
        assert_eq!(
            role_of(number, &states(&[SENSITIVE]), &none_if, &none),
            Role::Document
        );
        let failed = Field::Failed("gone".to_owned());
        assert_eq!(role_of(number, &failed, &none_if, &none), Role::Document);
    }
}

#[test]
fn a_push_button_with_a_popup_is_a_menubutton() {
    let none = attributes(&[]);
    let none_if = interfaces(&[]);
    assert_eq!(
        role_of(43, &states(&[HAS_POPUP]), &none_if, &none),
        Role::MenuButton
    );
    assert_eq!(
        role_of(43, &states(&[SENSITIVE]), &none_if, &none),
        Role::Button
    );
    let failed = Field::Failed("gone".to_owned());
    assert_eq!(role_of(43, &failed, &none_if, &none), Role::Button);
}

#[test]
fn a_landmark_takes_its_xml_role_when_listed_and_is_a_region_otherwise() {
    let none_if = interfaces(&[]);
    let landmark =
        |pairs: &[(&str, &str)]| role_of(110, &states(&[]), &none_if, &attributes(pairs));
    assert_eq!(landmark(&[("xml-roles", "navigation")]), Role::Navigation);
    assert_eq!(
        landmark(&[("xml-roles", "Contentinfo extra")]),
        Role::ContentInfo
    );
    assert_eq!(landmark(&[("xml-roles", "region")]), Role::Region);
    assert_eq!(landmark(&[]), Role::Region);
    let failed = Field::Failed("gone".to_owned());
    assert_eq!(role_of(110, &states(&[]), &none_if, &failed), Role::Region);
}

#[test]
fn a_failed_role_read_is_unknown_not_the_unknown_role() {
    let mut traits = NodeTraits::failed("gone");
    assert_eq!(
        resolve_role(&traits, &attributes(&[])),
        LocatorField::Unknown
    );
    traits.role = Field::Present(0);
    assert_eq!(
        resolve_role(&traits, &attributes(&[])),
        LocatorField::Known("unknown".to_owned())
    );
}

fn traits_of(number: u32, role_name: &str) -> NodeTraits {
    NodeTraits {
        role: Field::Present(number),
        role_name: Field::Present(role_name.to_owned()),
        states: states(&[SENSITIVE]),
        interfaces: interfaces(&[]),
    }
}

fn role_token(traits: &NodeTraits, pairs: &[(&str, &str)]) -> LocatorField<String> {
    resolve_role(traits, &attributes(pairs))
}

#[test]
fn a_gtk_switch_resolves_switch_and_every_other_check_box_stays_checkbox() {
    let switch = LocatorField::Known("switch".to_owned());
    let checkbox = LocatorField::Known("checkbox".to_owned());
    assert_eq!(
        role_token(&traits_of(7, "switch"), &[("toolkit", "GTK")]),
        switch
    );
    assert_eq!(
        role_token(&traits_of(7, "switch"), &[("toolkit", "Qt")]),
        checkbox
    );
    assert_eq!(role_token(&traits_of(7, "switch"), &[]), checkbox);
    assert_eq!(
        role_token(&traits_of(7, "check box"), &[("toolkit", "GTK")]),
        checkbox
    );
    assert_eq!(
        role_token(&traits_of(43, "switch"), &[("toolkit", "GTK")]),
        LocatorField::Known("button".to_owned())
    );
}
