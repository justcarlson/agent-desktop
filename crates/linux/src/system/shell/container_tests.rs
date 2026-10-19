use agent_desktop_core::{Deadline, ErrorCode};

use crate::system::shell::container::{Anchor, Landmark, MENU, PUSH_BUTTON, Remembered, locate_in};
use crate::system::shell::fake_tree::{FakeTree, at, gnome_shell};

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn find(
    tree: &FakeTree,
    memory: &Remembered,
    landmark: Landmark,
) -> crate::system::shell::container::Located {
    locate_in(
        tree,
        memory,
        &FakeTree::root(),
        Anchor::Landmark(landmark),
        deadline(),
    )
    .expect("the landmark is in the tree")
}

#[test]
fn the_message_list_is_found_by_its_clear_button_open_or_closed() {
    for open in [true, false] {
        let tree = gnome_shell(open, false);
        let located = find(&tree, &Remembered::default(), Landmark::MessageList);
        assert_eq!(located.container.element.path(), "/list");
        assert_eq!(located.anchor.element.path(), "/clear");
        assert_eq!(located.anchor.showing, open);
    }
}

#[test]
fn a_copy_of_the_landmark_inside_the_overview_is_never_read() {
    let tree = gnome_shell(true, false);
    find(&tree, &Remembered::default(), Landmark::MessageList);
    assert!(!tree.reads().iter().any(|path| path == "/ov_clear"));
}

#[test]
fn the_system_menu_popup_is_six_levels_above_its_power_button() {
    let tree = gnome_shell(true, false);
    let located = find(&tree, &Remembered::default(), Landmark::SystemMenu);
    assert_eq!(located.container.element.path(), "/syspop");
    assert_eq!(located.anchor.element.path(), "/power");
}

#[test]
fn the_tray_panel_is_the_right_box_that_holds_the_system_button() {
    let tree = gnome_shell(false, false);
    let located = find(&tree, &Remembered::default(), Landmark::TrayPanel);
    assert_eq!(located.container.element.path(), "/right");
}

#[test]
fn the_date_menu_is_the_unnamed_menu_near_the_middle_of_the_top_bar() {
    let mut tree = gnome_shell(false, false);
    tree.add(Some("/right"), "/odd", MENU, "", at(1230, 0, 20, 32));
    let located = find(&tree, &Remembered::default(), Landmark::DateMenu);
    assert_eq!(located.anchor.element.path(), "/date");
}

#[test]
fn a_missing_landmark_is_an_error_naming_what_was_not_found() {
    let mut tree = gnome_shell(false, false);
    tree.vanish("/clear");
    let error = locate_in(
        &tree,
        &Remembered::default(),
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
        &Remembered::default(),
        &FakeTree::root(),
        Anchor::Landmark(Landmark::DockPanel),
        deadline(),
    )
    .expect_err("the overview copy is skipped");
    assert_eq!(error.code, ErrorCode::ElementNotFound);
}

#[test]
fn a_remembered_find_costs_two_reads_and_is_checked_against_its_rule() {
    let tree = gnome_shell(true, false);
    let memory = Remembered::default();
    find(&tree, &memory, Landmark::MessageList);
    let before = tree.reads().len();
    let again = find(&tree, &memory, Landmark::MessageList);
    assert_eq!(tree.reads().len() - before, 2);
    assert_eq!(again.container.element.path(), "/list");
}

#[test]
fn a_remembered_path_that_lost_its_landmark_is_searched_again() {
    let mut tree = gnome_shell(true, false);
    let memory = Remembered::default();
    find(&tree, &memory, Landmark::MessageList);
    tree.rename("/clear", "Something else");
    tree.add(
        Some("/footer"),
        "/clear2",
        PUSH_BUTTON,
        "Clear all notifications",
        at(615, 535, 86, 30),
    );
    let again = find(&tree, &memory, Landmark::MessageList);
    assert_eq!(again.anchor.element.path(), "/clear2");
}

#[test]
fn a_remembered_find_of_another_shell_is_not_trusted() {
    let tree = gnome_shell(true, false);
    let memory = Remembered::default();
    find(&tree, &memory, Landmark::MessageList);
    let mut restarted = FakeTree::root();
    restarted.bus_name = ":1.99".to_owned();
    let error = locate_in(
        &tree,
        &memory,
        &restarted,
        Anchor::Landmark(Landmark::MessageList),
        deadline(),
    );
    assert!(
        error.is_err(),
        "the remembered nodes belong to the old shell"
    );
}
