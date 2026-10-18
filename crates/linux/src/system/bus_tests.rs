use super::*;
use zbus::Message;

fn method_error(name: &str) -> zbus::Error {
    let call = Message::method_call("/probe", "Probe")
        .and_then(|builder| builder.build(&()))
        .expect("a method call message must build");
    let reply = Message::error(&call.header(), name)
        .and_then(|builder| builder.build(&()))
        .expect("an error reply must build");
    zbus::Error::MethodError(name.try_into().expect("a valid error name"), None, reply)
}

#[test]
fn a_reply_timeout_and_an_elapsed_deadline_are_both_timeouts() {
    let timed_out = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::TimedOut,
    )));
    assert_eq!(classify(&timed_out).code, ErrorCode::Timeout);

    let expired = Deadline::after(0).expect("a zero deadline builds");
    let error = SharedBus::default()
        .get(expired)
        .expect_err("an expired deadline must not reach the bus");
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn an_unrecognised_method_error_is_internal_and_names_the_error() {
    let error = classify(&method_error("org.example.Error.Unheard"));
    assert_eq!(error.code, ErrorCode::Internal);
    assert!(
        error
            .platform_detail
            .is_some_and(|detail| detail.contains("org.example.Error.Unheard"))
    );
}
