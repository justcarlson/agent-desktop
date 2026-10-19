use super::{Node, find, is_cut_short};
use crate::tree::role_numbers::{DIALOG, FRAME, PANEL};
use crate::tree::state_bits::SHOWING;
use crate::tree::test_support::{read_of, words};
use crate::tree::{AtspiElement, Field, WalkedNode};

fn node(role: u32, name: &str, showing: bool, path: &str, children: Vec<Node>) -> Node {
    let mut read = read_of(role, name, &[]);
    read.node.element = AtspiElement::new(":1.2", path, 7);
    let bits: &[u32] = if showing { &[SHOWING] } else { &[] };
    read.node.traits.states = Field::Present(words(bits));
    WalkedNode {
        payload: read,
        children,
        truncated: false,
        child_count: None,
        source_index: 0,
        in_web_document: false,
    }
}

fn window(children: Vec<Node>) -> Node {
    node(FRAME, "App", true, "/w", children)
}

fn deep_dialog(showing: bool) -> Node {
    let dialog = node(DIALOG, "About", showing, "/d", Vec::new());
    let inner = node(PANEL, "", true, "/p2", vec![dialog]);
    node(PANEL, "", true, "/p1", vec![inner])
}

#[test]
fn a_showing_dialog_below_the_window_is_found_with_its_address_and_title() {
    let tree = window(vec![deep_dialog(true)]);
    let found = find(&tree);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].element.path(), "/d");
    assert_eq!(found[0].title, Some("About"));
}

#[test]
fn a_dialog_that_stays_in_the_tree_without_showing_is_not_found() {
    assert!(find(&window(vec![deep_dialog(false)])).is_empty());
}

#[test]
fn a_dialog_that_left_the_tree_is_not_found() {
    let tree = window(vec![node(PANEL, "", true, "/p1", Vec::new())]);
    assert!(find(&tree).is_empty());
}

#[test]
fn the_window_root_is_never_a_nested_dialog() {
    let root = node(DIALOG, "About", true, "/w", Vec::new());
    assert!(find(&root).is_empty());
}

#[test]
fn only_the_outermost_of_nested_dialogs_is_found() {
    let inner = node(DIALOG, "Inner", true, "/inner", Vec::new());
    let outer = node(DIALOG, "Outer", true, "/outer", vec![inner]);
    let tree = window(vec![outer]);
    let found = find(&tree);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].element.path(), "/outer");
}

#[test]
fn a_blank_name_is_no_title() {
    let tree = window(vec![node(DIALOG, "", true, "/d", Vec::new())]);
    assert_eq!(find(&tree)[0].title, None);
}

#[test]
fn a_dialog_inside_a_web_document_is_not_found() {
    let mut inside = node(DIALOG, "page dialog", true, "/d", Vec::new());
    inside.in_web_document = true;
    assert!(find(&window(vec![inside])).is_empty());
}

#[test]
fn a_walk_with_an_unread_subtree_is_cut_short() {
    let mut cut = node(PANEL, "", true, "/p", Vec::new());
    cut.truncated = true;
    assert!(is_cut_short(&window(vec![cut])));
    assert!(!is_cut_short(&window(vec![deep_dialog(true)])));
}
