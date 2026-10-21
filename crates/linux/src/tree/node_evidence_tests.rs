use agent_desktop_core::{ErrorCode, IdentifierKind};

use super::*;
use crate::tree::ancestry::{self, Ancestry};
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::{attributes, read_of};

fn dom_button() -> ObservedRead {
    let mut read = read_of(PUSH_BUTTON, "Save", &["click"]);
    read.node.attributes = attributes(&[("id", "save-button")]);
    read
}

fn element_in(web: bool) -> AtspiElement {
    let mut element = AtspiElement::new(":1.2", "/button", 40);
    let frame = AtspiElement::new(":1.2", "/frame", 40);
    element.set_ancestry(Ancestry::of(Some(frame), None, web));
    element
}

#[test]
fn a_dom_identifier_is_read_below_a_web_document_and_only_there() {
    let read = dom_button();
    let in_web = evidence_for(&read, &element_in(true));
    let kinds: Vec<_> = in_web
        .identifiers
        .identifiers()
        .iter()
        .map(|identifier| identifier.kind)
        .collect();
    assert_eq!(kinds, [IdentifierKind::AxDomIdentifier]);

    let outside = evidence_for(&read, &element_in(false));
    assert!(outside.identifiers.identifiers().is_empty());
    assert!(outside.identifiers.is_complete());
}

#[test]
fn a_handle_with_no_window_frame_is_refused_rather_than_read_with_a_guessed_flag() {
    let bare = AtspiElement::new(":1.2", "/button", 40);
    let error = require_ancestry(&bare).expect_err("no ancestry");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert!(require_ancestry(&element_in(false)).is_ok());
    let frame = AtspiElement::new(":1.2", "/frame", 40);
    assert!(
        require_ancestry(&{
            let mut e = AtspiElement::new(":1.2", "/x", 40);
            e.set_ancestry(ancestry::of_root(&frame));
            e
        })
        .is_ok()
    );
}

#[test]
fn menu_regression_inner_toggle_parent_keeps_its_frame_for_live_reads() {
    use crate::tree::ancestry::{self, Ancestry};
    for web in [false, true] {
        let frame = AtspiElement::new(":1.1", "/frame", 7);
        let mut parent = AtspiElement::new(":1.1", "/button", 7);
        parent.set_ancestry(Ancestry::of(Some(frame.clone()), None, web));
        let mut inner = AtspiElement::new(":1.1", "/toggle", 7);
        inner.set_ancestry(ancestry::child(&parent, Some(PUSH_BUTTON)));
        let restored = inner
            .ancestry()
            .button_parent()
            .expect("walked popup opener");
        require_ancestry(&restored).expect("parent live reads must retain their known context");
        assert_eq!(restored, parent);
        assert_eq!(restored.frame(), Some(&frame));
        assert_eq!(restored.in_web_document(), web);
    }
}
