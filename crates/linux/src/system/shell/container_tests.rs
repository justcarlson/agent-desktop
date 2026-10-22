use agent_desktop_core::{Deadline, ErrorCode};

use super::{Landmark, locate_in};
use crate::system::shell::anchor::Anchor;
use crate::system::shell::fake_tree::MENU;
use crate::system::shell::fake_tree::{FakeTree, PANEL, WINDOW, at, gnome_shell};
use crate::tree::role_numbers::{LABEL, PUSH_BUTTON, TOGGLE_BUTTON};

fn locate(
    tree: &FakeTree,
    landmark: Landmark,
) -> Result<super::Located, agent_desktop_core::AdapterError> {
    locate_in(
        tree,
        &FakeTree::root(),
        Anchor::Landmark(landmark),
        Deadline::after(5_000).unwrap(),
    )
}

#[test]
fn labels_depths_and_panel_geometry_do_not_prove_popup_identity() {
    let tree = gnome_shell(true, true);
    for landmark in [Landmark::SystemMenu, Landmark::DockPanel] {
        let error =
            locate(&tree, landmark).expect_err("generic panels and labels prove no popup identity");
        assert_eq!(error.code, ErrorCode::ActionNotSupported, "{landmark:?}");
        assert_eq!(error.details.unwrap()["kind"], "shell_container_unproven");
    }
}

#[test]
fn the_message_list_is_the_container_two_levels_above_the_footer_row() {
    for open in [true, false] {
        let located = locate(&gnome_shell(open, false), Landmark::MessageList)
            .expect("the footer row proves the list");
        assert_eq!(located.container.element.path(), "/list");
    }
}

#[test]
fn the_date_menu_is_the_one_showing_menu_without_a_name() {
    let located = locate(&gnome_shell(false, false), Landmark::DateMenu).expect("date menu");
    assert_eq!(located.anchor.element.path(), "/date");
}

#[test]
fn a_clear_button_named_like_the_footer_proves_nothing_without_its_row() {
    let mut tree = FakeTree::new();
    tree.add(None, "/w", WINDOW, "", at(0, 0, 0, 0));
    tree.add(Some("/w"), "/p", PANEL, "", at(0, 0, 0, 0));
    tree.add(Some("/p"), "/q", PANEL, "", at(0, 0, 0, 0));
    tree.add(
        Some("/q"),
        "/clear",
        PUSH_BUTTON,
        "Clear all notifications",
        at(0, 0, 10, 10),
    );
    let error = locate(&tree, Landmark::MessageList).expect_err("no footer row");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn two_footer_rows_refuse_instead_of_selecting_by_order() {
    let mut tree = gnome_shell(true, false);
    tree.add(Some("/p2"), "/list2", PANEL, "", at(0, 0, 10, 10));
    tree.add(Some("/list2"), "/footer2", PANEL, "", at(0, 0, 10, 10));
    tree.add(Some("/footer2"), "/l2", LABEL, "", at(0, 0, 10, 10));
    tree.add(Some("/footer2"), "/t2", TOGGLE_BUTTON, "", at(0, 0, 10, 10));
    tree.add(Some("/footer2"), "/c2", PUSH_BUTTON, "", at(0, 0, 10, 10));
    assert_eq!(
        locate(&tree, Landmark::MessageList).unwrap_err().code,
        ErrorCode::AmbiguousTarget
    );
}

#[test]
fn a_second_unnamed_showing_menu_makes_the_date_menu_ambiguous() {
    let mut tree = gnome_shell(false, false);
    tree.add(Some("/center"), "/other", MENU, "", at(10, 0, 50, 32));
    assert_eq!(
        locate(&tree, Landmark::DateMenu).unwrap_err().code,
        ErrorCode::AmbiguousTarget
    );
}

#[test]
fn an_unproven_native_tray_uses_only_the_logical_shell_root() {
    let tree = gnome_shell(false, false);
    let located =
        locate(&tree, Landmark::TrayPanel).expect("a logical tray is independent of panel layout");
    assert_eq!(located.container.element.path(), "/w");
    assert_eq!(located.container.role, WINDOW);
}
