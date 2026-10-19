use super::super::identity;
use super::super::menu_layout::{Prop, RawEntry, parse};
use super::*;

fn raw(id: i32, label: &str, children: Vec<RawEntry>) -> RawEntry {
    RawEntry {
        id,
        props: [("label".to_owned(), Prop::Text(label.into()))]
            .into_iter()
            .collect(),
        children,
    }
}

fn item() -> TrayTarget {
    TrayTarget {
        owner: ":1.7".into(),
        object_path: "/org/ayatana/NotificationItem/x".into(),
        menu_path: None,
        entry_id: None,
        revision: None,
        label_path: Vec::new(),
    }
}

fn layout() -> Layout {
    parse(
        4,
        &raw(
            0,
            "",
            vec![
                raw(1, "Open", vec![]),
                raw(2, "More", vec![raw(3, "Deep", vec![])]),
            ],
        ),
    )
}

#[test]
fn every_entry_counts_once_at_every_depth() {
    assert_eq!(child_count(&layout().root.children), 3);
    assert_eq!(child_count(&[]), 0);
}

#[test]
fn a_target_built_for_an_entry_survives_its_own_encoding() {
    let target = entry_target(
        &item(),
        "/MenuBar",
        4,
        vec!["More".into(), "Deep".into()],
        3,
    );
    let decoded = identity::decode(&identity::encode(&target)).expect("decodes");
    assert_eq!(decoded.entry_id, Some(3));
    assert_eq!(decoded.label_path, ["More", "Deep"]);
    assert_eq!(decoded.menu_path.as_deref(), Some("/MenuBar"));
}

#[test]
fn states_follow_enabled_checked_and_submenu() {
    let mut entry = layout().root.children[0].clone();
    assert!(states_of(&entry).is_empty());
    entry.enabled = false;
    entry.checked = Some(true);
    entry.submenu = true;
    assert_eq!(states_of(&entry), ["disabled", "checked", "haspopup"]);
}

#[test]
fn a_separator_has_the_separator_role_and_everything_else_the_item_role() {
    let mut entry = layout().root.children[0].clone();
    assert_eq!(role_of(&entry), "menuitem");
    entry.separator = true;
    assert_eq!(role_of(&entry), "separator");
}
