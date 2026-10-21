use super::*;
use agent_desktop_core::DeliveryDisposition;

#[test]
fn menu_regression_pressed_opener_without_a_popup_never_verifies_delivery() {
    let popup = AtspiElement::new(":1.1", "/popup", 7);
    for (wanted, before, after, verified) in [
        (None, vec![], Some(vec![]), false),
        (Some(true), vec![], Some(vec![]), false),
        (Some(true), vec![], None, false),
        (
            Some(false),
            vec![popup.clone()],
            Some(vec![popup.clone()]),
            false,
        ),
        (None, vec![], Some(vec![popup.clone()]), true),
        (Some(true), vec![], Some(vec![popup.clone()]), true),
        (Some(false), vec![popup], Some(vec![]), true),
    ] {
        let result = ActionResult::delivered_unverified("click")
            .with_steps(vec![
                agent_desktop_core::ActionStep::succeeded("Action.DoAction").with_verified(true),
            ])
            .with_verified_delivery();
        let events = std::cell::RefCell::new(Vec::new());
        let actual = attempt(
            wanted,
            || {
                events.borrow_mut().push("before");
                Some(before)
            },
            || {
                events.borrow_mut().push("invoke");
                Ok(result)
            },
            |_| {
                events.borrow_mut().push("after");
                after
            },
        );
        assert_eq!(*events.borrow(), ["before", "invoke", "after"]);
        if verified {
            assert_eq!(
                actual.expect("popup observed").disposition().delivery(),
                DeliveryDisposition::DeliveredVerified
            );
        } else {
            let error = actual.expect_err("state alone must not verify");
            assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
            let details = error.details.expect("evidence");
            assert_eq!(details["kind"], "popup_not_observed");
            assert_eq!(details["steps"][0]["verified"], false);
        }
    }
}

#[test]
fn an_unobserved_popup_after_a_noop_preserves_no_delivery() {
    let result = ActionResult::satisfied_without_delivery("expand");
    let error = confirm(result, Some(true), &[], Some(&[])).expect_err("no popup");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn an_unread_baseline_refuses_before_invoking_the_opener() {
    let error = attempt(
        Some(true),
        || None,
        || panic!("an unread baseline must not deliver"),
        |_| panic!("there was no action to verify"),
    )
    .expect_err("refusal");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}
