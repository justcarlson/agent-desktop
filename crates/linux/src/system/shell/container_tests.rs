use agent_desktop_core::{Deadline, ErrorCode};

use super::{Landmark, locate_in};
use crate::system::shell::anchor::Anchor;
use crate::system::shell::fake_tree::{FakeTree, PANEL, WINDOW, at, gnome_shell};
use crate::tree::role_numbers::{LIST, NOTIFICATION};

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
    for landmark in [
        Landmark::MessageList,
        Landmark::SystemMenu,
        Landmark::DateMenu,
        Landmark::DockPanel,
    ] {
        let error =
            locate(&tree, landmark).expect_err("generic panels and labels prove no popup identity");
        assert_eq!(error.code, ErrorCode::ActionNotSupported, "{landmark:?}");
        assert_eq!(error.details.unwrap()["kind"], "shell_container_unproven");
    }
}

#[test]
fn an_unproven_native_tray_uses_only_the_logical_shell_root() {
    let tree = gnome_shell(false, false);
    let located =
        locate(&tree, Landmark::TrayPanel).expect("a logical tray is independent of panel layout");
    assert_eq!(located.container.element.path(), "/w");
    assert_eq!(located.container.role, WINDOW);
}

#[test]
fn notification_list_identity_follows_roles_through_wrappers_in_any_locale() {
    for wrappers in [0, 1, 7] {
        let mut tree = FakeTree::new();
        tree.add(None, "/w", WINDOW, "", at(-300, 500, 230, 90));
        tree.add(
            Some("/w"),
            "/overview",
            PANEL,
            "Overview",
            at(100, -50, 20, 200),
        );
        let mut parent = "/overview".to_owned();
        for index in 0..wrappers {
            let path = format!("/wrapper{index}");
            tree.add(Some(&parent), &path, PANEL, "", at(0, 0, 0, 0));
            parent = path;
        }
        tree.add(Some(&parent), "/history", LIST, "履歴", at(0, 0, 0, 0));
        tree.add(Some("/history"), "/row", PANEL, "", at(0, 0, 0, 0));
        tree.add(
            Some("/row"),
            "/notification",
            NOTIFICATION,
            "通知",
            at(0, 0, 0, 0),
        );
        let located = locate(&tree, Landmark::MessageList).expect("roles prove the list");
        assert_eq!(located.container.element.path(), "/history");
    }
}

#[test]
fn two_notification_lists_refuse_instead_of_selecting_by_order() {
    let mut tree = FakeTree::new();
    tree.add(None, "/w", WINDOW, "", at(0, 0, 0, 0));
    for (list, notice) in [("/a", "/n1"), ("/b", "/n2")] {
        tree.add(Some("/w"), list, LIST, "", at(0, 0, 0, 0));
        tree.add(Some(list), notice, NOTIFICATION, "", at(0, 0, 0, 0));
    }
    assert_eq!(
        locate(&tree, Landmark::MessageList).unwrap_err().code,
        ErrorCode::AmbiguousTarget
    );
}
