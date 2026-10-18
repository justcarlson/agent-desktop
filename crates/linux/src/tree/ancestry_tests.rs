use std::collections::HashMap;

use super::{Ancestry, child, of_root, select};
use crate::tree::test_support::read_of;
use crate::tree::walker_expand::child_elements;
use crate::tree::{AtspiElement, Field};

fn element(bus: &str, path: &str, pid: u32) -> AtspiElement {
    AtspiElement::new(bus, path, pid)
}

fn at(element: &AtspiElement, role: u32) -> (AtspiElement, Option<u32>) {
    (element.clone(), Some(role))
}

#[test]
fn the_nearest_scroll_container_wins_over_an_outer_one() {
    let frame = element(":1.1", "/root/frame", 1);
    let outer = element(":1.1", "/root/frame/panel/scroll", 1);
    let viewport = element(":1.1", "/root/frame/panel/scroll/viewport", 1);
    let chain = vec![
        at(&frame, 23),
        at(&element(":1.1", "/root/frame/panel", 1), 39),
        at(&outer, 49),
        at(&viewport, 68),
        at(
            &element(":1.1", "/root/frame/panel/scroll/viewport/list", 1),
            31,
        ),
    ];
    let ancestry = select(&chain);
    assert_eq!(ancestry.frame(), Some(&frame));
    assert_eq!(ancestry.scroll_container(), Some(&viewport));
}

#[test]
fn a_chain_without_a_container_and_an_empty_chain() {
    let chain = [
        at(&element(":1.1", "/root/frame", 1), 23),
        at(&element(":1.1", "/root/frame/panel", 1), 39),
    ];
    let ancestry = select(&chain);
    assert_eq!(ancestry.frame(), Some(&element(":1.1", "/root/frame", 1)));
    assert_eq!(ancestry.scroll_container(), None);
    let empty = select(&[]);
    assert_eq!(empty.frame(), None);
    assert_eq!(empty.scroll_container(), None);
}

#[test]
fn a_child_inherits_the_frame_and_keeps_the_parents_container() {
    let frame = element(":1.1", "/root/frame", 1);
    let scroll = element(":1.1", "/root/frame/scroll", 1);
    let mut parent = element(":1.1", "/root/frame/scroll/inner", 1);
    parent.set_ancestry(Ancestry::of(Some(frame.clone()), Some(scroll.clone())));
    let ancestry = child(&parent, Some(39));
    assert_eq!(ancestry.frame(), Some(&frame));
    assert_eq!(ancestry.scroll_container(), Some(&scroll));
}

#[test]
fn a_scroll_pane_or_viewport_parent_is_itself_the_container() {
    for role in [49, 68] {
        let mut parent = element(":1.1", "/root/frame/scroll", 1);
        parent.set_ancestry(Ancestry::of(Some(element(":1.1", "/root/frame", 1)), None));
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
        .set_ancestry(Ancestry::of(Some(frame.clone()), None));
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
