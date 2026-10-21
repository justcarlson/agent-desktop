use super::*;
use crate::tree::test_support::read_of;

fn node(x: i32, y: i32, children: Vec<WalkedNode<ObservedRead>>) -> WalkedNode<ObservedRead> {
    let mut payload = read_of(1, "n", &[]);
    payload.node.extents = Field::Present(Extents {
        x,
        y,
        width: 10,
        height: 10,
    });
    WalkedNode {
        payload,
        children,
        truncated: false,
        child_count: None,
        source_index: 0,
        in_web_document: false,
    }
}

fn gtk4() -> ToolkitId {
    ToolkitId {
        name: Some("GTK".to_owned()),
        version: Some("4.14.5".to_owned()),
    }
}

#[test]
fn gtk4_menu_origin_selection_uses_the_popup_source() {
    assert_eq!(
        select_origin(true, || Some((730, 410)), || Some((120, 80))),
        Some((730, 410))
    );
}

#[test]
fn ordinary_window_origin_selection_uses_the_managed_client_source() {
    assert_eq!(
        select_origin(false, || Some((730, 410)), || Some((120, 80))),
        Some((120, 80))
    );
}

fn window(x: i32, y: i32) -> Option<Extents> {
    Some(Extents {
        x,
        y,
        width: 10,
        height: 10,
    })
}

fn extents_of(node: &WalkedNode<ObservedRead>) -> Option<(i32, i32)> {
    match &node.payload.node.extents {
        Field::Present(extents) => Some((extents.x, extents.y)),
        _ => None,
    }
}

#[test]
fn a_gtk4_node_is_placed_at_its_window_rectangle_moved_to_the_client_origin() {
    let mut root = node(0, 0, vec![node(0, 0, Vec::new())]);
    apply(
        &mut root,
        &gtk4(),
        Some((460, 165)),
        &[window(0, 0), window(180, 46)],
    );
    assert_eq!(extents_of(&root), Some((460, 165)));
    assert_eq!(extents_of(&root.children[0]), Some((640, 211)));
}

#[test]
fn a_gtk4_menu_uses_the_owning_popup_origin_for_pointer_bounds() {
    let mut menu = node(0, 0, vec![node(0, 0, Vec::new())]);
    apply(
        &mut menu,
        &gtk4(),
        Some((730, 410)),
        &[window(0, 0), window(26, 38)],
    );

    assert_eq!(extents_of(&menu), Some((730, 410)));
    assert_eq!(extents_of(&menu.children[0]), Some((756, 448)));
}

#[test]
fn without_a_client_origin_a_node_keeps_its_window_rectangle_as_reference_evidence() {
    let mut root = node(0, 0, vec![node(0, 0, Vec::new())]);
    apply(&mut root, &gtk4(), None, &[window(1, 2), window(180, 46)]);
    assert_eq!(extents_of(&root), None);
    assert_eq!(extents_of(&root.children[0]), None);
    assert_eq!(
        root.children[0].payload.evidence_bounds,
        Some((window(180, 46).expect("rect"), BoundsSpace::Window))
    );
}

#[test]
fn a_client_origin_keeps_a_screen_position_and_the_window_rectangle_as_reference_evidence() {
    let mut root = node(0, 0, Vec::new());
    apply(&mut root, &gtk4(), Some((460, 165)), &[window(1, 2)]);
    assert_eq!(extents_of(&root), Some((461, 167)));
    assert_eq!(
        root.payload.evidence_bounds,
        Some((window(1, 2).expect("rect"), BoundsSpace::Window))
    );
}

#[test]
fn a_node_whose_window_rectangle_was_not_read_loses_its_position() {
    let mut root = node(0, 0, vec![node(0, 0, Vec::new())]);
    apply(&mut root, &gtk4(), Some((460, 165)), &[window(0, 0), None]);
    assert_eq!(extents_of(&root), Some((460, 165)));
    assert_eq!(extents_of(&root.children[0]), None);
}

#[test]
fn a_node_off_every_screen_consumes_no_window_rectangle() {
    let mut root = node(
        0,
        0,
        vec![node(i32::MIN, i32::MIN, Vec::new()), node(0, 0, Vec::new())],
    );
    apply(
        &mut root,
        &gtk4(),
        Some((100, 100)),
        &[window(0, 0), window(5, 6)],
    );
    assert_eq!(extents_of(&root.children[0]), Some((i32::MIN, i32::MIN)));
    assert_eq!(extents_of(&root.children[1]), Some((105, 106)));
}

#[test]
fn placed_elements_follow_the_order_apply_consumes_them_in() {
    let mut root = node(
        0,
        0,
        vec![
            node(i32::MIN, i32::MIN, Vec::new()),
            node(0, 0, vec![node(0, 0, Vec::new())]),
        ],
    );
    assert_eq!(placed_elements(&mut root).len(), 3);
}
