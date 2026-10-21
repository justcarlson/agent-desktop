use agent_desktop_core::{
    Action, ActionRequest, Deadline, ErrorCode, InteractionLease, InteractionPolicy, NativeHandle,
};

use super::execute_action;
use crate::adapter::LinuxAdapter;

#[test]
fn a_foreign_handle_is_invalid_args_before_any_arm_runs() {
    let adapter = LinuxAdapter::new();
    let deadline = Deadline::standard().expect("a standard deadline builds");
    let lease = InteractionLease::guarded(deadline, ()).expect("local validation lease");
    let foreign = NativeHandle::new(5u32);
    let error = execute_action(
        adapter.bus(),
        &foreign,
        ActionRequest::headless(Action::Click),
        &lease,
    )
    .expect_err("a foreign handle is refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn only_the_no_verb_refusal_routes_to_physical_delivery() {
    use super::has_no_verb;
    use crate::actions::click::no_semantic_verb;
    assert!(has_no_verb(&no_semantic_verb(&Action::Click)));
    let leaf = agent_desktop_core::AdapterError::new(ErrorCode::ActionNotSupported, "leaf");
    assert!(!has_no_verb(&leaf));
    let other = agent_desktop_core::AdapterError::new(ErrorCode::ActionFailed, "x")
        .with_details(serde_json::json!({ "semantic_verb": "absent" }));
    assert!(!has_no_verb(&other));
}

fn timeout(disposition: agent_desktop_core::DeliverySemantics) -> agent_desktop_core::AdapterError {
    agent_desktop_core::AdapterError::new(ErrorCode::Timeout, "t").with_disposition(disposition)
}

#[test]
fn a_sent_timeout_with_a_stopped_target_names_the_stop() {
    use super::name_stopped_target;
    let named = name_stopped_target(
        timeout(agent_desktop_core::DeliverySemantics::uncertain()),
        7,
        |_| true,
    );
    assert_eq!(named.details.expect("details")["target_stopped"], true);
}

#[test]
fn an_unsent_timeout_with_a_stopped_target_carries_no_details() {
    use super::name_stopped_target;
    let unsent = timeout(agent_desktop_core::DeliverySemantics::not_delivered());
    assert!(name_stopped_target(unsent, 7, |_| true).details.is_none());
}

#[test]
fn a_sent_timeout_with_a_running_target_carries_no_details() {
    use super::name_stopped_target;
    let sent = timeout(agent_desktop_core::DeliverySemantics::uncertain());
    assert!(name_stopped_target(sent, 7, |_| false).details.is_none());
}

fn done(label: &'static str) -> agent_desktop_core::ActionResult {
    agent_desktop_core::ActionResult::from_execution(
        &Action::Click,
        vec![agent_desktop_core::ActionStep::succeeded(label)],
    )
}

fn refusal(code: ErrorCode, check: Option<&str>) -> agent_desktop_core::AdapterError {
    let error = agent_desktop_core::AdapterError::new(code, "refused")
        .with_disposition(agent_desktop_core::DeliverySemantics::not_delivered());
    match check {
        Some(check) => error.with_details(serde_json::json!({ "check": check })),
        None => error,
    }
}

#[test]
fn a_headed_click_that_the_pointer_delivers_never_runs_the_semantic_call() {
    use super::physical_first;
    let result = physical_first(
        || Ok(done("XTEST.click")),
        || panic!("the semantic call must not run"),
    )
    .expect("delivered");
    assert_eq!(result.steps[0].label(), "XTEST.click");
}

#[test]
fn a_headed_right_click_uses_physical_delivery_before_semantic_delivery() {
    use super::right_click_dispatch;

    let result = right_click_dispatch(
        InteractionPolicy::headed(),
        || Ok(done("XTEST.click")),
        || panic!("the semantic call must not run after physical delivery"),
    )
    .expect("the headed pointer delivery succeeds");

    assert_eq!(result.steps[0].label(), "XTEST.click");
}

#[test]
fn a_headless_right_click_stays_semantic() {
    use super::right_click_dispatch;

    let result = right_click_dispatch(
        InteractionPolicy::headless(),
        || panic!("headless right-click must not move the pointer"),
        || Ok(done("Action.DoAction")),
    )
    .expect("semantic delivery");

    assert_eq!(result.steps[0].label(), "Action.DoAction");
}

#[test]
fn a_headed_right_click_keeps_the_existing_not_delivered_fallback() {
    use super::right_click_dispatch;

    let result = right_click_dispatch(
        InteractionPolicy::headed(),
        || Err(refusal(ErrorCode::ActionNotSupported, None)),
        || Ok(done("Action.DoAction")),
    )
    .expect("semantic fallback after a refused pointer delivery");

    assert_eq!(result.steps[0].label(), "Action.DoAction");
}

#[test]
fn an_unaimable_or_unsupported_pointer_falls_back_to_the_semantic_call() {
    use super::physical_first;
    for error in [
        refusal(ErrorCode::ActionNotSupported, None),
        refusal(ErrorCode::ActionFailed, Some("geometry")),
    ] {
        let result =
            physical_first(|| Err(error), || Ok(done("Action.DoAction"))).expect("falls back");
        assert_eq!(result.steps[0].label(), "Action.DoAction");
    }
}

#[test]
fn any_other_pointer_refusal_is_returned_and_the_semantic_call_stays_unused() {
    use super::physical_first;
    for error in [
        refusal(ErrorCode::ActionFailed, Some("window_active")),
        refusal(ErrorCode::PolicyDenied, None),
        agent_desktop_core::AdapterError::new(ErrorCode::ActionNotSupported, "sent")
            .with_disposition(agent_desktop_core::DeliverySemantics::uncertain()),
    ] {
        let kept = error.code.clone();
        let outcome = physical_first(|| Err(error), || panic!("the semantic call must not run"));
        assert_eq!(outcome.expect_err("refused").code, kept);
    }
}

#[test]
fn typing_with_no_semantic_verb_reaches_the_physical_keys_and_a_press_is_not_in_that_set() {
    use super::{has_no_verb, reaches_physical_without_verb};
    use crate::actions::click::no_semantic_verb;
    let typing = Action::TypeText("x".to_owned());
    assert!(reaches_physical_without_verb(&typing));
    assert!(has_no_verb(&no_semantic_verb(&typing)));
    assert!(!reaches_physical_without_verb(&Action::Toggle));
    assert!(!reaches_physical_without_verb(&Action::SetValue(
        "x".to_owned()
    )));
}
