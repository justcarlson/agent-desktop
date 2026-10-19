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

#[test]
fn a_stopped_target_is_named_only_after_a_sent_timeout() {
    use super::name_stopped_target;
    use agent_desktop_core::{AdapterError, DeliverySemantics};
    let unsent = AdapterError::new(ErrorCode::Timeout, "t")
        .with_disposition(DeliverySemantics::not_delivered());
    assert!(
        name_stopped_target(unsent, std::process::id())
            .details
            .is_none()
    );
}
