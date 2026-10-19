use crate::system::shell::fake_tree::{SENTINEL, at};
use crate::system::shell::light::{Light, is_placed};
use crate::tree::AtspiElement;

fn light(extents: Option<crate::tree::Extents>) -> Light {
    Light {
        element: AtspiElement::new(":1.1", "/a", 1),
        role: 43,
        name: "Go".to_owned(),
        extents,
        showing: true,
        visible: true,
        children: Vec::new(),
    }
}

#[test]
fn extents_name_a_place_only_with_area_and_no_sentinel() {
    assert!(is_placed(at(0, 0, 72, 32)));
    assert!(is_placed(at(-10, -1, 60, 84)));
    assert!(!is_placed(at(0, 0, 0, 32)));
    assert!(!is_placed(at(0, 0, 10, 0)));
    assert!(!is_placed(SENTINEL));
}

#[test]
fn a_light_matches_by_role_and_exact_name() {
    let node = light(None);
    assert!(node.is(43, "Go"));
    assert!(!node.is(43, "go"));
    assert!(!node.is(62, "Go"));
    assert!(node.placed().is_none());
    assert_eq!(light(Some(at(1, 2, 3, 4))).placed(), Some(at(1, 2, 3, 4)));
}
