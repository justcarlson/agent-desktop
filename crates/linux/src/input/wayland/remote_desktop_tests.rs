use std::sync::Arc;

use zbus::Message;

use super::*;

fn method_error(name: &str) -> zbus::Error {
    let call = Message::method_call("/probe", "Probe")
        .and_then(|builder| builder.build(&()))
        .expect("a method call message must build");
    let reply = Message::error(&call.header(), name)
        .and_then(|builder| builder.build(&()))
        .expect("an error reply must build");
    zbus::Error::MethodError(name.try_into().expect("a valid error name"), None, reply)
}

fn assert_refusal(error: &AdapterError) {
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("semantic action"))
    );
    let details = error.details.as_ref().expect("details");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_missing_service_is_a_refusal_that_points_at_the_semantic_action() {
    assert_refusal(&classify_call(&method_error(SERVICE_UNKNOWN)));
    assert_refusal(&classify_call(&method_error(NAME_HAS_NO_OWNER)));
}

#[test]
fn access_denied_is_a_refusal_that_points_at_the_semantic_action() {
    assert_refusal(&classify_call(&method_error(ACCESS_DENIED)));
}

#[test]
fn an_unrecognised_method_error_is_a_failed_action_that_names_the_error() {
    let error = classify_call(&method_error("org.example.Error.Unheard"));
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert!(
        error
            .platform_detail
            .is_some_and(|detail| detail.contains("org.example.Error.Unheard"))
    );
}

#[test]
fn a_reply_that_never_comes_is_a_timeout() {
    let slow = zbus::Error::InputOutput(Arc::new(std::io::Error::from(ErrorKind::TimedOut)));
    assert_eq!(classify_call(&slow).code, ErrorCode::Timeout);
}

#[test]
fn an_unanswered_session_creation_is_a_timeout_that_started_nothing() {
    let slow = zbus::Error::InputOutput(Arc::new(std::io::Error::from(ErrorKind::TimedOut)));
    let error = creation_failure(classify_call(&slow));
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_ne!(error.code, ErrorCode::Internal);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.expect("details")["physical_delivery_started"],
        false
    );
}

#[test]
fn a_refused_session_creation_keeps_its_own_error() {
    let refused = creation_failure(classify_call(&method_error(ACCESS_DENIED)));
    assert_eq!(refused.code, ErrorCode::ActionNotSupported);
}
