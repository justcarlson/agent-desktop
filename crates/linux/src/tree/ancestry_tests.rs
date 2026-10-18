use std::collections::HashMap;

use super::{Ancestry, child, of_root};
use crate::tree::test_support::read_of;
use crate::tree::walker_expand::child_elements;
use crate::tree::{AtspiElement, Field};

fn element(bus: &str, path: &str, pid: u32) -> AtspiElement {
    AtspiElement::new(bus, path, pid)
}

#[test]
fn a_child_inherits_the_frame_and_keeps_the_parents_container() {
    let frame = element(":1.1", "/root/frame", 1);
    let scroll = element(":1.1", "/root/frame/scroll", 1);
    let mut parent = element(":1.1", "/root/frame/scroll/inner", 1);
    parent.set_ancestry(Ancestry::of(
        Some(frame.clone()),
        Some(scroll.clone()),
        false,
    ));
    let ancestry = child(&parent, Some(39));
    assert_eq!(ancestry.frame(), Some(&frame));
    assert_eq!(ancestry.scroll_container(), Some(&scroll));
}

#[test]
fn a_scroll_pane_or_viewport_parent_is_itself_the_container() {
    for role in [49, 68] {
        let mut parent = element(":1.1", "/root/frame/scroll", 1);
        parent.set_ancestry(Ancestry::of(
            Some(element(":1.1", "/root/frame", 1)),
            None,
            false,
        ));
        let ancestry = child(&parent, Some(role));
        assert_eq!(ancestry.scroll_container(), Some(&parent.address()));
    }
}

#[test]
fn two_handles_to_one_address_with_different_ancestry_compare_equal() {
    let plain = element(":1.1", "/root/frame/node", 1);
    let mut with = element(":1.1", "/root/frame/node", 1);
    with.set_ancestry(of_root(&element(":1.1", "/root/frame", 1)));
    assert_eq!(plain, with);
}

#[test]
fn a_handle_with_no_ancestry_gives_none_for_both() {
    let element = element(":1.1", "/root/frame/node", 1);
    assert_eq!(element.frame(), None);
    assert_eq!(element.scroll_container(), None);
}

#[test]
fn children_inherit_the_frame_and_the_parent_as_container_when_it_scrolls() {
    let frame = element(":1.1", "/root/frame", 1);
    let mut read = read_of(49, "scroll", &[]);
    read.node.element = element(":1.1", "/root/frame/scroll", 1);
    read.node
        .element
        .set_ancestry(Ancestry::of(Some(frame.clone()), None, false));
    read.node.children = Field::Present(vec![(
        ":2.9".to_owned(),
        "/org/a11y/atspi/accessible/4".to_owned(),
    )]);
    let pids = HashMap::from([(":2.9".to_owned(), 9u32)]);
    let children = child_elements(&read.node, &pids);
    assert_eq!(children.len(), 1);
    let child = &children[0];
    assert_eq!(child.bus_name(), ":2.9");
    assert_eq!(child.pid(), 9);
    assert_eq!(child.frame(), Some(&frame));
    assert_eq!(
        child.scroll_container(),
        Some(&element(":1.1", "/root/frame/scroll", 1))
    );
}

fn parent_in(web: bool) -> AtspiElement {
    let mut parent = element(":1.1", "/root/frame/node", 1);
    parent.set_ancestry(Ancestry::of(None, None, web));
    parent
}

#[test]
fn a_web_document_parent_puts_its_child_inside_a_web_document() {
    for role in [95, 82] {
        assert!(child(&parent_in(false), Some(role)).in_web_document());
    }
}

#[test]
fn a_child_stays_inside_a_web_document_whatever_its_parents_role() {
    assert!(child(&parent_in(true), Some(39)).in_web_document());
    assert!(child(&parent_in(true), None).in_web_document());
}

#[test]
fn a_child_outside_any_web_document_is_not_inside_one() {
    assert!(!child(&parent_in(false), Some(39)).in_web_document());
    assert!(!child(&parent_in(false), None).in_web_document());
    assert!(!of_root(&element(":1.1", "/root/frame", 1)).in_web_document());
}

#[test]
fn the_web_flag_survives_cloning_the_handle_but_not_the_address() {
    let mut node = element(":1.1", "/root/frame/node", 1);
    node.set_ancestry(Ancestry::of(None, None, true));
    assert!(node.clone().in_web_document());
    assert!(!node.address().in_web_document());
}
