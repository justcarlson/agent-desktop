use agent_desktop_core::{ActionResult, AdapterError, DeliverySemantics, ErrorCode};

use super::ActedOn;
use crate::tree::AtspiElement;

fn item() -> AtspiElement {
    AtspiElement::new(":1.7".to_owned(), "/item".to_owned(), 42)
}

#[test]
fn a_delivered_action_lets_its_element_be_read_without_a_screen_position() {
    let acted = ActedOn::default();
    assert!(!acted.covers(&item()));
    let delivered = Ok::<_, AdapterError>(ActionResult::delivered_unverified("toggle"));
    acted.record(Some(&item()), &delivered);
    assert!(acted.covers(&item()));
    let other = AtspiElement::new(":1.7".to_owned(), "/other".to_owned(), 42);
    assert!(!acted.covers(&other));
}

#[test]
fn a_refused_action_does_not_lift_the_position_gate() {
    let acted = ActedOn::default();
    let refused: Result<ActionResult, _> =
        Err(AdapterError::new(ErrorCode::ActionFailed, "refused")
            .with_disposition(DeliverySemantics::not_delivered()));
    acted.record(Some(&item()), &refused);
    assert!(!acted.covers(&item()));
}
