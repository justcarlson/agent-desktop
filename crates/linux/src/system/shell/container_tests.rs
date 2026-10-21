use agent_desktop_core::{Deadline, ErrorCode};

use crate::system::shell::anchor::Anchor;
use crate::system::shell::container::{Landmark, MENU, locate_in};
use crate::system::shell::fake_tree::{FakeTree, at, gnome_shell};

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn find(tree: &FakeTree, landmark: Landmark) -> crate::system::shell::located::Located {
    locate_in(
        tree,
        &FakeTree::root(),
        Anchor::Landmark(landmark),
        deadline(),
    )
    .expect("the landmark is in the tree")
}

#[test]
fn a_copy_of_the_landmark_inside_the_overview_is_never_read() {
    let tree = gnome_shell(true, false);
    find(&tree, Landmark::MessageList);
    assert!(!tree.reads().iter().any(|path| path == "/ov_clear"));
}

#[test]
fn landmark_containers_follow_the_measured_tree() {
    for open in [true, false] {
        let tree = gnome_shell(open, false);
        let located = find(&tree, Landmark::MessageList);
        assert_eq!(located.container.element.path(), "/list");
        assert_eq!(located.anchor.element.path(), "/clear");
        assert_eq!(located.anchor.showing, open);
    }

    let tree = gnome_shell(true, false);
    let located = find(&tree, Landmark::SystemMenu);
    assert_eq!(located.container.element.path(), "/syspop");
    assert_eq!(located.anchor.element.path(), "/power");

    let tree = gnome_shell(false, false);
    let located = find(&tree, Landmark::TrayPanel);
    assert_eq!(located.container.element.path(), "/right");
}

#[test]
fn the_date_menu_is_the_unnamed_menu_near_the_middle_of_the_top_bar() {
    let mut tree = gnome_shell(false, false);
    tree.add(Some("/right"), "/odd", MENU, "", at(1230, 0, 20, 32));
    let located = find(&tree, Landmark::DateMenu);
    assert_eq!(located.anchor.element.path(), "/date");
}

#[test]
fn a_missing_landmark_is_an_error_naming_what_was_not_found() {
    let mut tree = gnome_shell(false, false);
    tree.vanish("/clear");
    let error = locate_in(
        &tree,
        &FakeTree::root(),
        Anchor::Landmark(Landmark::MessageList),
        deadline(),
    )
    .expect_err("no list is no result");
    assert_eq!(error.code, ErrorCode::ElementNotFound);
    assert!(error.message.contains("message list"));
}

#[test]
fn a_dock_that_exists_only_in_the_overview_is_not_a_dock() {
    let tree = gnome_shell(false, false);
    let error = locate_in(
        &tree,
        &FakeTree::root(),
        Anchor::Landmark(Landmark::DockPanel),
        deadline(),
    )
    .expect_err("the overview copy is skipped");
    assert_eq!(error.code, ErrorCode::ElementNotFound);
}
