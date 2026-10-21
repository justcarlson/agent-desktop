use agent_desktop_core::ErrorCode;

use super::*;
use crate::tree::resolve_fake::{FRAME_PATH, Fake, deadline, node};
use crate::tree::role_numbers::{PANEL, PUSH_BUTTON};

/// A list whose rows each hold a twin of themselves: same role, name and size.
fn rows(names: &[&str]) -> Fake {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/list", node(PANEL, "list", 400, 300));
    for name in names {
        let outer = format!("/{name}");
        let inner = format!("/{name}-inner");
        fake.add("/list", &outer, node(PUSH_BUTTON, name, 100, 20));
        fake.add(&outer, &inner, node(PUSH_BUTTON, name, 100, 20));
    }
    fake
}

fn without_row(fake: &mut Fake, name: &str) {
    fake.nodes
        .get_mut("/list")
        .expect("list")
        .children
        .retain(|path| path != &format!("/{name}"));
}

fn resolved(fake: &mut Fake, entry: &RefEntry) -> String {
    resolve(fake, entry, deadline())
        .expect("the ref resolves")
        .path()
        .to_owned()
}

#[test]
fn nested_twins_resolve_by_the_depth_the_ref_was_taken_at_after_a_sibling_row_leaves() {
    let mut fake = rows(&["alpha", "beta"]);
    let outer = fake.entry_of("/beta", false);
    let inner = fake.entry_of("/beta-inner", false);
    assert_eq!(outer.scope.path.len() + 1, inner.scope.path.len());
    without_row(&mut fake, "alpha");
    assert_eq!(resolved(&mut fake, &outer), "/beta");
    assert_eq!(resolved(&mut fake, &inner), "/beta-inner");
    assert_eq!(fake.reads.searches, 2);
}

#[test]
fn removed_sibling_refs_stay_stale_while_surviving_twins_keep_identity() {
    let mut fake = rows(&["alpha", "beta"]);
    let removed = [
        fake.entry_of("/alpha", false),
        fake.entry_of("/alpha-inner", false),
    ];
    let survivors = [
        fake.entry_of("/beta", false),
        fake.entry_of("/beta-inner", false),
    ];
    without_row(&mut fake, "alpha");
    for mut entry in removed {
        entry.scope.path.clear();
        let error = resolve(&mut fake, &entry, deadline()).expect_err("removed row");
        assert_eq!(error.code, ErrorCode::StaleRef);
    }
    assert_eq!(resolved(&mut fake, &survivors[0]), "/beta");
    assert_eq!(resolved(&mut fake, &survivors[1]), "/beta-inner");
    fake.stopped = Some("files".to_owned());
    let error = resolve(&mut fake, &survivors[0], deadline()).expect_err("stopped app");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
}

#[test]
fn twins_at_neither_the_stored_depth_nor_one_of_them_stay_ambiguous() {
    let mut fake = rows(&["beta"]);
    let mut entry = fake.entry_of("/beta", false);
    entry.scope.path = [0, 0, 0, 0].into_iter().collect();
    let error = resolve(&mut fake, &entry, deadline()).expect_err("ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn two_twins_at_the_stored_depth_stay_ambiguous() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/a", node(PUSH_BUTTON, "Save", 30, 20));
    fake.add(FRAME_PATH, "/b", node(PUSH_BUTTON, "Save", 30, 20));
    let mut entry = fake.entry_of("/b", false);
    entry.scope.path = [5].into_iter().collect();
    let error = resolve(&mut fake, &entry, deadline()).expect_err("ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn a_region_left_unread_withholds_the_depth_pick() {
    let mut fake = rows(&["alpha", "beta"]);
    let outer = fake.entry_of("/beta", false);
    without_row(&mut fake, "alpha");
    fake.nodes.get_mut("/list").expect("list").children_unread = true;
    let error = resolve(&mut fake, &outer, deadline()).expect_err("incomplete");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert!(error.is_explicitly_retryable());
}

#[test]
fn a_drill_down_ref_never_picks_by_depth() {
    let mut fake = rows(&["beta"]);
    let mut entry = fake.entry_of("/beta", false);
    entry.scope.root_ref = Some("@s1:e3".to_owned());
    entry.scope.path_is_absolute = false;
    let error = resolve(&mut fake, &entry, deadline()).expect_err("ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}
