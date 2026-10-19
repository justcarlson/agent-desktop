use agent_desktop_core::{Action, ActionRequest, Deadline, ErrorCode, NativeHandle, SystemOps};

use super::execute_action;
use crate::adapter::LinuxAdapter;

#[test]
fn a_foreign_handle_is_invalid_args_before_any_arm_runs() {
    let adapter = LinuxAdapter::new();
    let deadline = Deadline::standard().expect("a standard deadline builds");
    let lease = SystemOps::acquire_interaction_lease(&adapter, deadline)
        .expect("the shared lease is granted");
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
