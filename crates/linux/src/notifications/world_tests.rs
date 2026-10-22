use std::collections::BTreeMap;

use super::*;
use crate::tree::role_numbers::{LABEL, NOTIFICATION, PUSH_BUTTON};

struct Tree(BTreeMap<String, Light>);

fn at(path: &str) -> AtspiElement {
    AtspiElement::new(":1.1", path, 1)
}

impl Tree {
    fn add(&mut self, parent: Option<&str>, path: &str, role: u32, name: &str) {
        self.0.insert(
            path.to_owned(),
            Light {
                element: at(path),
                role,
                name: name.to_owned(),
                extents: None,
                showing: true,
                visible: true,
                children: Vec::new(),
            },
        );
        if let Some(parent) = parent.and_then(|parent| self.0.get_mut(parent)) {
            parent.children.push(at(path));
        }
    }
}

impl Nodes for Tree {
    fn read(&self, element: &AtspiElement) -> Result<Light, AdapterError> {
        self.0
            .get(element.path())
            .cloned()
            .ok_or_else(|| AdapterError::new(ErrorCode::ElementNotFound, "gone"))
    }
}

fn deadline() -> Deadline {
    Deadline::after(1_000).expect("a deadline")
}

fn sample() -> Tree {
    let mut tree = Tree(BTreeMap::new());
    tree.add(None, "/root", 39, "");
    tree.add(Some("/root"), "/n", NOTIFICATION, "");
    tree.add(Some("/n"), "/n/app", LABEL, "Mail");
    tree.add(Some("/n"), "/n/run", PUSH_BUTTON, "Run");
    tree.add(Some("/root"), "/footer", 39, "");
    tree
}

#[test]
fn a_subtree_flattens_in_document_order_with_depths_and_roles() {
    let reading = read_subtree(&sample(), &at("/root"), deadline()).expect("read");
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
            (1, Role::Other, ""),
        ]
    );
    assert!(reading.complete);
}

#[test]
fn a_child_that_vanishes_makes_the_reading_incomplete() {
    let mut tree = sample();
    tree.0.remove("/n/run");
    let reading = read_subtree(&tree, &at("/root"), deadline()).expect("read");
    assert!(!reading.complete);
    assert_eq!(reading.nodes.len(), 4);
}
