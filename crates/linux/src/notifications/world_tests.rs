use super::*;
use crate::tree::test_support::read_of;
use crate::tree::{Field, WalkCounts};

fn walked(
    payload: ObservedRead,
    children: Vec<WalkedNode<ObservedRead>>,
) -> WalkedNode<ObservedRead> {
    WalkedNode {
        payload,
        children,
        truncated: false,
        child_count: None,
        source_index: 0,
        in_web_document: false,
    }
}

fn walk_of(root: WalkedNode<ObservedRead>, complete: bool) -> Walk<ObservedRead> {
    Walk {
        root,
        complete,
        node_count: 3,
        max_depth: 2,
        counts: WalkCounts::default(),
    }
}

#[test]
fn project_flattens_in_document_order_with_depths_and_roles() {
    let tree = walked(
        read_of(39, "", &[]),
        vec![walked(
            read_of(NOTIFICATION, "", &[]),
            vec![
                walked(read_of(LABEL, "Mail", &[]), vec![]),
                walked(read_of(PUSH_BUTTON, "Run", &[]), vec![]),
            ],
        )],
    );
    let reading = project(&walk_of(tree, true));
    let shape: Vec<(usize, Role, &str)> = reading
        .nodes
        .iter()
        .map(|node| (node.depth, node.role, node.name.as_str()))
        .collect();
    assert_eq!(
        shape,
        vec![
            (0, Role::Other, ""),
            (1, Role::Notification, ""),
            (2, Role::Label, "Mail"),
            (2, Role::PushButton, "Run"),
        ]
    );
    assert!(reading.complete);
}

#[test]
fn a_cut_short_walk_is_not_complete() {
    let tree = walked(read_of(39, "", &[]), vec![]);
    assert!(!project(&walk_of(tree, false)).complete);
}

#[test]
fn an_unreadable_name_makes_the_reading_incomplete() {
    let mut payload = read_of(LABEL, "x", &[]);
    payload.node.properties.name = Field::Failed("gone".to_owned());
    let tree = walked(read_of(39, "", &[]), vec![walked(payload, vec![])]);
    assert!(!project(&walk_of(tree, true)).complete);
}

#[test]
fn an_unreadable_role_makes_the_reading_incomplete() {
    let mut payload = read_of(LABEL, "x", &[]);
    payload.node.traits.role = Field::Failed("gone".to_owned());
    let tree = walked(read_of(39, "", &[]), vec![walked(payload, vec![])]);
    assert!(!project(&walk_of(tree, true)).complete);
}
