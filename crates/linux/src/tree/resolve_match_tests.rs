use agent_desktop_core::{IdentifierEvidence, LocatorField};

use super::*;
use crate::tree::test_support::{evidence, rect, ref_entry, with_identifier};

fn named(role: &str, name: &str) -> RefEntry {
    let mut entry = ref_entry(role);
    entry.identity.name = Some(name.to_owned());
    entry
}

fn sized(mut entry: RefEntry, bounds: Rect) -> RefEntry {
    entry.geometry.bounds = Some(bounds);
    entry.geometry.bounds_hash = bounds.bounds_hash();
    entry
}

#[test]
fn an_unreadable_role_withholds_the_verdict_and_a_different_role_is_rejected() {
    let entry = named("button", "Save");
    let mut unread = evidence("button", Some("Save"), None, None);
    unread.role = LocatorField::Unknown;
    assert_eq!(admit_node(&entry, &unread), Admission::Unread);

    let other = evidence("link", Some("Save"), None, None);
    assert_eq!(admit_node(&entry, &other), Admission::Reject);

    let same = evidence("button", Some("Save"), None, None);
    assert_eq!(admit_node(&entry, &same), Admission::Collect);
}

#[test]
fn rows_that_share_an_identifier_are_told_apart_by_their_stable_text() {
    let entry = with_identifier(named("cell", "alpha"), "list.row");
    let same_row = evidence("cell", Some("alpha"), Some("list.row"), None);
    let other_row = evidence("cell", Some("beta"), Some("list.row"), None);
    assert_eq!(admit_node(&entry, &same_row), Admission::Collect);
    assert_eq!(admit_node(&entry, &other_row), Admission::Reject);
}

#[test]
fn rows_with_one_identifier_and_one_text_both_stay_candidates() {
    let entry = with_identifier(named("cell", "alpha"), "list.row");
    let first = evidence("cell", Some("alpha"), Some("list.row"), None);
    let second = evidence("cell", Some("alpha"), Some("list.row"), None);
    assert_eq!(admit_node(&entry, &first), Admission::Collect);
    assert_eq!(admit_node(&entry, &second), Admission::Collect);
}

#[test]
fn an_identifier_match_with_no_stored_text_resolves_on_the_identifier_alone() {
    let entry = with_identifier(ref_entry("button"), "toolbar.save");
    let live = evidence("button", Some("anything"), Some("toolbar.save"), None);
    assert_eq!(admit_node(&entry, &live), Admission::Collect);
}

#[test]
fn an_identifier_read_that_was_incomplete_is_unread_and_never_a_refutation() {
    let entry = with_identifier(named("button", "Save"), "toolbar.save");
    let mut live = evidence("button", Some("Save"), None, None);
    live.identifiers = IdentifierEvidence::typed([], None, false);
    assert_eq!(admit_node(&entry, &live), Admission::Unread);
}

#[test]
fn an_identifier_match_whose_stored_text_cannot_be_read_is_unread() {
    let entry = with_identifier(named("button", "Save"), "toolbar.save");
    let mut live = evidence("button", None, Some("toolbar.save"), None);
    live.name = LocatorField::Unknown;
    assert_eq!(admit_node(&entry, &live), Admission::Unread);
}

#[test]
fn a_size_contradicts_only_when_both_sizes_are_known_and_differ() {
    let entry = sized(
        named("button", "View Options"),
        rect(10.0, 10.0, 34.0, 34.0),
    );
    let same_size_elsewhere = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(900.0, 400.0, 34.0, 34.0)),
    );
    let narrower = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(10.0, 10.0, 23.0, 34.0)),
    );
    let unreadable = {
        let mut live = evidence("button", Some("View Options"), None, None);
        live.ref_evidence.bounds = LocatorField::Unknown;
        live
    };
    let no_extents = evidence("button", Some("View Options"), None, None);
    let minus_one = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(-1.0, -1.0, -1.0, -1.0)),
    );
    assert!(!size_contradicts(&entry, &same_size_elsewhere));
    assert!(size_contradicts(&entry, &narrower));
    assert!(!size_contradicts(&entry, &unreadable));
    assert!(!size_contradicts(&entry, &no_extents));
    assert!(!size_contradicts(&entry, &minus_one));
}

#[test]
fn a_ref_that_stored_no_rectangle_or_an_empty_one_has_no_size_to_contradict() {
    let live = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(0.0, 0.0, 23.0, 34.0)),
    );
    assert!(!size_contradicts(&named("button", "View Options"), &live));
    let flat = sized(named("button", "View Options"), rect(0.0, 0.0, 34.0, 0.0));
    assert!(!size_contradicts(&flat, &live));
}

#[test]
fn a_landing_is_positional_so_a_twin_elsewhere_does_not_matter_but_a_resize_declines_it() {
    let entry = sized(
        named("button", "View Options"),
        rect(10.0, 10.0, 34.0, 34.0),
    );
    let landed = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(300.0, 20.0, 34.0, 34.0)),
    );
    let resized = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(300.0, 20.0, 23.0, 34.0)),
    );
    let renamed = evidence(
        "button",
        Some("Other"),
        None,
        Some(rect(300.0, 20.0, 34.0, 34.0)),
    );
    assert!(accepts_landing(&entry, &landed));
    assert!(!accepts_landing(&entry, &resized));
    assert!(!accepts_landing(&entry, &renamed));
}

#[test]
fn a_ref_with_no_stored_size_accepts_a_landing_that_matches_its_identity() {
    let entry = named("button", "View Options");
    let live = evidence(
        "button",
        Some("View Options"),
        None,
        Some(rect(0.0, 0.0, 23.0, 34.0)),
    );
    assert!(accepts_landing(&entry, &live));
}

#[test]
fn only_a_rectangle_with_an_area_can_identify_a_ref_that_has_no_text() {
    let anonymous = sized(ref_entry("entry"), rect(5.0, 5.0, 120.0, 24.0));
    assert!(provisional_geometry_candidate(&anonymous));
    assert!(!entry_is_unverifiable(&anonymous));

    let container = sized(ref_entry("group"), rect(0.0, 0.0, 710.0, 0.0));
    assert!(!provisional_geometry_candidate(&container));
    assert!(entry_is_unverifiable(&container));

    let named_entry = sized(named("group", "List View"), rect(0.0, 0.0, 710.0, 0.0));
    assert!(!provisional_geometry_candidate(&named_entry));
    assert!(!entry_is_unverifiable(&named_entry));
}

#[test]
fn a_ref_with_nothing_to_match_is_unverifiable_and_an_identifier_or_text_is_enough() {
    assert!(entry_is_unverifiable(&ref_entry("button")));
    assert!(!entry_is_unverifiable(&named("button", "Save")));
    assert!(!entry_is_unverifiable(&with_identifier(
        ref_entry("button"),
        "toolbar.save"
    )));
}

#[test]
fn an_unreadable_identity_is_promoted_by_a_matching_size_and_refuted_by_a_different_one() {
    let entry = sized(ref_entry("entry"), rect(5.0, 5.0, 120.0, 24.0));
    let same = evidence("entry", None, None, Some(rect(900.0, 9.0, 120.0, 24.0)));
    let other = evidence("entry", None, None, Some(rect(5.0, 5.0, 130.0, 24.0)));
    let mut unreadable_same = same.clone();
    unreadable_same.name = LocatorField::Unknown;
    let mut unreadable_other = other.clone();
    unreadable_other.name = LocatorField::Unknown;
    assert_eq!(admit_node(&entry, &unreadable_same), Admission::Collect);
    assert_eq!(admit_node(&entry, &unreadable_other), Admission::Reject);
}

#[test]
fn a_ref_known_only_by_its_size_is_refuted_by_a_node_whose_size_was_read_and_differs() {
    let entry = sized(ref_entry("textfield"), rect(0.0, 0.0, 690.0, 464.0));
    let mut resized = evidence("textfield", None, None, Some(rect(0.0, 0.0, 690.0, 463.0)));
    assert_eq!(admit_node(&entry, &resized), Admission::Reject);
    let same = evidence("textfield", None, None, Some(rect(5.0, 9.0, 690.0, 464.0)));
    assert_eq!(admit_node(&entry, &same), Admission::Collect);
    resized.ref_evidence.bounds = LocatorField::Unknown;
    assert_eq!(admit_node(&entry, &resized), Admission::Unread);
    resized.ref_evidence.bounds = LocatorField::Absent;
    assert_eq!(admit_node(&entry, &resized), Admission::Reject);
}
