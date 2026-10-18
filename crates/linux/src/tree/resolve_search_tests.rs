use agent_desktop_core::Rect;

use super::*;
use crate::tree::resolve_fake::{FRAME_PATH, Fake, node};
use crate::tree::role_numbers::{DOCUMENT_WEB, PUSH_BUTTON};

fn toolbar(fake: &mut Fake) {
    fake.add(FRAME_PATH, "/a", node(PUSH_BUTTON, "Open", 30, 20));
    fake.add(FRAME_PATH, "/b", node(PUSH_BUTTON, "Save", 30, 20));
    fake.add(FRAME_PATH, "/c", node(PUSH_BUTTON, "Save", 44, 20));
}

#[test]
fn every_node_that_answers_to_the_stored_evidence_is_a_candidate_with_its_extents() {
    let mut fake = Fake::new();
    toolbar(&mut fake);
    let entry = fake.entry_of("/b", true);
    let searched = evaluate(&fake.walk(), &entry, false);
    let paths: Vec<_> = searched
        .candidates
        .iter()
        .map(|candidate| candidate.element.path().to_owned())
        .collect();
    assert_eq!(paths, ["/b", "/c"]);
    let widths: Vec<_> = searched
        .candidates
        .iter()
        .map(|candidate| candidate.bounds.map(|rect: Rect| rect.width))
        .collect();
    assert_eq!(widths, [Some(30.0), Some(44.0)]);
    assert!(!searched.unread_region);
}

#[test]
fn a_walk_that_was_cut_short_leaves_a_region_unread() {
    let mut fake = Fake::new();
    toolbar(&mut fake);
    fake.nodes.get_mut("/a").expect("added").children_unread = true;
    let entry = fake.entry_of("/b", true);
    let searched = evaluate(&fake.walk(), &entry, false);
    assert!(searched.unread_region);
}

#[test]
fn a_childless_frame_of_a_web_engine_is_unread_and_a_native_one_is_empty() {
    let fake = Fake::new();
    let mut entry = fake.entry_of("/frame", true);
    entry.identity.name = Some("window".to_owned());
    assert!(evaluate(&fake.walk(), &entry, true).unread_region);
    assert!(!evaluate(&fake.walk(), &entry, false).unread_region);
}

#[test]
fn a_childless_web_document_is_unread_until_it_fills_in() {
    let mut fake = Fake::new();
    fake.add(
        FRAME_PATH,
        "/document",
        node(DOCUMENT_WEB, "page", 800, 500),
    );
    let entry = fake.entry_of("/document", true);
    assert!(evaluate(&fake.walk(), &entry, false).unread_region);
    fake.add("/document", "/button", node(PUSH_BUTTON, "Save", 30, 20));
    let filled = fake.entry_of("/button", true);
    assert!(!evaluate(&fake.walk(), &filled, false).unread_region);
}
