use super::*;
use zbus::Message;

struct Registry {
    attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[zbus::interface(name = "org.a11y.atspi.Registry")]
impl Registry {
    fn register_event(
        &self,
        event: &str,
        properties: Vec<String>,
        app: &str,
    ) -> zbus::fdo::Result<()> {
        assert_eq!(event, "window:activate");
        assert!(properties.is_empty());
        assert!(app.is_empty());
        if self
            .attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            == 0
        {
            Err(zbus::fdo::Error::Failed("registration refused".into()))
        } else {
            Ok(())
        }
    }
}

#[test]
fn failed_event_registration_propagates_and_a_later_attempt_succeeds() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let attempts = Arc::new(AtomicUsize::new(0));
    let mut servers = Vec::new();
    for attempt in 0..2 {
        let (client, server) = std::os::unix::net::UnixStream::pair().expect("socket pair");
        let (client, server) = zbus::block_on(async {
            futures_lite::future::zip(
                Builder::async_io_unix_stream(client).p2p().build(),
                Builder::async_io_unix_stream(server)
                    .server(zbus::Guid::generate())
                    .expect("server guid")
                    .p2p()
                    .serve_at(
                        REGISTRY_PATH,
                        Registry {
                            attempts: attempts.clone(),
                        },
                    )
                    .expect("registry interface")
                    .build(),
            )
            .await
        });
        servers.push(server.expect("server connection"));
        let ready = register_connection(
            client.expect("client connection"),
            Deadline::after(5_000).expect("registration deadline"),
        );
        if attempt == 0 {
            let error = ready.expect_err("a refused registration must reject the connection");
            assert!(
                error
                    .platform_detail
                    .unwrap()
                    .contains("registration refused")
            );
        } else {
            let _connection = ready.expect("registration retry");
        }
        assert_eq!(attempts.load(Ordering::SeqCst), attempt + 1);
    }
}

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

#[test]
fn only_a_transport_failure_drops_the_cached_connection() {
    let broken = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::BrokenPipe,
    )));
    let slow = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::TimedOut,
    )));
    assert!(is_connection_failure(&broken));
    assert!(!is_connection_failure(&slow));
    assert!(!is_connection_failure(&method_error(
        "org.example.Error.Unheard"
    )));
}
