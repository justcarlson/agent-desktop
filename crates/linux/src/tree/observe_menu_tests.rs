use super::rooted;
use crate::tree::AtspiElement;

#[test]
fn the_popup_root_is_its_own_frame_and_not_the_window() {
    let root = rooted(&AtspiElement::new(":1.2", "/popup", 7));
    assert_eq!(root.frame().map(AtspiElement::path), Some("/popup"));
}
